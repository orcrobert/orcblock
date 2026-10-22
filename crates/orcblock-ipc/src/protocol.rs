use jiff::Timestamp;
use jiff::civil::Date;
use orcblock_core::{
    AppName, BundleId, CommandError, Domain, ListId, ListName, Month, POTIONS_PER_MONTH, State,
    TargetList,
};
use serde::{Deserialize, Serialize};

/// The protocol version both sides must agree on.
pub const PROTOCOL_VERSION: u32 = 3;

/// The longest line either side accepts, newline included.
pub const MAX_MESSAGE_LEN: usize = 1024 * 1024;

/// A client request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// The client's [`PROTOCOL_VERSION`].
    pub v: u32,
    /// What to do.
    #[serde(flatten)]
    pub command: Command,
}

impl Request {
    /// A request at the current protocol version.
    #[must_use]
    pub fn new(command: Command) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            command,
        }
    }
}

/// The commands the daemon understands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// Report the current state.
    Status,
    /// Create a list, empty or holding the given sites and apps. Nothing is
    /// created if any of them is refused.
    CreateList {
        /// The list's name, validated by the daemon.
        name: String,
        /// Sites to start with, validated like `add_site`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sites: Vec<String>,
        /// Apps to start with, validated like `add_app`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        apps: Vec<NewApp>,
    },
    /// Rename a list.
    RenameList {
        /// The list to rename.
        list: ListId,
        /// The new name, validated by the daemon.
        name: String,
    },
    /// Delete a list and everything on it.
    DeleteList {
        /// The list to delete.
        list: ListId,
    },
    /// Add a site to a list; `site` is what the user typed (a domain or URL).
    AddSite {
        /// The list to add to.
        list: ListId,
        /// Raw user input, cleaned and validated by the daemon.
        site: String,
    },
    /// Remove a site from a list.
    RemoveSite {
        /// The list to remove from.
        list: ListId,
        /// The domain as listed, or any input that cleans up to it.
        site: String,
    },
    /// Add an app to a list.
    AddApp {
        /// The list to add to.
        list: ListId,
        /// The app's bundle ID, validated by the daemon.
        bundle_id: String,
        /// The app's display name, validated by the daemon.
        name: String,
    },
    /// Remove an app from a list.
    RemoveApp {
        /// The list to remove from.
        list: ListId,
        /// The app's bundle ID, in any case.
        bundle_id: String,
    },
    /// Start a block over every site and app on the given lists.
    StartBlock {
        /// The block's length in seconds.
        secs: u64,
        /// The lists to block.
        lists: Vec<ListId>,
    },
    /// End the running block early with a sleep potion.
    DrinkPotion,
}

/// The daemon's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Response {
    /// The command succeeded; this is the state after it.
    Ok {
        /// The state after the command.
        status: Status,
    },
    /// The command was refused or failed.
    Error {
        /// What kind of failure, for clients that branch on it.
        kind: ErrorKind,
        /// A message ready to show the user.
        message: String,
    },
}

impl Response {
    /// An error response.
    #[must_use]
    pub fn error(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::Error {
            kind,
            message: message.into(),
        }
    }
}

impl From<&CommandError> for Response {
    fn from(err: &CommandError) -> Self {
        let kind = match err {
            CommandError::Locked(_)
            | CommandError::AppLocked(_)
            | CommandError::ListLocked(_)
            | CommandError::LastList => ErrorKind::Locked,
            CommandError::AlreadyListed { .. }
            | CommandError::AppAlreadyListed(_)
            | CommandError::ListNameTaken(_) => ErrorKind::AlreadyListed,
            CommandError::ListNotFound(_) => ErrorKind::ListNotFound,
            CommandError::TooManyLists => ErrorKind::TooManyLists,
            CommandError::TooManyTargets => ErrorKind::InvalidList,
            CommandError::NotListed(_) | CommandError::AppNotListed(_) => ErrorKind::NotListed,
            CommandError::Unblockable(_) => ErrorKind::Unblockable,
            CommandError::EmptyBlocklist => ErrorKind::EmptyBlocklist,
            CommandError::AlreadyBlocking => ErrorKind::AlreadyBlocking,
            CommandError::NotBlocking => ErrorKind::NotBlocking,
            CommandError::NoPotions { .. } => ErrorKind::NoPotions,
        };
        Self::error(kind, err.to_string())
    }
}

/// Why a request failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Sites and apps cannot be removed during a block.
    Locked,
    /// The site (or a parent of it) or the app is already listed.
    AlreadyListed,
    /// The site or app is not listed.
    NotListed,
    /// No list has the given ID.
    ListNotFound,
    /// The most lists the daemon keeps exist already.
    TooManyLists,
    /// The app is part of macOS or `OrcBlock` and is never blocked.
    Unblockable,
    /// A block needs at least one site or app.
    EmptyBlocklist,
    /// A block is already running.
    AlreadyBlocking,
    /// No block is running.
    NotBlocking,
    /// No sleep potions are left this month.
    NoPotions,
    /// The input is not a blockable domain.
    InvalidSite,
    /// The input is not a valid bundle ID or app name.
    InvalidApp,
    /// The input is not a valid list name, or a new list was to hold too
    /// many sites and apps.
    InvalidList,
    /// The duration is out of range.
    InvalidDuration,
    /// Client and daemon speak different protocol versions.
    Version,
    /// The request could not be parsed.
    BadRequest,
    /// The daemon failed to save or enforce; the state is unchanged.
    Internal,
}

