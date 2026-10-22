use std::collections::BTreeMap;
use std::io;
use std::time::Duration;

use jiff::Timestamp;
use orcblock_core::{AppName, BlockDuration, BundleId, CommandError, Domain, ListName, State};
use orcblock_ipc::{Command, ErrorKind, Mode, Response, Status};
use tracing::{error, info, warn};

use crate::{AppGuard, Blocked, Clock, Enforcer, Store, StoreError};

/// How often a running block is persisted and its enforcement verified.
pub const VERIFY_EVERY: Duration = Duration::from_secs(15);

/// How often a block with apps looks for blocked apps to close.
pub const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// The most time credited for a period the daemon was not running (a crash
/// or a powered-off Mac). Bounds what setting the clock forward while the
/// daemon is down can achieve; a clock set back credits nothing.
pub const MAX_RESTART_CREDIT: Duration = Duration::from_hours(24);

/// The daemon's request handler and timekeeper.
///
/// Every change is validated by core on a copy of the state, persisted, and
/// only then adopted and enforced, so a failed save never leaves the daemon
/// enforcing something it would forget on restart.
#[derive(Debug)]
pub struct Service<C, E> {
    state: State,
    store: Store,
    clock: C,
    enforcer: E,
    blocked: Blocked,
    apps: AppGuard,
    mode: Mode,
    last_reading: Duration,
    since_verify: Duration,
    since_sweep: Duration,
}

/// The daemon could not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// The state could not be loaded or saved.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The monotonic clock could not be read.
    #[error("read the monotonic clock: {0}")]
    Clock(#[source] io::Error),
}

