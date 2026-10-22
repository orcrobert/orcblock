//! The `orcblock` command.

mod duration;
mod render;
mod target;

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
    /// Create, rename or delete a list.
    #[command(subcommand)]
    List(ListCmd),
    /// Add a site; a URL is cleaned up to its domain.
    Add {
        /// A domain or URL, like youtube.com or www.reddit.com/r/rust.
        site: String,
        /// The list to add to; needed when there are several.
        #[arg(long)]
        list: Option<String>,
    },
    /// Remove a site (not from a list in the running block).
    Remove {
        /// The site as listed.
        site: String,
        /// The list to remove from; needed when there are several.
        #[arg(long)]
        list: Option<String>,
    },
    /// Add an app by its bundle ID, like com.tinyspeck.slackmacgap.
    AddApp {
        /// The app's bundle ID (`osascript -e 'id of app "Slack"'` prints it).
        bundle_id: String,
        /// The name to show; defaults to the bundle ID.
        #[arg(long)]
        name: Option<String>,
        /// The list to add to; needed when there are several.
        #[arg(long)]
        list: Option<String>,
    },
    /// Remove an app (not from a list in the running block).
    RemoveApp {
        /// The app's bundle ID as listed.
        bundle_id: String,
        /// The list to remove from; needed when there are several.
        #[arg(long)]
        list: Option<String>,
    },
    /// Block sites and apps for a while: 25m, 2h, 1h30m, 1d.
    Start {
        /// How long, between 1 minute and 7 days.
        duration: String,
        /// A list to block; repeat for several. Every list by default.
        #[arg(long = "list")]
        lists: Vec<String>,
    },
    /// Drink a sleep potion to end the running block early.
    Potion,
}

#[derive(Debug, Subcommand)]
enum ListCmd {
    /// Create an empty list.
    Create {
        /// The new list's name.
        name: String,
    },
    /// Rename a list.
    Rename {
        /// The list's current name.
        list: String,
        /// Its new name.
        name: String,
    },
    /// Delete a list and everything on it (not while it is blocked).
    Delete {
        /// The list's name.
        list: String,
    },
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
    let current = call(client.status())?;
    let list = |name: Option<&str>| target::list(&current, name).map_err(anyhow::Error::msg);
    let status = match cli.command.unwrap_or(Cmd::Status) {
        Cmd::Status => return print(&current),
        Cmd::List(ListCmd::Create { name }) => call(client.create_list(&name))?,
        Cmd::List(ListCmd::Rename { list: from, name }) => {
            call(client.rename_list(list(Some(&from))?, &name))?
        }
        Cmd::List(ListCmd::Delete { list: name }) => call(client.delete_list(list(Some(&name))?))?,
        Cmd::Add { site, list: name } => call(client.add_site(list(name.as_deref())?, &site))?,
        Cmd::Remove { site, list: name } => {
            call(client.remove_site(list(name.as_deref())?, &site))?
        }
        Cmd::AddApp {
            bundle_id,
            name,
            list: list_name,
        } => call(client.add_app(
            list(list_name.as_deref())?,
            &bundle_id,
            name.as_deref().unwrap_or(&bundle_id),
        ))?,
        Cmd::RemoveApp {
            bundle_id,
            list: name,
        } => call(client.remove_app(list(name.as_deref())?, &bundle_id))?,
        Cmd::Start { duration, lists } => {
            let duration = duration::parse(&duration).map_err(anyhow::Error::msg)?;
            let duration =
                BlockDuration::from_secs(duration.as_secs()).map_err(anyhow::Error::msg)?;
            let lists = target::lists(&current, &lists).map_err(anyhow::Error::msg)?;
            call(client.start_block(&lists, duration))?
        }
        Cmd::Potion => call(client.drink_potion())?,
    };
    print(&status)
}

fn print(status: &Status) -> Result<()> {
    let text = render::status(status, &TimeZone::system());
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
