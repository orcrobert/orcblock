use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Domain;

/// One site on the list and when it was added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The site; blocking it also blocks its subdomains.
    pub domain: Domain,
    /// When the user added it.
    pub added_at: Timestamp,
}

/// The sites the user wants blocked, in the order they were added.
///
/// Invariant: no entry covers another, so the list never holds both
/// `youtube.com` and `m.youtube.com`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Entry>", into = "Vec<Entry>")]
pub struct Blocklist {
    entries: Vec<Entry>,
}

/// Returned by [`Blocklist::insert`] when an existing entry already covers
/// the new domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Covered {
    pub(crate) by: Domain,
}

impl Blocklist {
    /// The entries in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The listed domains in the order they were added.
    pub fn domains(&self) -> impl Iterator<Item = &Domain> {
        self.entries.iter().map(|entry| &entry.domain)
    }

    /// True when the list has no sites.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// True when a listed domain is exactly `domain`.
    #[must_use]
    pub fn contains(&self, domain: &Domain) -> bool {
        self.domains().any(|listed| listed == domain)
    }

    /// The listed domain that covers `domain`, if any.
    #[must_use]
    pub fn covering(&self, domain: &Domain) -> Option<&Domain> {
        self.domains().find(|listed| listed.covers(domain))
    }

    /// Adds `domain`, replacing any listed subdomains it now covers, and
    /// returns the replaced ones.
    pub(crate) fn insert(
        &mut self,
        domain: Domain,
        now: Timestamp,
    ) -> Result<Vec<Domain>, Covered> {
        if let Some(by) = self.covering(&domain) {
            return Err(Covered { by: by.clone() });
        }
        let mut replaced = Vec::new();
        self.entries.retain(|entry| {
            let covered = domain.covers(&entry.domain);
            if covered {
                replaced.push(entry.domain.clone());
            }
            !covered
        });
        self.entries.push(Entry {
            domain,
            added_at: now,
        });
        Ok(replaced)
    }

    /// Removes `domain` if it is listed exactly; returns whether it was.
    pub(crate) fn remove(&mut self, domain: &Domain) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| &entry.domain != domain);
        self.entries.len() != before
    }
}

impl TryFrom<Vec<Entry>> for Blocklist {
    type Error = String;

    fn try_from(entries: Vec<Entry>) -> Result<Self, Self::Error> {
        for (i, a) in entries.iter().enumerate() {
            for b in &entries[i + 1..] {
                if a.domain.covers(&b.domain) || b.domain.covers(&a.domain) {
                    return Err(format!(
                        "blocklist lists both {} and {}",
                        a.domain.as_ascii(),
                        b.domain.as_ascii()
                    ));
                }
            }
        }
        Ok(Self { entries })
    }
}

impl From<Blocklist> for Vec<Entry> {
    fn from(list: Blocklist) -> Self {
        list.entries
    }
}

#[cfg(test)]
mod tests;
