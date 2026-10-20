use std::collections::BTreeSet;
use std::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{
    AppEntry, AppList, AppName, BlockDuration, Blocklist, BundleId, Domain, Month, PotionLedger,
    Session,
};

/// Everything the daemon persists: the sites and apps, the running block and
/// the potion ledger.
///
/// Every command validates fully before it mutates, so a rejected command
/// leaves the state exactly as it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    blocklist: Blocklist,
    #[serde(default)]
    apps: AppList,
    session: Option<Session>,
    potions: PotionLedger,
}

/// Why a block ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum SessionEnd {
    /// The chosen duration ran out.
    Finished,
    /// A sleep potion ended it early; `left` potions remain this month.
    Potion {
        /// Potions left this month after this one.
        left: u8,
    },
}

/// A command the rules refuse. The messages are shown to the user as is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    /// Sites cannot be removed while a block runs.
    #[error("The orc won't move: {0} stays blocked until the block ends")]
    Locked(Domain),
    /// The domain, or a parent of it, is already listed.
    #[error("{domain} is already blocked by {by}")]
    AlreadyListed {
        /// The domain that was asked for.
        domain: Domain,
        /// The listed domain that covers it.
        by: Domain,
    },
    /// The domain is not on the list.
    #[error("{0} is not on the list")]
    NotListed(Domain),
    /// Apps cannot be removed while a block runs.
    #[error("The orc won't move: {0} stays blocked until the block ends")]
    AppLocked(AppName),
    /// The app is already on the list.
    #[error("{0} is already on the list")]
    AppAlreadyListed(AppName),
    /// The app is not on the list.
    #[error("{0} is not on the list")]
    AppNotListed(BundleId),
    /// The app is part of macOS or `OrcBlock` and is never blocked.
    #[error("The orc won't block {0}: your Mac or OrcBlock needs it")]
    Unblockable(AppName),
    /// A block needs at least one site or app.
    #[error("Add a site or an app before starting a block")]
    EmptyBlocklist,
    /// Only one block runs at a time.
    #[error("A block is already running")]
    AlreadyBlocking,
    /// There is no block to end.
    #[error("No block is running")]
    NotBlocking,
    /// Every potion this month is spent.
    #[error("No sleep potions left; the orc brews more on {refill}")]
    NoPotions {
        /// The first day potions are available again.
        refill: jiff::civil::Date,
    },
}

impl State {
    /// A fresh state: empty list, no block, every potion available in
    /// `month`.
    #[must_use]
    pub fn new(month: Month) -> Self {
        Self {
            blocklist: Blocklist::default(),
            apps: AppList::default(),
            session: None,
            potions: PotionLedger::new(month),
        }
    }

    /// The list of sites.
    #[must_use]
    pub fn blocklist(&self) -> &Blocklist {
        &self.blocklist
    }

    /// The list of apps.
    #[must_use]
    pub fn apps(&self) -> &AppList {
        &self.apps
    }

    /// The running block, if any.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The potion ledger.
    #[must_use]
    pub fn potions(&self) -> &PotionLedger {
        &self.potions
    }

    /// True while a block runs.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.session.is_some()
    }

    /// The domains to enforce right now: the running block's, or none.
    #[must_use]
    pub fn blocked_domains(&self) -> BTreeSet<Domain> {
        self.session
            .as_ref()
            .map(|session| session.domains().clone())
            .unwrap_or_default()
    }

    /// The apps to close right now: the running block's, or none. Protected
    /// apps are left out even if a hand-edited state file lists them.
    #[must_use]
    pub fn blocked_apps(&self) -> BTreeSet<BundleId> {
        self.session
            .iter()
            .flat_map(Session::apps)
            .filter(|bundle_id| !bundle_id.is_protected())
            .cloned()
            .collect()
    }

    /// Lists `domain`. During a block it is enforced at once and stays
    /// enforced until the block ends.
    pub fn add_site(&mut self, domain: Domain, now: Timestamp) -> Result<(), CommandError> {
        self.blocklist
            .insert(domain.clone(), now)
            .map_err(|covered| CommandError::AlreadyListed {
                domain: domain.clone(),
                by: covered.by,
            })?;
        if let Some(session) = &mut self.session {
            session.add(domain);
        }
        Ok(())
    }

    /// Unlists `domain`; refused while a block runs.
    pub fn remove_site(&mut self, domain: &Domain) -> Result<(), CommandError> {
        if !self.blocklist.contains(domain) {
            return Err(CommandError::NotListed(domain.clone()));
        }
        if self.is_blocking() {
            return Err(CommandError::Locked(domain.clone()));
        }
        self.blocklist.remove(domain);
        Ok(())
    }

    /// Lists an app. During a block it is closed from now on and stays
    /// blocked until the block ends.
    pub fn add_app(
        &mut self,
        bundle_id: BundleId,
        name: AppName,
        now: Timestamp,
    ) -> Result<(), CommandError> {
        if bundle_id.is_protected() {
            return Err(CommandError::Unblockable(name));
        }
        if let Some(listed) = self.apps.get(&bundle_id) {
            return Err(CommandError::AppAlreadyListed(listed.name.clone()));
        }
        self.apps.insert(AppEntry {
            bundle_id: bundle_id.clone(),
            name,
            added_at: now,
        });
        if let Some(session) = &mut self.session {
            session.add_app(bundle_id);
        }
        Ok(())
    }

    /// Unlists an app; refused while a block runs.
    pub fn remove_app(&mut self, bundle_id: &BundleId) -> Result<(), CommandError> {
        let Some(listed) = self.apps.get(bundle_id) else {
            return Err(CommandError::AppNotListed(bundle_id.clone()));
        };
        if self.is_blocking() {
            return Err(CommandError::AppLocked(listed.name.clone()));
        }
        self.apps.remove(bundle_id);
        Ok(())
    }

    /// Starts a block over every listed site and app.
    pub fn start_block(
        &mut self,
        duration: BlockDuration,
        now: Timestamp,
    ) -> Result<&Session, CommandError> {
        if self.is_blocking() {
            return Err(CommandError::AlreadyBlocking);
        }
        if self.blocklist.is_empty() && self.apps.is_empty() {
            return Err(CommandError::EmptyBlocklist);
        }
        let domains = self.blocklist.domains().cloned().collect();
        let apps = self.apps.bundle_ids().cloned().collect();
        Ok(self
            .session
            .insert(Session::new(domains, apps, duration, now)))
    }

    /// Counts `elapsed` off the running block and ends it when no time is
    /// left.
    pub fn advance(&mut self, elapsed: Duration) -> Option<SessionEnd> {
        let session = self.session.as_mut()?;
        session.advance(elapsed);
        if session.is_over() {
            self.session = None;
            Some(SessionEnd::Finished)
        } else {
            None
        }
    }

    /// Ends the running block early by spending a sleep potion of `month`.
    pub fn drink_potion(&mut self, month: Month) -> Result<SessionEnd, CommandError> {
        if !self.is_blocking() {
            return Err(CommandError::NotBlocking);
        }
        let left = self
            .potions
            .spend(month)
            .ok_or_else(|| CommandError::NoPotions {
                refill: self.potions.refill(month).first_day(),
            })?;
        self.session = None;
        Ok(SessionEnd::Potion { left })
    }
}

#[cfg(test)]
mod tests;
