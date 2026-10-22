use std::collections::BTreeSet;
use std::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{
    AppEntry, AppList, AppName, BlockDuration, Blocklist, BundleId, Domain, ListId, ListName,
    Month, PotionLedger, Session, TargetList,
};

/// The most lists a state holds.
pub const MAX_LISTS: usize = 32;

/// Everything the daemon persists: the lists of sites and apps, the running
/// block and the potion ledger.
///
/// Every command validates fully before it mutates, so a rejected command
/// leaves the state exactly as it was.
///
/// Invariants: at least one list, at most [`MAX_LISTS`], unique IDs, all
/// below `next_list_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredState")]
pub struct State {
    lists: Vec<TargetList>,
    next_list_id: u32,
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
    /// Sites cannot be removed from a list in the running block.
    #[error("The orc won't move: {0} stays blocked until the block ends")]
    Locked(Domain),
    /// The domain, or a parent of it, is already on the list.
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
    /// Apps cannot be removed from a list in the running block.
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
    /// No list has this ID.
    #[error("That list no longer exists")]
    ListNotFound(ListId),
    /// Another list already has this name, ignoring case.
    #[error("There is already a list named {0}")]
    ListNameTaken(ListName),
    /// [`MAX_LISTS`] lists exist already.
    #[error("The orc keeps at most 32 lists")]
    TooManyLists,
    /// The last list cannot be deleted.
    #[error("Keep at least one list")]
    LastList,
    /// A list in the running block cannot be deleted.
    #[error("The orc won't move: {0} is part of the running block")]
    ListLocked(ListName),
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
    /// A fresh state: one empty list, no block, every potion available in
    /// `month`.
    #[must_use]
    pub fn new(month: Month) -> Self {
        Self::with_first_list(
            Blocklist::default(),
            AppList::default(),
            None,
            PotionLedger::new(month),
        )
    }

    fn with_first_list(
        sites: Blocklist,
        apps: AppList,
        mut session: Option<Session>,
        potions: PotionLedger,
    ) -> Self {
        let id = ListId::new(1);
        if let Some(session) = &mut session {
            session.lists = BTreeSet::from([id]);
        }
        Self {
            lists: vec![TargetList {
                id,
                name: ListName::first(),
                sites,
                apps,
            }],
            next_list_id: 2,
            session,
            potions,
        }
    }

    /// The lists, in the order they were created.
    #[must_use]
    pub fn lists(&self) -> &[TargetList] {
        &self.lists
    }

