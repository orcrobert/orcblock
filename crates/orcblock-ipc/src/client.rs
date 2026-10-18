use std::io::{self, BufReader, Write};
#[cfg(windows)]
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::time::Duration;

use orcblock_core::BlockDuration;

use crate::codec::{self, CodecError};
use crate::{Command, ErrorKind, Request, Response, Status};

/// Where a daemon listens: a Unix socket path.
#[cfg(unix)]
pub type Endpoint = std::path::PathBuf;

/// Where a daemon listens: a loopback TCP address, because Windows has no
/// Unix sockets in the standard library (ADR 0005).
#[cfg(windows)]
pub type Endpoint = std::net::SocketAddr;

/// Where the installed daemon listens.
#[cfg(unix)]
pub const DEFAULT_SOCKET: &str = "/var/run/orcblockd.sock";

/// Where the installed daemon listens; [`PORT`] on loopback.
#[cfg(windows)]
pub const DEFAULT_SOCKET: &str = "127.0.0.1:47811";

/// The loopback port of the installed daemon on Windows.
pub const PORT: u16 = 47811;

/// The loopback port of a development daemon on Windows.
pub const DEV_PORT: u16 = 47812;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// A blocking client for the daemon's socket. Each call opens a fresh
/// connection, so a `Client` is cheap to clone and share across threads.
#[derive(Debug, Clone)]
pub struct Client {
    socket: Endpoint,
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
        Self::new(default_endpoint())
    }
}

/// [`DEFAULT_SOCKET`] as an [`Endpoint`].
#[must_use]
pub fn default_endpoint() -> Endpoint {
    #[cfg(unix)]
    return Endpoint::from(DEFAULT_SOCKET);
    #[cfg(windows)]
    return std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, PORT));
}

impl Client {
    /// A client for the daemon listening at `socket`.
    pub fn new(socket: impl Into<Endpoint>) -> Self {
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
    pub fn socket(&self) -> &Endpoint {
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
        let mut stream = self.connect().map_err(ClientError::Unavailable)?;
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

    #[cfg(unix)]
    fn connect(&self) -> io::Result<UnixStream> {
        UnixStream::connect(&self.socket)
    }

    #[cfg(windows)]
    fn connect(&self) -> io::Result<TcpStream> {
        TcpStream::connect_timeout(&self.socket, self.timeout)
    }
}

#[cfg(test)]
mod tests;
