use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use orcblock_ipc::Mode;
use orcblockd::{Blocked, DryRunEnforcer, Service, Store, SystemClock, server};
use tokio::signal::unix::{SignalKind, signal};
use tracing::error;
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
}

#[derive(Debug, Args)]
struct ServeArgs {
    /// Development mode: log enforcement instead of changing the system, keep
    /// state and the socket in --data-dir, run without root.
    #[arg(long)]
    dev: bool,
    /// Where the state file lives [default with --dev: ./data].
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// The socket path [default with --dev: <data-dir>/orcblockd.sock].
    #[arg(long)]
    socket: Option<PathBuf>,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    match Cli::parse().command {
        Cmd::Serve(args) => match serve(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                error!("{err:#}");
                ExitCode::FAILURE
            }
        },
    }
}

fn serve(args: ServeArgs) -> Result<()> {
    if !args.dev {
        bail!("live enforcement is not built yet; run `orcblockd serve --dev`");
    }
    let data_dir = args.data_dir.unwrap_or_else(|| PathBuf::from("data"));
    fs::create_dir_all(&data_dir).with_context(|| format!("create {}", data_dir.display()))?;
    let socket = args
        .socket
        .unwrap_or_else(|| data_dir.join("orcblockd.sock"));

    let blocked = Blocked::default();
    let service = Service::start(
        Store::new(data_dir.join("state.json")),
        SystemClock,
        DryRunEnforcer::default(),
        blocked,
        Mode::DryRun,
    )
    .context("start the service")?;

    let runtime = tokio::runtime::Runtime::new().context("start the async runtime")?;
    runtime.block_on(async {
        let listener =
            server::bind(&socket).with_context(|| format!("listen on {}", socket.display()))?;
        let mut terminate = signal(SignalKind::terminate()).context("watch for SIGTERM")?;
        let shutdown = async move {
            tokio::select! {
                _ = terminate.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        };
        server::run(service, listener, shutdown).await;
        let _ = fs::remove_file(&socket);
        Ok(())
    })
}
