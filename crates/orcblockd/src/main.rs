use std::fs::{self, OpenOptions};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use orcblock_ipc::{Endpoint, Mode};
use orcblockd::dns::{self, Sinkhole};
use orcblockd::{
    Blocked, DryRunEnforcer, Enforcer, Layout, Service, Store, SystemClock, SystemRunner, install,
    server,
};
use tokio::sync::watch;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "OrcBlock's root daemon: keeps blocks in place whether or not the app runs"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run the daemon in the foreground (launchd or the Windows task runs this).
    Serve(ServeArgs),
    /// Install this binary as a launchd daemon or Windows task and start it
    /// (needs root or an elevated prompt).
    Install,
    /// Stop and remove the daemon (needs root or an elevated prompt; refused
    /// during a block).
    Uninstall,
}

#[derive(Debug, Args)]
struct ServeArgs {
    /// Development mode: log enforcement instead of changing the system, keep
    /// state in --data-dir, run without root.
    #[arg(long)]
    dev: bool,
    /// Where the state file lives with --dev.
    #[arg(long, default_value = "data", requires = "dev")]
    data_dir: PathBuf,
    /// Where to listen with --dev: a socket path, or an address on Windows
    /// [default: <data-dir>/orcblockd.sock, or 127.0.0.1:47812 on Windows].
    #[arg(long, requires = "dev")]
    socket: Option<Endpoint>,
    /// Where the DNS sinkhole listens; resolver files point here
    /// [default: 127.0.0.1:53535 (127.0.0.1:53 on Windows), or 127.0.0.1:53536 with --dev].
    #[arg(long)]
    dns_addr: Option<SocketAddr>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Only the installed daemon writes the log: install runs before the data
    // directory is known to be locked, and must not write into one a user
    // made.
    let installed = matches!(&cli.command, Cmd::Serve(args) if !args.dev);
    init_logging(installed.then(|| Layout::system().log_file()).as_deref());
    let result = match cli.command {
        Cmd::Serve(args) => serve(args),
        Cmd::Install => run_install(),
        Cmd::Uninstall => run_uninstall(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// Logs to stderr, and on Windows also appends to `log_file`: nothing
/// collects a scheduled task's stderr the way launchd does.
fn init_logging(log_file: Option<&Path>) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file = log_file.filter(|_| cfg!(windows)).and_then(|path| {
        fs::create_dir_all(path.parent()?).ok()?;
        OpenOptions::new().create(true).append(true).open(path).ok()
    });
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match file {
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::io::stderr.and(Mutex::new(file)))
            .init(),
        None => builder.with_writer(std::io::stderr).init(),
    }
}

#[cfg(unix)]
fn require_root() -> Result<()> {
    if !nix::unistd::geteuid().is_root() {
        anyhow::bail!("this needs root; run it with sudo");
    }
    Ok(())
}

#[cfg(windows)]
fn require_root() -> Result<()> {
    Ok(install::windows::require_elevation(&SystemRunner)?)
}

#[cfg(unix)]
fn dev_socket(data_dir: &Path) -> Endpoint {
    data_dir.join("orcblockd.sock")
}

#[cfg(windows)]
fn dev_socket(_data_dir: &Path) -> Endpoint {
    SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 47812))
}

#[cfg(unix)]
fn live_enforcer(layout: &Layout, dns_addr: SocketAddr) -> impl Enforcer + 'static {
    orcblockd::MacEnforcer::new(layout.clone(), SystemRunner, dns_addr.port())
}

#[cfg(windows)]
fn live_enforcer(_layout: &Layout, _dns_addr: SocketAddr) -> impl Enforcer + 'static {
    orcblockd::WinEnforcer::new(SystemRunner)
}

#[cfg(unix)]
fn live_socket(layout: &Layout) -> Endpoint {
    layout.socket()
}

#[cfg(windows)]
fn live_socket(_layout: &Layout) -> Endpoint {
    orcblock_ipc::default_endpoint()
}

