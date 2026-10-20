use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{AppName, BundleId};

/// One app on the list and when it was added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
    /// What blocking matches on.
    pub bundle_id: BundleId,
    /// The name shown to people.
    pub name: AppName,
    /// When the user added it.
    pub added_at: Timestamp,
}

/// The apps the user wants blocked, in the order they were added.
///
/// Invariant: each bundle ID appears at most once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<AppEntry>", into = "Vec<AppEntry>")]
pub struct AppList {
    entries: Vec<AppEntry>,
}

impl AppList {
    /// The entries in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[AppEntry] {
        &self.entries
    }

    /// The listed bundle IDs in the order they were added.
    pub fn bundle_ids(&self) -> impl Iterator<Item = &BundleId> {
        self.entries.iter().map(|entry| &entry.bundle_id)
    }

    /// True when the list has no apps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry for `bundle_id`, if listed.
    #[must_use]
    pub fn get(&self, bundle_id: &BundleId) -> Option<&AppEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.bundle_id == bundle_id)
    }

    /// Adds an app; returns false and changes nothing when it is already
    /// listed.
    pub(crate) fn insert(&mut self, entry: AppEntry) -> bool {
        if self.get(&entry.bundle_id).is_some() {
            return false;
        }
        self.entries.push(entry);
        true
    }

    /// Removes `bundle_id` if listed; returns whether it was.
    pub(crate) fn remove(&mut self, bundle_id: &BundleId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| &entry.bundle_id != bundle_id);
        self.entries.len() != before
    }
}

impl TryFrom<Vec<AppEntry>> for AppList {
    type Error = String;

    fn try_from(entries: Vec<AppEntry>) -> Result<Self, Self::Error> {
        for (i, a) in entries.iter().enumerate() {
            if entries[i + 1..].iter().any(|b| b.bundle_id == a.bundle_id) {
                return Err(format!("app list lists {} twice", a.bundle_id));
            }
        }
        Ok(Self { entries })
    }
}

impl From<AppList> for Vec<AppEntry> {
    fn from(list: AppList) -> Self {
        list.entries
    }
}

#[cfg(test)]
mod tests;
