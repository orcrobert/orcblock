use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use orcblock_ipc::Mode;
use orcblockd::dns::{self, Sinkhole};
use orcblockd::{
    AppGuard, Blocked, DryRunEnforcer, DryRunTerminator, Enforcer, Layout, MacEnforcer,
    OwnUidTerminator, Service, Store, SystemClock, SystemConsole, SystemRunner, SystemTerminator,
    Waker, install, server,
};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::watch;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

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
    /// Run the daemon in the foreground (launchd runs this).
    Serve(ServeArgs),
    /// Install this binary as a launchd daemon and start it (needs root).
    Install,
    /// Stop and remove the launchd daemon (needs root; refused during a block).
    Uninstall,
}

#[derive(Debug, Args)]
struct ServeArgs {
    /// Development mode: log enforcement instead of changing the system, keep
    /// state and the socket in --data-dir, run without root.
    #[arg(long)]
    dev: bool,
    /// Where the state file lives with --dev.
    #[arg(long, default_value = "data", requires = "dev")]
    data_dir: PathBuf,
    /// The socket path with --dev [default: <data-dir>/orcblockd.sock].
    #[arg(long, requires = "dev")]
    socket: Option<PathBuf>,
    /// Where the DNS sinkhole listens; resolver files point here
    /// [default: 127.0.0.1:53535, or 127.0.0.1:53536 with --dev].
    #[arg(long)]
    dns_addr: Option<SocketAddr>,
    /// With --dev, really close blocked apps, but only your own; refused as
    /// root. Without it the dev daemon only logs what it would close.
    #[arg(long, requires = "dev")]
    dev_kill_own_apps: bool,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    let result = match Cli::parse().command {
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

fn require_root() -> Result<()> {
    if !nix::unistd::geteuid().is_root() {
        bail!("this needs root; run it with sudo");
    }
    Ok(())
}

fn serve(args: ServeArgs) -> Result<()> {
    if args.dev {
        fs::create_dir_all(&args.data_dir)
            .with_context(|| format!("create {}", args.data_dir.display()))?;
        let socket = args
            .socket
            .unwrap_or_else(|| args.data_dir.join("orcblockd.sock"));
        let store = Store::new(args.data_dir.join("state.json"));
        let apps = if args.dev_kill_own_apps {
            if nix::unistd::geteuid().is_root() {
                bail!(
                    "--dev-kill-own-apps would close every user's apps as root; run it unprivileged"
                );
            }
            AppGuard::new(
                SystemRunner,
                OwnUidTerminator::new(nix::unistd::getuid().as_raw()),
            )
        } else {
            AppGuard::new(SystemRunner, DryRunTerminator)
        };
        run_daemon(
            store,
            DryRunEnforcer::default(),
            apps,
            Mode::DryRun,
            &socket,
            args.dns_addr.unwrap_or(dns::DEV_ADDR),
        )
    } else {
        require_root()?;
        let layout = Layout::system();
        fs::create_dir_all(layout.data_dir()).context("create the data directory")?;
        let dns_addr = args.dns_addr.unwrap_or(dns::DEFAULT_ADDR);
        let enforcer = MacEnforcer::new(layout.clone(), SystemRunner, dns_addr.port());
        run_daemon(
            Store::new(layout.state_file()),
            enforcer,
            AppGuard::new(SystemRunner, SystemTerminator)
                .waking(Waker::new(SystemRunner, SystemConsole)),
            Mode::Live,
            &layout.socket(),
            dns_addr,
        )
    }
}

fn run_daemon<E: Enforcer + 'static>(
    store: Store,
    enforcer: E,
    apps: AppGuard,
    mode: Mode,
    socket: &Path,
    dns_addr: SocketAddr,
) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("start the async runtime")?;
    let sinkhole = runtime
        .block_on(Sinkhole::bind(dns_addr))
        .with_context(|| format!("bind the DNS sinkhole on {dns_addr}"))?;

    // Enforce any running block before accepting requests.
    let blocked = Blocked::default();
    let service = Service::start(store, SystemClock, enforcer, blocked.clone(), apps, mode)
        .context("start the service")?;

    runtime.block_on(async {
        let (stop_dns, dns_stopped) = watch::channel(false);
        let dns = tokio::spawn(sinkhole.serve(blocked, dns_stopped));
        let listener =
            server::bind(socket).with_context(|| format!("listen on {}", socket.display()))?;
        let mut terminate = signal(SignalKind::terminate()).context("watch for SIGTERM")?;
        let shutdown = async move {
            tokio::select! {
                _ = terminate.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        };
        server::run(service, listener, shutdown).await;
        let _ = stop_dns.send(true);
        let _ = dns.await;
        let _ = fs::remove_file(socket);
        Ok(())
    })
}

fn run_install() -> Result<()> {
    require_root()?;
    let exe = std::env::current_exe().context("find this binary")?;
    install::install(&Layout::system(), &SystemRunner, &exe)?;
    info!("installed and started {}", orcblockd::LABEL);
    Ok(())
}

fn run_uninstall() -> Result<()> {
    require_root()?;
    let layout = Layout::system();
    let mut enforcer = MacEnforcer::new(layout.clone(), SystemRunner, dns::DEFAULT_ADDR.port());
    install::uninstall(&layout, &SystemRunner, &mut enforcer)?;
    info!(
        "uninstalled; the state file stays in {}",
        layout.data_dir().display()
    );
    Ok(())
}
