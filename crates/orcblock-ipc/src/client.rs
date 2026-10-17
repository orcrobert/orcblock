use std::io::{self, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use orcblock_core::BlockDuration;

use crate::codec::{self, CodecError};
use crate::{Command, ErrorKind, Request, Response, Status};

/// Where the installed daemon listens.
pub const DEFAULT_SOCKET: &str = "/var/run/orcblockd.sock";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// A blocking client for the daemon's socket. Each call opens a fresh
/// connection, so a `Client` is cheap to clone and share across threads.
#[derive(Debug, Clone)]
pub struct Client {
    socket: PathBuf,
    timeout: Duration,
}

/// A failed call.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The daemon is not installed or not running.
    #[error("the OrcBlock helper is not running ({0})")]
    Unavailable(#[source] io::Error),
    /// The connection failed mid-call.
    #[error("lost the connection to the OrcBlock helper: {0}")]
    Io(#[from] io::Error),
    /// The daemon's answer could not be read.
    #[error("unreadable answer from the OrcBlock helper: {0}")]
    Codec(#[from] CodecError),
    /// The daemon closed the connection without answering.
    #[error("the OrcBlock helper hung up without answering")]
    Closed,
    /// The daemon refused the command.
    #[error("{message}")]
    Refused {
        /// What kind of refusal.
        kind: ErrorKind,
        /// A message ready to show the user.
        message: String,
    },
}

impl Default for Client {
    fn default() -> Self {
        Self::new(DEFAULT_SOCKET)
    }
}

impl Client {
    /// A client for the daemon listening at `socket`.
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets the read and write timeout for each call.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The socket this client connects to.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// The current state.
    pub fn status(&self) -> Result<Status, ClientError> {
        self.send(Command::Status)
    }

    /// Lists `site` (a domain or URL as typed).
    pub fn add_site(&self, site: &str) -> Result<Status, ClientError> {
        self.send(Command::AddSite {
            site: site.to_owned(),
        })
    }

    /// Unlists `site`.
    pub fn remove_site(&self, site: &str) -> Result<Status, ClientError> {
        self.send(Command::RemoveSite {
            site: site.to_owned(),
        })
    }

    /// Starts a block.
    pub fn start_block(&self, duration: BlockDuration) -> Result<Status, ClientError> {
        self.send(Command::StartBlock {
            secs: duration.into(),
        })
    }

    /// Ends the running block with a sleep potion.
    pub fn drink_potion(&self) -> Result<Status, ClientError> {
        self.send(Command::DrinkPotion)
    }

    /// Sends one command and waits for the answer.
    pub fn send(&self, command: Command) -> Result<Status, ClientError> {
        let mut stream = UnixStream::connect(&self.socket).map_err(ClientError::Unavailable)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        stream.write_all(&codec::encode(&Request::new(command))?)?;

        let mut reader = BufReader::new(stream);
        let mut line = Vec::new();
        if !codec::read_line(&mut reader, &mut line)? {
            return Err(ClientError::Closed);
        }
        match codec::decode(&line)? {
            Response::Ok { status } => Ok(status),
            Response::Error { kind, message } => Err(ClientError::Refused { kind, message }),
        }
    }
}

#[cfg(test)]
mod tests;
