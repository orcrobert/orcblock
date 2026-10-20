use jiff::Timestamp;
use jiff::civil::Date;
use orcblock_core::{CommandError, Domain, Month, POTIONS_PER_MONTH, State};
use serde::{Deserialize, Serialize};

/// The protocol version both sides must agree on.
pub const PROTOCOL_VERSION: u32 = 1;

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
    /// List a site; `site` is what the user typed (a domain or URL).
    AddSite {
        /// Raw user input, cleaned and validated by the daemon.
        site: String,
    },
    /// Unlist a site.
    RemoveSite {
        /// The domain as listed, or any input that cleans up to it.
        site: String,
    },
    /// Start a block over every listed site.
    StartBlock {
        /// The block's length in seconds.
        secs: u64,
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
            CommandError::Locked(_) | CommandError::AppLocked(_) => ErrorKind::Locked,
            CommandError::AlreadyListed { .. } | CommandError::AppAlreadyListed(_) => {
                ErrorKind::AlreadyListed
            }
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
    /// Sites cannot be removed during a block.
    Locked,
    /// The site, or a parent of it, is already listed.
    AlreadyListed,
    /// The site is not listed.
    NotListed,
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
    /// The list, in the order sites were added.
    pub sites: Vec<Site>,
    /// The running block, if any.
    pub block: Option<Block>,
    /// The sleep potions.
    pub potions: Potions,
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
    /// The domains enforced.
    pub domains: Vec<Domain>,
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
    /// Builds the snapshot of `state` as of `now`, in calendar `month`.
    #[must_use]
    pub fn new(
        state: &State,
        now: Timestamp,
        month: Month,
        mode: Mode,
        daemon_version: &str,
    ) -> Self {
        let sites = state
            .blocklist()
            .entries()
            .iter()
            .map(|entry| Site {
                display: entry.domain.to_unicode(),
                domain: entry.domain.clone(),
                added_at: entry.added_at,
            })
            .collect();
        let block = state.session().map(|session| Block {
            domains: session.domains().iter().cloned().collect(),
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
            sites,
            block,
            potions,
        }
    }
}

#[cfg(test)]
mod tests;
