use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::app::is_invisible;
use crate::{AppList, Blocklist};

const MAX_NAME_CHARS: usize = 40;

/// Identifies a list. Assigned by [`State`](crate::State) and never reused,
/// so a client holding a stale ID cannot reach a list created later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ListId(u32);

/// The name of a list, such as `Deep work`.
///
/// Invariants: trimmed, 1-40 characters, no control characters and no
/// invisible ones that reorder or hide text. Two lists may not share a name
/// ignoring case; see [`ListName::same_as`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListName(String);

/// Why an input could not be turned into a [`ListName`]. The messages are
/// shown to the user as is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListNameError {
    /// Nothing but whitespace was given.
    #[error("Give the list a name")]
    Empty,
    /// Longer than 40 characters.
    #[error("A list name has at most 40 characters")]
    TooLong,
    /// Holds control or invisible characters.
    #[error("The list name has characters the orc can't show")]
    Unreadable,
}

/// A named list of sites and apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetList {
    pub(crate) id: ListId,
    pub(crate) name: ListName,
    #[serde(default)]
    pub(crate) sites: Blocklist,
    #[serde(default)]
    pub(crate) apps: AppList,
}

impl ListId {
    /// The ID as a number, for the wire and the command line.
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// The number itself.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl ListName {
    /// Cleans a name: surrounding whitespace is dropped.
    pub fn parse(input: &str) -> Result<Self, ListNameError> {
        let name = input.trim();
        if name.is_empty() {
            return Err(ListNameError::Empty);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(ListNameError::TooLong);
        }
        if name.chars().any(|c| c.is_control() || is_invisible(c)) {
            return Err(ListNameError::Unreadable);
        }
        Ok(Self(name.to_owned()))
    }

    /// Drops what [`ListName::parse`] refuses instead of failing; `None`
    /// when nothing printable is left.
    fn cleaned(input: &str) -> Option<Self> {
        let kept: String = input
            .chars()
            .filter(|&c| !c.is_control() && !is_invisible(c))
            .collect();
        let name: String = kept.trim().chars().take(MAX_NAME_CHARS).collect();
        let name = name.trim_end();
        (!name.is_empty()).then(|| Self(name.to_owned()))
    }

    /// True when both names read the same ignoring case.
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }

    /// The name as shown.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TargetList {
    /// The list's ID.
    #[must_use]
    pub fn id(&self) -> ListId {
        self.id
    }

    /// The list's name.
    #[must_use]
    pub fn name(&self) -> &ListName {
        &self.name
    }

    /// The sites on the list.
    #[must_use]
    pub fn sites(&self) -> &Blocklist {
        &self.sites
    }

    /// The apps on the list.
    #[must_use]
    pub fn apps(&self) -> &AppList {
        &self.apps
    }

    /// True when the list has no sites and no apps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty() && self.apps.is_empty()
    }
}

impl fmt::Display for ListId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for ListName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for ListName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ListName {
    /// Cleans rather than refuses, like [`AppName`](crate::AppName): a
    /// state file must never be set aside over a display name.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::cleaned(&raw).ok_or_else(|| serde::de::Error::custom(ListNameError::Empty))
    }
}

#[cfg(test)]
mod tests;
