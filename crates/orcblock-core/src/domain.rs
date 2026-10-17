use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MAX_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

/// A registrable site name such as `youtube.com`, cleaned from whatever the
/// user typed or pasted.
///
/// Invariants: lowercase ASCII (Unicode names are stored as punycode), at
/// least two labels, each 1-63 characters of `a-z`, `0-9` and inner `-`, at
/// most 253 characters, no leading `www.`, and a top-level label that is not
/// all digits (so IPv4 addresses never parse). Blocking a domain also blocks
/// all of its subdomains.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(String);

/// Why an input could not be turned into a [`Domain`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Nothing but whitespace or punctuation was given.
    #[error("enter a site, like youtube.com")]
    Empty,
    /// The input is an IP address; only names can be blocked.
    #[error("{0} is an IP address; enter the site's name instead")]
    IpAddress(String),
    /// The name has a single label, like `localhost`.
    #[error("{0} is not a full site name; try something like {0}.com")]
    SingleLabel(String),
    /// The name breaks DNS syntax rules.
    #[error("{0} is not a valid site name")]
    Invalid(String),
}

impl Domain {
    /// Cleans user input into a domain.
    ///
    /// Accepts bare names and full URLs: the scheme, credentials, port, path,
    /// query, fragment, a trailing dot and a leading `www.` are dropped, case
    /// is folded and Unicode names are converted to punycode.
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let host = extract_host(input)?;
        if host.is_empty() {
            return Err(DomainError::Empty);
        }
        let ascii = idna::domain_to_ascii(&host).map_err(|_| DomainError::Invalid(host.clone()))?;
        validate(&ascii, &host)?;
        Ok(Self(strip_www(ascii)))
    }

    /// The stored ASCII form, as used in DNS and in file names.
    #[must_use]
    pub fn as_ascii(&self) -> &str {
        &self.0
    }

    /// The form to show people: Unicode labels instead of punycode.
    #[must_use]
    pub fn to_unicode(&self) -> String {
        let (unicode, result) = idna::domain_to_unicode(&self.0);
        if result.is_ok() {
            unicode
        } else {
            self.0.clone()
        }
    }

    /// True when blocking `self` also blocks `other`: they are equal or
    /// `other` is a subdomain of `self`.
    #[must_use]
    pub fn covers(&self, other: &Domain) -> bool {
        other.0 == self.0
            || other
                .0
                .strip_suffix(self.0.as_str())
                .is_some_and(|prefix| prefix.ends_with('.'))
    }

    /// True when `name` (a DNS query name in any case, with or without the
    /// trailing dot) is this domain or one of its subdomains.
    #[must_use]
    pub fn matches_name(&self, name: &str) -> bool {
        let name = name.strip_suffix('.').unwrap_or(name);
        if name.len() < self.0.len() {
            return false;
        }
        let (prefix, suffix) = name.split_at(name.len() - self.0.len());
        suffix.eq_ignore_ascii_case(&self.0) && (prefix.is_empty() || prefix.ends_with('.'))
    }
}

fn extract_host(input: &str) -> Result<String, DomainError> {
    let mut rest = input.trim();
    if let Some((scheme, after)) = rest.split_once("://")
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    {
        rest = after;
    }
    rest = rest.trim_start_matches('/');
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    rest = &rest[..end];
    if let Some((_, host)) = rest.rsplit_once('@') {
        rest = host;
    }
    if rest.starts_with('[') {
        let ip = rest
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or_default();
        return Err(DomainError::IpAddress(ip.to_owned()));
    }
    if let Some((host, port)) = rest.rsplit_once(':') {
        if !port.chars().all(|c| c.is_ascii_digit()) {
            return Err(DomainError::Invalid(rest.to_owned()));
        }
        rest = host;
    }
    let host = rest.strip_suffix('.').unwrap_or(rest);
    if host.contains(char::is_whitespace) {
        return Err(DomainError::Invalid(host.to_owned()));
    }
    Ok(host.to_owned())
}

fn validate(ascii: &str, original: &str) -> Result<(), DomainError> {
    let invalid = || DomainError::Invalid(original.to_owned());
    if ascii.is_empty() || ascii.len() > MAX_LEN {
        return Err(invalid());
    }
    let labels: Vec<&str> = ascii.split('.').collect();
    for label in &labels {
        let valid_chars = label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if label.is_empty()
            || label.len() > MAX_LABEL_LEN
            || !valid_chars
            || label.starts_with('-')
            || label.ends_with('-')
        {
            return Err(invalid());
        }
    }
    let tld = labels.last().copied().unwrap_or_default();
    if tld.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DomainError::IpAddress(original.to_owned()));
    }
    if labels.len() < 2 {
        return Err(DomainError::SingleLabel(original.to_owned()));
    }
    Ok(())
}

fn strip_www(ascii: String) -> String {
    match ascii.strip_prefix("www.") {
        Some(rest) if rest.contains('.') => rest.to_owned(),
        _ => ascii,
    }
}

impl fmt::Display for Domain {
    /// Writes the Unicode form; use [`Domain::as_ascii`] for DNS and files.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_unicode())
    }
}

impl FromStr for Domain {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for Domain {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Domain {
    /// Re-validates on the way in, so a hand-edited state file or a hostile
    /// client cannot smuggle in a name that breaks the invariants.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let domain = Self::parse(&raw).map_err(serde::de::Error::custom)?;
        if domain.0 == raw {
            Ok(domain)
        } else {
            Err(serde::de::Error::custom(format!(
                "{raw} is not a cleaned domain"
            )))
        }
    }
}

#[cfg(test)]
mod tests;