impl<C: Clock, E: Enforcer> Service<C, E> {
    /// Loads the saved state, credits the time the daemon was down, and
    /// enforces whatever block is still running before returning, closing
    /// its blocked apps through `apps`.
    pub fn start(
        store: Store,
        clock: C,
        enforcer: E,
        blocked: Blocked,
        apps: AppGuard,
        mode: Mode,
    ) -> Result<Self, StartError> {
        let now = clock.now();
        let last_reading = clock.monotonic().map_err(StartError::Clock)?;
        let state = match store.load() {
            Ok(Some(saved)) => {
                let mut state = saved.state;
                let credit = restart_credit(saved.saved_at, now);
                if state.advance(credit).is_some() {
                    info!(?credit, "the block ended while the daemon was down");
                }
                state
            }
            Ok(None) => State::new(clock.month()),
            Err(err @ StoreError::Corrupt { .. }) => {
                let aside = store.quarantine()?;
                error!(%err, aside = %aside.display(), "starting from an empty state");
                State::new(clock.month())
            }
            Err(err) => return Err(err.into()),
        };
        store.save(&state, now)?;

        let mut service = Self {
            state,
            store,
            clock,
            enforcer,
            blocked,
            apps,
            mode,
            last_reading,
            since_verify: Duration::ZERO,
            since_sweep: Duration::ZERO,
        };
        service.reconcile(false);
        Ok(service)
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> &State {
        &self.state
    }

    /// The enforcer, for inspection.
    #[must_use]
    pub fn enforcer(&self) -> &E {
        &self.enforcer
    }

    /// The snapshot clients see.
    #[must_use]
    pub fn status(&self) -> Status {
        let mut status = Status::new(
            &self.state,
            self.clock.now(),
            self.clock.month(),
            self.mode,
            env!("CARGO_PKG_VERSION"),
        );
        status.blocked_launches = self.apps.launches();
        status
    }

    /// Answers one request. The clock is advanced first, so a block that
    /// just ran out is over before the command is judged.
    pub fn handle(&mut self, command: Command) -> Response {
        self.tick();
        let now = self.clock.now();
        match command {
            Command::Status => self.ok(),
            Command::CreateList { name } => match ListName::parse(&name) {
                Ok(name) => self.change(|state| state.create_list(name).map(drop)),
                Err(err) => refusal(ErrorKind::InvalidList, &err),
            },
            Command::RenameList { list, name } => match ListName::parse(&name) {
                Ok(name) => self.change(|state| state.rename_list(list, name)),
                Err(err) => refusal(ErrorKind::InvalidList, &err),
            },
            Command::DeleteList { list } => self.change(|state| state.delete_list(list)),
            Command::AddSite { list, site } => match Domain::parse(&site) {
                Ok(domain) => self.change(|state| state.add_site(list, domain, now)),
                Err(err) => refusal(ErrorKind::InvalidSite, &err),
            },
            Command::RemoveSite { list, site } => match Domain::parse(&site) {
                Ok(domain) => self.change(|state| state.remove_site(list, &domain)),
                Err(err) => refusal(ErrorKind::InvalidSite, &err),
            },
            Command::AddApp {
                list,
                bundle_id,
                name,
            } => {
                match BundleId::parse(&bundle_id).and_then(|id| Ok((id, AppName::parse(&name)?))) {
                    Ok((bundle_id, name)) => {
                        self.change(|state| state.add_app(list, bundle_id, name, now))
                    }
                    Err(err) => refusal(ErrorKind::InvalidApp, &err),
                }
            }
            Command::RemoveApp { list, bundle_id } => match BundleId::parse(&bundle_id) {
                Ok(bundle_id) => self.change(|state| state.remove_app(list, &bundle_id)),
                Err(err) => refusal(ErrorKind::InvalidApp, &err),
            },
            Command::StartBlock { secs, lists } => match BlockDuration::from_secs(secs) {
                Ok(duration) => {
                    self.change(|state| state.start_block(&lists, duration, now).map(drop))
                }
                Err(err) => refusal(ErrorKind::InvalidDuration, &err),
            },
            Command::DrinkPotion => {
                let month = self.clock.month();
                self.change(|state| state.drink_potion(month).map(drop))
            }
        }
    }

    /// Counts monotonic time off the running block; ends it when time runs
    /// out; closes blocked apps every [`SWEEP_EVERY`]; persists and verifies
    /// enforcement every [`VERIFY_EVERY`].
    pub fn tick(&mut self) {
        let reading = match self.clock.monotonic() {
            Ok(reading) => reading,
            Err(err) => {
                warn!(%err, "monotonic clock unreadable; the block is not counting down");
                return;
            }
        };
        let elapsed = reading.saturating_sub(self.last_reading);
        self.last_reading = reading;

        if let Some(end) = self.state.advance(elapsed) {
            info!(?end, "block over");
            self.persist();
            self.reconcile(false);
            return;
        }
        if !self.state.is_blocking() {
            return;
        }
        self.since_sweep += elapsed;
        if self.since_sweep >= SWEEP_EVERY {
            self.since_sweep = Duration::ZERO;
            self.sweep_apps(true);
        }
        self.since_verify += elapsed;
        if self.since_verify >= VERIFY_EVERY {
            self.since_verify = Duration::ZERO;
            self.persist();
            self.verify();
        }
    }

    fn ok(&self) -> Response {
        Response::Ok {
            status: self.status(),
        }
    }

    fn change(&mut self, rule: impl FnOnce(&mut State) -> Result<(), CommandError>) -> Response {
        let mut next = self.state.clone();
        if let Err(err) = rule(&mut next) {
            return Response::from(&err);
        }
        if let Err(err) = self.store.save(&next, self.clock.now()) {
            error!(%err, "refusing a change that could not be saved");
            return Response::error(
                ErrorKind::Internal,
                "The orc could not write down the change; nothing was changed",
            );
        }
        // A change to the blocked apps (a block starting, an app added)
        // closes apps that were already open: no shield for those. Any
        // other change leaves recording on, so a launch it happens to catch
        // still gets one.
        let record = next.blocked_apps() == self.state.blocked_apps();
        self.state = next;
        self.reconcile(record);
        self.ok()
    }

    /// Persists the state now, as on shutdown.
    pub fn save(&self) -> Result<(), StoreError> {
        self.store.save(&self.state, self.clock.now())
    }

    fn persist(&self) {
        if let Err(err) = self.save() {
            error!(%err, "could not save the state");
        }
    }

    /// Makes enforcement match the state. The sinkhole learns about new
    /// domains before the resolver files send lookups to it, and forgets
    /// them only after the files are gone. `record` says whether apps it
    /// closes count as blocked launches.
    fn reconcile(&mut self, record: bool) {
        let domains = self.state.blocked_domains();
        let result = if domains.is_empty() {
            let result = self.enforcer.clear();
            self.blocked.replace(domains);
            result
        } else {
            self.blocked.replace(domains.clone());
            self.enforcer.apply(&domains)
        };
        if let Err(err) = result {
            error!(%err, "enforcement failed; retrying on the next verify");
        }
        self.sweep_apps(record);
    }

    /// Closes running blocked apps. `record` is false when the set of
    /// blocked apps has just been decided (block start, an app added, a
    /// restart): those apps were open before, so they get no shield.
    fn sweep_apps(&mut self, record: bool) {
        let blocked = self.state.blocked_apps();
        if blocked.is_empty() {
            self.apps.idle();
            return;
        }
        let names: BTreeMap<_, _> = blocked
            .into_iter()
            .map(|bundle_id| {
                let name = self
                    .state
                    .lists()
                    .iter()
                    .find_map(|list| list.apps().get(&bundle_id))
                    // Only a hand-edited state file blocks an app missing
                    // from the list; it is still closed, under its ID.
                    .map_or_else(
                        || AppName::from_bundle_id(&bundle_id),
                        |entry| entry.name.clone(),
                    );
                (bundle_id, name)
            })
            .collect();
        self.apps
            .sweep(&names, self.last_reading, self.clock.now(), record);
    }

    fn verify(&mut self) {
        let domains = self.state.blocked_domains();
        match self.enforcer.verify(&domains) {
            Ok(true) => warn!("enforcement had drifted and was repaired"),
            Ok(false) => {}
            Err(err) => error!(%err, "verify failed; retrying on the next verify"),
        }
    }
}

/// The time to credit for a gap between saves across a restart.
fn restart_credit(saved_at: Timestamp, now: Timestamp) -> Duration {
    Duration::try_from(now.duration_since(saved_at))
        .unwrap_or(Duration::ZERO)
        .min(MAX_RESTART_CREDIT)
}

fn refusal(kind: ErrorKind, err: &impl ToString) -> Response {
    Response::error(kind, err.to_string())
}

#[cfg(test)]
mod tests;