    /// The list with `id`, if any.
    #[must_use]
    pub fn list(&self, id: ListId) -> Option<&TargetList> {
        self.lists.iter().find(|list| list.id == id)
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

    /// Creates an empty list and returns its ID.
    pub fn create_list(&mut self, name: ListName) -> Result<ListId, CommandError> {
        if self.lists.len() >= MAX_LISTS {
            return Err(CommandError::TooManyLists);
        }
        self.check_name_free(&name, None)?;
        let id = ListId::new(self.next_list_id);
        self.next_list_id += 1;
        self.lists.push(TargetList {
            id,
            name,
            sites: Blocklist::default(),
            apps: AppList::default(),
        });
        Ok(id)
    }

    /// Renames a list. Allowed during a block: a name blocks nothing.
    pub fn rename_list(&mut self, id: ListId, name: ListName) -> Result<(), CommandError> {
        self.check_name_free(&name, Some(id))?;
        self.list_mut(id)?.name = name;
        Ok(())
    }

    /// Deletes a list; refused for the last list and for a list in the
    /// running block.
    pub fn delete_list(&mut self, id: ListId) -> Result<(), CommandError> {
        let list = self.list(id).ok_or(CommandError::ListNotFound(id))?;
        if self.in_block(id) {
            return Err(CommandError::ListLocked(list.name.clone()));
        }
        if self.lists.len() == 1 {
            return Err(CommandError::LastList);
        }
        self.lists.retain(|list| list.id != id);
        Ok(())
    }

    /// Adds `domain` to a list. If the list is in the running block, the
    /// site is enforced at once and stays enforced until the block ends.
    pub fn add_site(
        &mut self,
        list: ListId,
        domain: Domain,
        now: Timestamp,
    ) -> Result<(), CommandError> {
        let joins_block = self.in_block(list);
        self.list_mut(list)?
            .sites
            .insert(domain.clone(), now)
            .map_err(|covered| CommandError::AlreadyListed {
                domain: domain.clone(),
                by: covered.by,
            })?;
        if let Some(session) = self.session.as_mut().filter(|_| joins_block) {
            session.add(domain);
        }
        Ok(())
    }

    /// Removes `domain` from a list; refused while the list is in the
    /// running block. Never changes what a running block enforces.
    pub fn remove_site(&mut self, list: ListId, domain: &Domain) -> Result<(), CommandError> {
        let locked = self.in_block(list);
        let target = self.list_mut(list)?;
        if !target.sites.contains(domain) {
            return Err(CommandError::NotListed(domain.clone()));
        }
        if locked {
            return Err(CommandError::Locked(domain.clone()));
        }
        target.sites.remove(domain);
        Ok(())
    }

    /// Adds an app to a list. If the list is in the running block, the app
    /// is closed from now on until the block ends.
    pub fn add_app(
        &mut self,
        list: ListId,
        bundle_id: BundleId,
        name: AppName,
        now: Timestamp,
    ) -> Result<(), CommandError> {
        if bundle_id.is_protected() {
            return Err(CommandError::Unblockable(name));
        }
        let joins_block = self.in_block(list);
        let target = self.list_mut(list)?;
        if let Some(listed) = target.apps.get(&bundle_id) {
            return Err(CommandError::AppAlreadyListed(listed.name.clone()));
        }
        target.apps.insert(AppEntry {
            bundle_id: bundle_id.clone(),
            name,
            added_at: now,
        });
        if let Some(session) = self.session.as_mut().filter(|_| joins_block) {
            session.add_app(bundle_id);
        }
        Ok(())
    }

    /// Removes an app from a list; refused while the list is in the running
    /// block. Never changes what a running block enforces.
    pub fn remove_app(&mut self, list: ListId, bundle_id: &BundleId) -> Result<(), CommandError> {
        let locked = self.in_block(list);
        let target = self.list_mut(list)?;
        let Some(listed) = target.apps.get(bundle_id) else {
            return Err(CommandError::AppNotListed(bundle_id.clone()));
        };
        if locked {
            return Err(CommandError::AppLocked(listed.name.clone()));
        }
        target.apps.remove(bundle_id);
        Ok(())
    }

    /// Starts a block over every site and app on `lists`.
    pub fn start_block(
        &mut self,
        lists: &[ListId],
        duration: BlockDuration,
        now: Timestamp,
    ) -> Result<&Session, CommandError> {
        if self.is_blocking() {
            return Err(CommandError::AlreadyBlocking);
        }
        let mut chosen = Vec::with_capacity(lists.len());
        for &id in lists {
            chosen.push(self.list(id).ok_or(CommandError::ListNotFound(id))?);
        }
        let domains: BTreeSet<Domain> = chosen
            .iter()
            .flat_map(|list| list.sites.domains())
            .cloned()
            .collect();
        // Each list is free of covered subdomains, but two lists together
        // may hold youtube.com and m.youtube.com.
        let domains: BTreeSet<Domain> = domains
            .iter()
            .filter(|domain| {
                !domains
                    .iter()
                    .any(|other| other != *domain && other.covers(domain))
            })
            .cloned()
            .collect();
        let apps: BTreeSet<BundleId> = chosen
            .iter()
            .flat_map(|list| list.apps.bundle_ids())
            .cloned()
            .collect();
        if domains.is_empty() && apps.is_empty() {
            return Err(CommandError::EmptyBlocklist);
        }
        let ids = chosen.iter().map(|list| list.id).collect();
        Ok(self
            .session
            .insert(Session::new(ids, domains, apps, duration, now)))
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

    fn in_block(&self, list: ListId) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.includes(list))
    }

    fn list_mut(&mut self, id: ListId) -> Result<&mut TargetList, CommandError> {
        self.lists
            .iter_mut()
            .find(|list| list.id == id)
            .ok_or(CommandError::ListNotFound(id))
    }

    fn check_name_free(&self, name: &ListName, except: Option<ListId>) -> Result<(), CommandError> {
        let taken = self
            .lists
            .iter()
            .any(|list| Some(list.id) != except && list.name.same_as(name));
        if taken {
            Err(CommandError::ListNameTaken(name.clone()))
        } else {
            Ok(())
        }
    }
}

/// What `state.json` may hold: the current shape, or the single list of
/// releases before lists (`blocklist` and `apps` at the top level).
#[derive(Deserialize)]
struct StoredState {
    #[serde(default)]
    lists: Vec<TargetList>,
    #[serde(default)]
    next_list_id: u32,
    #[serde(default)]
    blocklist: Blocklist,
    #[serde(default)]
    apps: AppList,
    session: Option<Session>,
    potions: PotionLedger,
}

impl TryFrom<StoredState> for State {
    type Error = String;

    /// Refuses only what cannot be repaired without guessing: duplicate list
    /// IDs. Everything else loads, because a state file that fails to load
    /// is set aside and its running block lost.
    fn try_from(stored: StoredState) -> Result<Self, Self::Error> {
        if stored.lists.is_empty() {
            return Ok(Self::with_first_list(
                stored.blocklist,
                stored.apps,
                stored.session,
                stored.potions,
            ));
        }
        let ids: BTreeSet<ListId> = stored.lists.iter().map(TargetList::id).collect();
        if ids.len() != stored.lists.len() {
            return Err("state lists the same list id twice".to_owned());
        }
        let highest = ids.last().map_or(0, |id| id.get());
        Ok(Self {
            lists: stored.lists,
            next_list_id: stored.next_list_id.max(highest.saturating_add(1)),
            session: stored.session,
            potions: stored.potions,
        })
    }
}

#[cfg(test)]
mod tests;
