//! The wire protocol between `orcblockd` and its clients.
//!
//! One JSON object per line in each direction. Every [`Request`] carries
//! [`PROTOCOL_VERSION`]; every successful [`Response`] carries a full
//! [`Status`], so a client never needs a second round trip. The [`codec`]
//! functions are transport-agnostic; [`Client`] is a blocking client over the
//! daemon's Unix socket for the CLI and the app.

#![warn(missing_docs)]

mod client;
pub mod codec;
mod protocol;

pub use client::{Client, ClientError, DEFAULT_SOCKET};
pub use protocol::{
    App, Block, BlockedLaunch, Command, ErrorKind, List, MAX_MESSAGE_LEN, Mode, PROTOCOL_VERSION,
    Potions, Request, Response, Site, Status,
};
