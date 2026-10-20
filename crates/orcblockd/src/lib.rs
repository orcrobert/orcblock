//! The `OrcBlock` daemon.
//!
//! [`Service`] owns the [`State`](orcblock_core::State): it answers requests,
//! counts blocks down with a monotonic [`Clock`], persists through [`Store`]
//! and reconciles an [`Enforcer`] with whatever core says should be blocked.
//! [`server`] puts it on a Unix socket and drives the tick loop.

#![warn(missing_docs)]

mod apps;
mod blocked;
mod clock;
pub mod dns;
mod enforce;
mod fsutil;
pub mod install;
mod layout;
pub mod server;
mod service;
mod store;
mod wake;

pub use apps::{
    AppGuard, DryRunTerminator, KILL_AFTER, LAUNCHES_KEPT, OwnUidTerminator, Stop,
    SystemTerminator, Terminator,
};
pub use blocked::Blocked;
pub use clock::{Clock, SystemClock};
pub use enforce::{
    CommandRunner, DryRunCall, DryRunEnforcer, EnforceError, Enforcer, MacEnforcer, Output,
    SystemRunner,
};
pub use layout::{LABEL, Layout};
pub use service::{MAX_RESTART_CREDIT, SWEEP_EVERY, Service, StartError, VERIFY_EVERY};
pub use store::{Saved, Store, StoreError};
pub use wake::{APP_BUNDLE_ID, Console, SystemConsole, WAKE_EVERY, Waker};
