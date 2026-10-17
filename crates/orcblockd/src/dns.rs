//! The DNS sinkhole that resolver files point blocked domains at.
//!
//! It answers on 127.0.0.1 over UDP and TCP and never forwards: A and AAAA
//! for a blocked name get `0.0.0.0` and `::`, any other type an empty
//! answer (so HTTPS/SVCB records cannot hint at ECH or QUIC), the bypass
//! canaries in [`CANARIES`] get NXDOMAIN while a block runs, and names it
//! does not own are refused.

pub(crate) mod wire;

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::watch;
use tokio::time::timeout;
use tracing::{debug, warn};

use crate::Blocked;
pub use wire::{Query, Rcode, RecordType};

/// Where the sinkhole listens; resolver files name this port.
pub const DEFAULT_ADDR: SocketAddr = SocketAddr::V4(std::net::SocketAddrV4::new(
    std::net::Ipv4Addr::LOCALHOST,
    53535,
));

/// Names answered NXDOMAIN while a block runs: Firefox's DNS-over-HTTPS canary and
/// iCloud Private Relay's opt-out names. The enforcer routes them here too.
pub const CANARIES: [&str; 3] = [
    "use-application-dns.net",
    "mask.icloud.com",
    "mask-h2.icloud.com",
];

/// TTL of sinkhole answers, short so a finished block is forgotten quickly.
const TTL_SECS: u32 = 10;

const TCP_TIMEOUT: Duration = Duration::from_secs(10);

/// What the sinkhole should do with a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Answer with the unspecified address for A/AAAA, nothing for others.
    Sinkhole,
    /// The name does not exist.
    NxDomain,
    /// Not ours to answer.
    Refused,
}

/// Decides how to answer `name` given what is blocked.
#[must_use]
pub fn decide(name: &str, blocked: &Blocked) -> Answer {
    if !blocked.is_active() {
        return Answer::Refused;
    }
    let bare = name.trim_end_matches('.');
    if CANARIES
        .iter()
        .any(|canary| bare.eq_ignore_ascii_case(canary))
    {
        Answer::NxDomain
    } else if blocked.blocks_name(name) {
        Answer::Sinkhole
    } else {
        Answer::Refused
    }
}

/// Builds the reply to one raw query, or `None` when the packet is too
/// broken to answer.
#[must_use]
pub fn reply(packet: &[u8], blocked: &Blocked) -> Option<Vec<u8>> {
    let query = match Query::parse(packet) {
        Ok(query) => query,
        Err(wire::ParseError::Unanswerable) => return None,
        Err(wire::ParseError::Malformed { header }) => {
            return Some(wire::error_reply(&header, Rcode::FormErr));
        }
        Err(wire::ParseError::NotImplemented { header }) => {
            return Some(wire::error_reply(&header, Rcode::NotImp));
        }
    };
    Some(match decide(query.name(), blocked) {
        Answer::Sinkhole => query.sinkhole_reply(TTL_SECS),
        Answer::NxDomain => query.reply(Rcode::NxDomain),
        Answer::Refused => query.reply(Rcode::Refused),
    })
}

/// The sinkhole's bound UDP and TCP sockets.
#[derive(Debug)]
pub struct Sinkhole {
    udp: UdpSocket,
    tcp: TcpListener,
}

impl Sinkhole {
    /// Binds UDP and TCP on `addr`.
    pub async fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self {
            udp: UdpSocket::bind(addr).await?,
            tcp: TcpListener::bind(addr).await?,
        })
    }

    /// The UDP address actually bound (useful with port 0).
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.udp.local_addr()
    }

    /// Answers queries from `blocked` until `stopped` changes.
    pub async fn serve(self, blocked: Blocked, stopped: watch::Receiver<bool>) {
        tokio::join!(
            serve_udp(self.udp, blocked.clone(), stopped.clone()),
            serve_tcp(self.tcp, blocked, stopped)
        );
    }
}

async fn serve_udp(socket: UdpSocket, blocked: Blocked, mut stopped: watch::Receiver<bool>) {
    let mut buf = [0u8; 512];
    loop {
        tokio::select! {
            _ = stopped.changed() => return,
            received = socket.recv_from(&mut buf) => match received {
                Ok((len, peer)) => {
                    if let Some(answer) = reply(&buf[..len], &blocked)
                        && let Err(err) = socket.send_to(&answer, peer).await
                    {
                        debug!(%err, "dns reply failed");
                    }
                }
                Err(err) => warn!(%err, "dns receive failed"),
            },
        }
    }
}

async fn serve_tcp(listener: TcpListener, blocked: Blocked, mut stopped: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = stopped.changed() => return,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    let blocked = blocked.clone();
                    tokio::spawn(async move {
                        if let Err(err) = tcp_connection(stream, &blocked).await {
                            debug!(%err, "dns tcp connection closed");
                        }
                    });
                }
                Err(err) => warn!(%err, "dns accept failed"),
            },
        }
    }
}

async fn tcp_connection(mut stream: TcpStream, blocked: &Blocked) -> io::Result<()> {
    loop {
        let mut len = [0u8; 2];
        match timeout(TCP_TIMEOUT, stream.read_exact(&mut len)).await {
            Err(_) => return Ok(()),
            Ok(Err(err)) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Ok(result) => result?,
        };
        let mut packet = vec![0u8; usize::from(u16::from_be_bytes(len))];
        timeout(TCP_TIMEOUT, stream.read_exact(&mut packet))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        let Some(answer) = reply(&packet, blocked) else {
            return Ok(());
        };
        let answer_len = u16::try_from(answer.len()).map_err(io::Error::other)?;
        stream.write_all(&answer_len.to_be_bytes()).await?;
        stream.write_all(&answer).await?;
    }
}

#[cfg(test)]
mod tests;
