use std::collections::BTreeSet;
use std::sync::{Arc, PoisonError, RwLock};

use orcblock_core::Domain;

/// The domains being blocked right now, shared between the service (which
/// writes it) and the DNS sinkhole (which answers from it).
#[derive(Debug, Clone, Default)]
pub struct Blocked(Arc<RwLock<BTreeSet<Domain>>>);

impl Blocked {
    /// Replaces the blocked set.
    pub fn replace(&self, domains: BTreeSet<Domain>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = domains;
    }

    /// True while anything is blocked, that is while a block runs.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.read(BTreeSet::is_empty)
    }

    /// True when the DNS name `name` is a blocked domain or one of its
    /// subdomains.
    #[must_use]
    pub fn blocks_name(&self, name: &str) -> bool {
        self.read(|domains| domains.iter().any(|domain| domain.matches_name(name)))
    }

    /// A copy of the blocked set.
    #[must_use]
    pub fn snapshot(&self) -> BTreeSet<Domain> {
        self.read(Clone::clone)
    }

    fn read<T>(&self, f: impl FnOnce(&BTreeSet<Domain>) -> T) -> T {
        f(&self.0.read().unwrap_or_else(PoisonError::into_inner))
    }
}

#[cfg(test)]
mod tests;
