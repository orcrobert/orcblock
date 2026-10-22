use std::collections::BTreeSet;
use std::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{BundleId, Domain, ListId};

/// How long a block runs: between one minute and seven days, in whole
/// seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct BlockDuration(Duration);

/// A requested block duration outside the allowed range. The messages are
/// shown to the user as is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DurationError {
    /// Shorter than [`BlockDuration::MIN`].
    #[error("A block lasts at least one minute")]
    TooShort,
    /// Longer than [`BlockDuration::MAX`].
    #[error("A block lasts at most seven days")]
    TooLong,
}

impl BlockDuration {
    /// The shortest block: one minute.
    pub const MIN: Duration = Duration::from_mins(1);
    /// The longest block: seven days.
    pub const MAX: Duration = Duration::from_hours(7 * 24);

    /// Validates a duration in seconds.
    pub fn from_secs(secs: u64) -> Result<Self, DurationError> {
        let duration = Duration::from_secs(secs);
        if duration < Self::MIN {
            Err(DurationError::TooShort)
        } else if duration > Self::MAX {
            Err(DurationError::TooLong)
        } else {
            Ok(Self(duration))
        }
    }

    /// Validates a duration in minutes.
    pub fn from_minutes(minutes: u64) -> Result<Self, DurationError> {
        Self::from_secs(minutes.saturating_mul(60))
    }

    /// The duration itself.
    #[must_use]
    pub fn get(self) -> Duration {
        self.0
    }
}

impl TryFrom<u64> for BlockDuration {
    type Error = DurationError;

    fn try_from(secs: u64) -> Result<Self, Self::Error> {
        Self::from_secs(secs)
    }
}

impl From<BlockDuration> for u64 {
    fn from(duration: BlockDuration) -> Self {
        duration.0.as_secs()
    }
}

/// A running block.
///
/// Time is tracked as `remaining`, which only ever goes down through
/// [`Session::advance`]; the caller decides how much time has elapsed (the
/// daemon uses a monotonic clock), so wall-clock changes cannot shorten it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Empty only in sessions saved before lists; the state fills it in on
    /// load.
    #[serde(default)]
    pub(crate) lists: BTreeSet<ListId>,
    domains: BTreeSet<Domain>,
    #[serde(default)]
    apps: BTreeSet<BundleId>,
    started_at: Timestamp,
    duration: BlockDuration,
    #[serde(with = "secs")]
    remaining: Duration,
}

impl Session {
    pub(crate) fn new(
        lists: BTreeSet<ListId>,
        domains: BTreeSet<Domain>,
        apps: BTreeSet<BundleId>,
        duration: BlockDuration,
        now: Timestamp,
    ) -> Self {
        Self {
            lists,
            domains,
            apps,
            started_at: now,
            duration,
            remaining: duration.get(),
        }
    }

    /// The lists this block was started over. Additions to them join the
    /// block; their entries cannot be removed while it runs.
    #[must_use]
    pub fn lists(&self) -> &BTreeSet<ListId> {
        &self.lists
    }

    /// True when `list` is part of this block.
    #[must_use]
    pub fn includes(&self, list: ListId) -> bool {
        self.lists.contains(&list)
    }

    /// The domains this block enforces.
    #[must_use]
    pub fn domains(&self) -> &BTreeSet<Domain> {
        &self.domains
    }

    /// The apps this block closes.
    #[must_use]
    pub fn apps(&self) -> &BTreeSet<BundleId> {
        &self.apps
    }

    /// When the block started, by the wall clock.
    #[must_use]
    pub fn started_at(&self) -> Timestamp {
        self.started_at
    }

    /// The duration chosen at start.
    #[must_use]
    pub fn duration(&self) -> BlockDuration {
        self.duration
    }

    /// Time left until the block ends on its own.
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.remaining
    }

    /// True when `domain` is enforced by this block, directly or through a
    /// parent domain.
    #[must_use]
    pub fn blocks(&self, domain: &Domain) -> bool {
        self.domains.iter().any(|blocked| blocked.covers(domain))
    }

    /// True once no time is left.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.remaining.is_zero()
    }

    pub(crate) fn add(&mut self, domain: Domain) {
        self.domains.insert(domain);
    }

    pub(crate) fn add_app(&mut self, bundle_id: BundleId) {
        self.apps.insert(bundle_id);
    }

    pub(crate) fn advance(&mut self, elapsed: Duration) {
        self.remaining = self.remaining.saturating_sub(elapsed);
    }
}

mod secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        duration: &Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(duration.as_secs())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Duration, D::Error> {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }
}

#[cfg(test)]
mod tests;
