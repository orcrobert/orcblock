//! The `OrcBlock` daemon.
//!
//! [`Service`] owns the [`State`](orcblock_core::State): it answers requests,
//! counts blocks down with a monotonic [`Clock`], persists through [`Store`]
//! and reconciles an [`Enforcer`] with whatever core says should be blocked.
//! [`server`] puts it on a Unix socket and drives the tick loop.

#![warn(missing_docs)]

mod blocked;
mod clock;
pub mod dns;
mod enforce;
pub mod server;
mod service;
mod store;

pub use blocked::Blocked;
pub use clock::{Clock, SystemClock};
pub use enforce::{DryRunEnforcer, EnforceError, Enforcer};
pub use service::{MAX_RESTART_CREDIT, Service, StartError, VERIFY_EVERY};
pub use store::{Saved, Store, StoreError};