/// Whether the daemon really enforces blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The installed helper, enforcing for real.
    Live,
    /// A development daemon that only logs what it would do.
    DryRun,
}

/// A snapshot of everything a client shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The daemon's crate version.
    pub daemon_version: String,
    /// Live or dry run.
    pub mode: Mode,
    /// The lists, in the order they were created.
    pub lists: Vec<List>,
    /// The running block, if any.
    pub block: Option<Block>,
    /// The sleep potions.
    pub potions: Potions,
    /// The most recent launches of blocked apps that the daemon closed,
    /// oldest first. Kept in memory only, so it is empty after a restart.
    pub blocked_launches: Vec<BlockedLaunch>,
}

/// An app to put on a new list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewApp {
    /// The app's bundle ID, validated by the daemon.
    pub bundle_id: String,
    /// The name to show, validated by the daemon.
    pub name: String,
}

/// A named list of sites and apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// What commands name the list by.
    pub id: ListId,
    /// The name people read.
    pub name: ListName,
    /// The sites, in the order they were added.
    pub sites: Vec<Site>,
    /// The apps, in the order they were added.
    pub apps: Vec<App>,
}

/// A listed app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    /// What blocking matches on; used for commands.
    pub bundle_id: BundleId,
    /// The name people read.
    pub name: AppName,
    /// When it was listed.
    pub added_at: Timestamp,
}

/// A blocked app that was opened during a block and closed by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedLaunch {
    /// Unique for the daemon's lifetime and increasing, so a client can tell
    /// which launches it has already shown.
    pub id: u64,
    /// The app that was closed.
    pub bundle_id: BundleId,
    /// Its listed name.
    pub name: AppName,
    /// When it was closed.
    pub at: Timestamp,
}

/// A listed site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    /// The cleaned ASCII domain, used for commands and favicons.
    pub domain: Domain,
    /// The domain as people read it (Unicode).
    pub display: String,
    /// When it was listed.
    pub added_at: Timestamp,
}

/// A running block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// The lists the block was started over; additions to them join it.
    pub lists: Vec<ListId>,
    /// The domains enforced.
    pub domains: Vec<Domain>,
    /// The apps closed while it runs.
    pub apps: Vec<BundleId>,
    /// When the block started.
    pub started_at: Timestamp,
    /// The chosen length in seconds.
    pub duration_secs: u64,
    /// Seconds left.
    pub remaining_secs: u64,
    /// The expected end by the wall clock, for display only.
    pub ends_at: Timestamp,
}

/// The sleep potion supply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Potions {
    /// Potions left this month.
    pub available: u8,
    /// Potions granted each month.
    pub per_month: u8,
    /// The first day of the next refill.
    pub refill_on: Date,
}

impl Status {
    /// Builds the snapshot of `state` as of `now`, in calendar `month`, with
    /// no blocked launches; the daemon fills those in from memory.
    #[must_use]
    pub fn new(
        state: &State,
        now: Timestamp,
        month: Month,
        mode: Mode,
        daemon_version: &str,
    ) -> Self {
        let lists = state.lists().iter().map(List::new).collect();
        let block = state.session().map(|session| Block {
            lists: session.lists().iter().copied().collect(),
            domains: session.domains().iter().cloned().collect(),
            apps: session.apps().iter().cloned().collect(),
            started_at: session.started_at(),
            duration_secs: u64::from(session.duration()),
            remaining_secs: session.remaining().as_secs(),
            ends_at: now
                .checked_add(session.remaining())
                .unwrap_or(Timestamp::MAX),
        });
        let potions = Potions {
            available: state.potions().available(month),
            per_month: POTIONS_PER_MONTH,
            refill_on: state.potions().refill(month).first_day(),
        };
        Self {
            daemon_version: daemon_version.to_owned(),
            mode,
            lists,
            block,
            potions,
            blocked_launches: Vec::new(),
        }
    }
}

impl List {
    fn new(list: &TargetList) -> Self {
        Self {
            id: list.id(),
            name: list.name().clone(),
            sites: list
                .sites()
                .entries()
                .iter()
                .map(|entry| Site {
                    display: entry.domain.to_unicode(),
                    domain: entry.domain.clone(),
                    added_at: entry.added_at,
                })
                .collect(),
            apps: list
                .apps()
                .entries()
                .iter()
                .map(|entry| App {
                    bundle_id: entry.bundle_id.clone(),
                    name: entry.name.clone(),
                    added_at: entry.added_at,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;
