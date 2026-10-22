//! The `OrcBlock` domain model.
//!
//! Everything here is pure and synchronous: no I/O, no clocks, no async.
//! Callers pass in the current time and elapsed durations, which keeps every
//! rule deterministic and testable. The daemon is the only caller that
//! persists [`State`] and turns it into enforcement.

#![warn(missing_docs)]

mod app;
mod applist;
mod blocklist;
mod domain;
mod list;
mod potion;
mod session;
mod state;

pub use app::{AppError, AppName, BundleId, bundles, outermost_bundle};
pub use applist::{AppEntry, AppList};
pub use blocklist::{Blocklist, Entry};
pub use domain::{Domain, DomainError};
pub use list::{ListId, ListName, ListNameError, TargetList};
pub use potion::{Month, MonthError, POTIONS_PER_MONTH, PotionLedger};
pub use session::{BlockDuration, DurationError, Session};
pub use state::{CommandError, SessionEnd, State};
