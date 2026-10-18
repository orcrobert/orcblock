use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;

use orcblock_core::Domain;
use tracing::info;

mod doh;
mod mac;
mod pf;
mod resolver;
pub(crate) mod runner;
pub(crate) mod windows;

pub use mac::MacEnforcer;
pub use runner::{CommandRunner, Output, SystemRunner};
pub use windows::WinEnforcer;

/// Turns the set of blocked domains into system configuration.
///
/// Implementations are idempotent: calling `apply` twice with the same set
/// changes nothing the second time.
pub trait Enforcer: Send {
    /// Makes the system block exactly `domains` (never empty).
    fn apply(&mut self, domains: &BTreeSet<Domain>) -> Result<(), EnforceError>;

    /// Checks that the system still blocks `domains` and repairs any drift.
    /// Returns `true` when something had to be repaired.
    fn verify(&mut self, domains: &BTreeSet<Domain>) -> Result<bool, EnforceError>;

    /// Removes everything this enforcer installed.
    fn clear(&mut self) -> Result<(), EnforceError>;
}

/// Enforcement failed; the block stays active and is retried on the next
/// verify.
#[derive(Debug, thiserror::Error)]
pub enum EnforceError {
    /// A file could not be read, written or removed.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was being done, such as "write".
        action: &'static str,
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// A system program failed.
    #[error("{program}: {reason}")]
    Program {
        /// The program's path.
        program: String,
        /// Its exit status and output, or why it could not run.
        reason: String,
    },
}

/// An enforcer that changes nothing and logs what it would do, for dev mode
/// and tests.
#[derive(Debug, Default)]
pub struct DryRunEnforcer {
    applied: BTreeSet<Domain>,
    calls: Vec<DryRunCall>,
}

/// A call recorded by [`DryRunEnforcer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DryRunCall {
    /// `apply` with these domains.
    Apply(BTreeSet<Domain>),
    /// `verify` with these domains.
    Verify(BTreeSet<Domain>),
    /// `clear`.
    Clear,
}

impl DryRunEnforcer {
    /// Every call so far, oldest first.
    #[must_use]
    pub fn calls(&self) -> &[DryRunCall] {
        &self.calls
    }

    /// The domains a real enforcer would be blocking now.
    #[must_use]
    pub fn applied(&self) -> &BTreeSet<Domain> {
        &self.applied
    }
}

impl Enforcer for DryRunEnforcer {
    fn apply(&mut self, domains: &BTreeSet<Domain>) -> Result<(), EnforceError> {
        if &self.applied != domains {
            info!(domains = ?names(domains), "dry run: would apply");
            self.applied.clone_from(domains);
        }
        self.calls.push(DryRunCall::Apply(domains.clone()));
        Ok(())
    }

    fn verify(&mut self, domains: &BTreeSet<Domain>) -> Result<bool, EnforceError> {
        self.calls.push(DryRunCall::Verify(domains.clone()));
        let drifted = &self.applied != domains;
        if drifted {
            self.applied.clone_from(domains);
        }
        Ok(drifted)
    }

    fn clear(&mut self) -> Result<(), EnforceError> {
        if !self.applied.is_empty() {
            info!("dry run: would clear");
            self.applied.clear();
        }
        self.calls.push(DryRunCall::Clear);
        Ok(())
    }
}

fn names(domains: &BTreeSet<Domain>) -> Vec<&str> {
    domains.iter().map(Domain::as_ascii).collect()
}
