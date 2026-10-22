use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MAX_ID_LEN: usize = 255;
const MAX_NAME_CHARS: usize = 100;

/// Bundle IDs that can never be blocked: closing them would leave the Mac
/// unusable or switch `OrcBlock` itself off.
const NEVER_BLOCK: &[&str] = &[
    "com.apple.finder",
    "com.apple.dock",
    "com.apple.loginwindow",
    "com.apple.systemuiserver",
    "com.apple.controlcenter",
    "com.apple.notificationcenterui",
];
const OWN_ID: &str = "com.orcrobert";

/// A macOS app's bundle identifier, such as `com.tinyspeck.slackmacgap`.
///
/// Invariants: lowercase ASCII (macOS compares bundle IDs case-insensitively),
/// at least two dot-separated labels, each made of `a-z`, `0-9` and `-`, and
/// at most 255 characters. It is never a path, so it can safely name things.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleId(String);

/// The name an app shows people, such as `Slack`. Display only: blocking
/// goes by [`BundleId`].
///
/// Invariants: trimmed, 1-100 characters, no control characters and no
/// invisible ones that reorder or hide text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppName(String);

/// Why an input could not be turned into a [`BundleId`] or [`AppName`]. The
/// messages are shown to the user as is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// No bundle ID was given.
    #[error("Choose an app to block")]
    Empty,
    /// The bundle ID breaks the reverse-DNS rules.
    #[error("That is not an app's bundle ID ({0}); pick the app from the list instead")]
    InvalidId(String),
    /// The app's display name is empty, too long or unprintable.
    #[error("The app's name is missing or unreadable")]
    InvalidName,
}

impl BundleId {
    /// Parses a bundle ID such as `com.apple.Safari`, folding case.
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let id = input.trim();
        if id.is_empty() {
            return Err(AppError::Empty);
        }
        let invalid = || AppError::InvalidId(shown(id));
        if id.len() > MAX_ID_LEN {
            return Err(invalid());
        }
        let mut labels = 0;
        for label in id.split('.') {
            let valid_chars = label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
            if label.is_empty() || !valid_chars {
                return Err(invalid());
            }
            labels += 1;
        }
        if labels < 2 {
            return Err(invalid());
        }
        Ok(Self(id.to_ascii_lowercase()))
    }

    /// The stored lowercase form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for apps the orc refuses to block: core parts of macOS and
    /// `OrcBlock` itself.
    #[must_use]
    pub fn is_protected(&self) -> bool {
        NEVER_BLOCK.contains(&self.0.as_str())
            || self
                .0
                .strip_prefix(OWN_ID)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
    }
}

impl AppName {
    /// Cleans a display name: surrounding whitespace is dropped.
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let name = input.trim();
        if name.is_empty()
            || name.chars().count() > MAX_NAME_CHARS
            || name.chars().any(|c| c.is_control() || is_invisible(c))
        {
            return Err(AppError::InvalidName);
        }
        Ok(Self(name.to_owned()))
    }

    /// Drops what [`AppName::parse`] refuses instead of failing; `None`
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

    /// A name for an app known only by its bundle ID.
    #[must_use]
    pub fn from_bundle_id(bundle_id: &BundleId) -> Self {
        Self(bundle_id.as_str().chars().take(MAX_NAME_CHARS).collect())
    }

    /// The name as shown.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The outermost `.app` directory in an absolute executable path, so a
/// helper deep inside `Slack.app/Contents/Frameworks` counts as Slack.
#[must_use]
pub fn outermost_bundle(path: &str) -> Option<&str> {
    bundles(path).next()
}

/// Every `.app` directory in an absolute executable path, outermost first.
/// The inner ones matter too: a copy of Slack inside a folder that is
/// itself named `Something.app` is still Slack.
pub fn bundles(path: &str) -> impl Iterator<Item = &str> {
    let absolute = path.starts_with('/');
    let mut start = 0;
    path.split('/').filter_map(move |component| {
        let end = start + component.len();
        start = end + 1;
        let bytes = component.as_bytes();
        let bundle = bytes.len() > 4 && bytes[bytes.len() - 4..].eq_ignore_ascii_case(b".app");
        (absolute && bundle).then(|| &path[..end])
    })
}

/// How much of a rejected bundle ID an error repeats; a request may carry
/// up to a megabyte of it.
const SHOWN_ID_CHARS: usize = 64;

fn shown(id: &str) -> String {
    let mut chars = id.chars();
    let mut shown: String = chars.by_ref().take(SHOWN_ID_CHARS).collect();
    if chars.next().is_some() {
        shown.push('…');
    }
    shown
}

/// Invisible characters that reorder, hide or break text (Unicode's format
/// characters and line separators), so a name could read as something else
/// on the shield and in the list. The zero-width joiner stays allowed: emoji
/// such as 👩‍💻 are built with it.
pub(crate) fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200C}'
            | '\u{200E}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

impl fmt::Display for BundleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for AppName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for BundleId {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for BundleId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BundleId {
    /// Re-validates on the way in; a hostile client or a hand-edited state
    /// file gets the same rules as typed input.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl Serialize for AppName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AppName {
    /// Cleans rather than refuses: a name saved before a release that
    /// refuses more characters must still load, or the state file would be
    /// set aside and a running block lost. The name is display only.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::cleaned(&raw).ok_or_else(|| serde::de::Error::custom(AppError::InvalidName))
    }
}

#[cfg(test)]
mod tests;
