//! The `orcblock` command.

mod duration;
mod render;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jiff::tz::TimeZone;
use orcblock_core::BlockDuration;
use orcblock_ipc::{Client, ClientError, DEFAULT_SOCKET, Status};

#[derive(Debug, Parser)]
#[command(version, about = "Manage OrcBlock from the terminal")]
struct Cli {
    /// The daemon's socket.
    #[arg(long, env = "ORCBLOCK_SOCKET", default_value = DEFAULT_SOCKET, global = true)]
    socket: PathBuf,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Show the sites, apps, the running block and the potions left (the default).
    Status,
    /// Add a site; a URL is cleaned up to its domain.
    Add {
        /// A domain or URL, like youtube.com or www.reddit.com/r/rust.
        site: String,
    },
    /// Remove a site (not while a block runs).
    Remove {
        /// The site as listed.
        site: String,
    },
    /// Add an app by its bundle ID, like com.tinyspeck.slackmacgap.
    AddApp {
        /// The app's bundle ID (`osascript -e 'id of app "Slack"'` prints it).
        bundle_id: String,
        /// The name to show; defaults to the bundle ID.
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove an app (not while a block runs).
    RemoveApp {
        /// The app's bundle ID as listed.
        bundle_id: String,
    },
    /// Block every listed site and app for a while: 25m, 2h, 1h30m, 1d.
    Start {
        /// How long, between 1 minute and 7 days.
        duration: String,
    },
    /// Drink a sleep potion to end the running block early.
    Potion,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = writeln!(io::stderr().lock(), "orcblock: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let client = Client::new(cli.socket);
    let status = match cli.command.unwrap_or(Cmd::Status) {
        Cmd::Status => call(client.status())?,
        Cmd::Add { site } => call(client.add_site(&site))?,
        Cmd::Remove { site } => call(client.remove_site(&site))?,
        Cmd::AddApp { bundle_id, name } => {
            call(client.add_app(&bundle_id, name.as_deref().unwrap_or(&bundle_id)))?
        }
        Cmd::RemoveApp { bundle_id } => call(client.remove_app(&bundle_id))?,
        Cmd::Start { duration } => {
            let duration = duration::parse(&duration).map_err(anyhow::Error::msg)?;
            let duration =
                BlockDuration::from_secs(duration.as_secs()).map_err(anyhow::Error::msg)?;
            call(client.start_block(duration))?
        }
        Cmd::Potion => call(client.drink_potion())?,
    };
    let text = render::status(&status, &TimeZone::system());
    io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .context("write to stdout")
}

fn call(result: Result<Status, ClientError>) -> Result<Status> {
    result.map_err(|err| match err {
        ClientError::Unavailable(_) => anyhow::anyhow!(
            "{err}; install it from the OrcBlock app, or run `make daemon-dev` and pass --socket data/orcblockd.sock"
        ),
        other => other.into(),
    })
}