#[cfg_attr(
    windows,
    expect(
        clippy::needless_pass_by_value,
        reason = "only Copy fields are taken on Windows"
    )
)]
fn serve(args: ServeArgs) -> Result<()> {
    if args.dev {
        fs::create_dir_all(&args.data_dir)
            .with_context(|| format!("create {}", args.data_dir.display()))?;
        let socket = args.socket.unwrap_or_else(|| dev_socket(&args.data_dir));
        let store = Store::new(args.data_dir.join("state.json"));
        run_daemon(
            store,
            DryRunEnforcer::default(),
            Mode::DryRun,
            &socket,
            args.dns_addr.unwrap_or(dns::DEV_ADDR),
        )
    } else {
        require_root()?;
        let layout = Layout::system();
        fs::create_dir_all(layout.data_dir()).context("create the data directory")?;
        let dns_addr = args.dns_addr.unwrap_or(dns::DEFAULT_ADDR);
        run_daemon(
            Store::new(layout.state_file()),
            live_enforcer(&layout, dns_addr),
            Mode::Live,
            &live_socket(&layout),
            dns_addr,
        )
    }
}

/// How often to retry binding the sinkhole while its port is taken.
const DNS_RETRY: Duration = Duration::from_secs(5);

fn run_daemon<E: Enforcer + 'static>(
    store: Store,
    enforcer: E,
    mode: Mode,
    socket: &Endpoint,
    dns_addr: SocketAddr,
) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("start the async runtime")?;

    // Enforce any running block before accepting requests.
    let blocked = Blocked::default();
    let service = Service::start(store, SystemClock::new(), enforcer, blocked.clone(), mode)
        .context("start the service")?;

    runtime.block_on(async {
        let (stop_dns, dns_stopped) = watch::channel(false);
        let dns = tokio::spawn(Sinkhole::serve_on(
            dns_addr,
            blocked,
            dns_stopped,
            DNS_RETRY,
        ));
        let listener =
            server::bind(socket).with_context(|| format!("listen on {}", display(socket)))?;
        server::run(service, listener, shutdown_signal()?).await;
        let _ = stop_dns.send(true);
        let _ = dns.await;
        #[cfg(unix)]
        let _ = fs::remove_file(socket);
        Ok(())
    })
}

#[cfg(unix)]
fn shutdown_signal() -> Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).context("watch for SIGTERM")?;
    Ok(async move {
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    })
}

/// Ctrl+C only: the task scheduler ends the daemon by terminating it, and
/// the state is saved on every change and every few seconds anyway. A task
/// has no console, so when Ctrl+C cannot be watched the daemon runs on.
#[cfg(windows)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the Unix version"
)]
fn shutdown_signal() -> Result<impl Future<Output = ()>> {
    Ok(async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::warn!(%err, "cannot watch for Ctrl+C");
            std::future::pending::<()>().await;
        }
    })
}

#[cfg(unix)]
fn display(socket: &Endpoint) -> String {
    socket.display().to_string()
}

#[cfg(windows)]
fn display(socket: &Endpoint) -> String {
    socket.to_string()
}

fn run_install() -> Result<()> {
    require_root()?;
    let exe = std::env::current_exe().context("find this binary")?;
    let layout = Layout::system();
    #[cfg(unix)]
    install::install(&layout, &SystemRunner, &exe)?;
    #[cfg(windows)]
    install::windows::install(&layout, &SystemRunner, &exe)
        .inspect_err(|err| note_in_log(&layout, err))?;
    info!("installed and started {}", layout.helper().display());
    Ok(())
}

/// Appends `err` to the daemon's log, where the app points people when an
/// install it started out of sight fails, if the data directory is locked.
#[cfg(windows)]
fn note_in_log(layout: &Layout, err: &install::InstallError) {
    use std::io::Write as _;

    if install::windows::is_locked_dir(&SystemRunner, &layout.data_dir())
        && let Ok(mut log) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(layout.log_file())
    {
        let _ = writeln!(log, "install failed: {err}");
    }
}

fn run_uninstall() -> Result<()> {
    require_root()?;
    let layout = Layout::system();
    let mut enforcer = live_enforcer(&layout, dns::DEFAULT_ADDR);
    #[cfg(unix)]
    install::uninstall(&layout, &SystemRunner, &mut enforcer)?;
    #[cfg(windows)]
    install::windows::uninstall(&layout, &SystemRunner, &mut enforcer)?;
    info!(
        "uninstalled; the state file stays in {}",
        layout.data_dir().display()
    );
    Ok(())
}
