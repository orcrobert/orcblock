use std::collections::BTreeSet;
use std::fs;

use orcblock_core::Month;

use super::*;
use crate::DryRunEnforcer;
use crate::clock::fake::FakeClock;
use crate::enforce::{DryRunCall, EnforceError};

const START: i64 = 1_800_000_000;

fn october() -> Month {
    Month::new(2026, 10).expect("valid")
}

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid")
}

struct Rig {
    dir: tempfile::TempDir,
    clock: FakeClock,
    blocked: Blocked,
}

impl Rig {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temp dir"),
            clock: FakeClock::new(Timestamp::from_second(START).expect("valid"), october()),
            blocked: Blocked::default(),
        }
    }

    fn store(&self) -> Store {
        Store::new(self.dir.path().join("state.json"))
    }

    fn start(&self) -> Service<FakeClock, DryRunEnforcer> {
        self.start_with(DryRunEnforcer::default())
    }

    fn start_with<E: Enforcer>(&self, enforcer: E) -> Service<FakeClock, E> {
        Service::start(
            self.store(),
            self.clock.clone(),
            enforcer,
            self.blocked.clone(),
            Mode::DryRun,
        )
        .expect("starts")
    }
}

fn status(response: Response) -> Status {
    match response {
        Response::Ok { status } => status,
        Response::Error { kind, message } => panic!("{kind:?}: {message}"),
    }
}

fn refusal(response: Response) -> (ErrorKind, String) {
    match response {
        Response::Error { kind, message } => (kind, message),
        Response::Ok { .. } => panic!("expected a refusal"),
    }
}

fn add(service: &mut Service<FakeClock, impl Enforcer>, site: &str) -> Status {
    status(service.handle(Command::AddSite { site: site.into() }))
}

#[test]
fn adds_cleaned_sites_and_reports_bad_ones() {
    let rig = Rig::new();
    let mut service = rig.start();
    let after = add(&mut service, "https://www.YouTube.com/watch?v=x");
    assert_eq!(after.sites[0].domain, d("youtube.com"));

    let (kind, message) = refusal(service.handle(Command::AddSite {
        site: "localhost".into(),
    }));
    assert_eq!(kind, ErrorKind::InvalidSite);
    assert_eq!(
        message,
        "Localhost is not a full site name; try something like localhost.com"
    );

    let (kind, _) = refusal(service.handle(Command::StartBlock { secs: 5 }));
    assert_eq!(kind, ErrorKind::InvalidDuration);
}

#[test]
fn a_block_is_enforced_counted_down_and_cleared() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    assert!(!rig.blocked.is_active(), "listing alone blocks nothing");

    let started = status(service.handle(Command::StartBlock { secs: 120 }));
    assert_eq!(started.block.as_ref().map(|b| b.remaining_secs), Some(120));
    assert!(rig.blocked.blocks_name("m.youtube.com"));
    assert_eq!(
        service.enforcer().applied(),
        &BTreeSet::from([d("youtube.com")])
    );

    rig.clock.advance(Duration::from_secs(119));
    service.tick();
    assert!(service.state().is_blocking());
    rig.clock.advance(Duration::from_secs(1));
    service.tick();
    assert!(!service.state().is_blocking());
    assert!(!rig.blocked.is_active());
    assert!(service.enforcer().applied().is_empty());
    assert_eq!(service.enforcer().calls().last(), Some(&DryRunCall::Clear));
}

#[test]
fn wall_clock_changes_do_not_shorten_a_block() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    service.handle(Command::StartBlock { secs: 3600 });

    rig.clock
        .set_wall(Timestamp::from_second(START + 86_400 * 30).expect("valid"));
    service.tick();
    let block = service.status().block.expect("still blocking");
    assert_eq!(block.remaining_secs, 3600);
}

#[test]
fn an_unreadable_monotonic_clock_never_ends_a_block() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    service.handle(Command::StartBlock { secs: 60 });
    rig.clock.break_monotonic();
    rig.clock.advance(Duration::from_secs(600));
    service.tick();
    assert!(service.state().is_blocking());
}

#[test]
fn verifies_and_persists_every_interval_while_blocking() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    service.handle(Command::StartBlock { secs: 600 });
    let verifies = |s: &Service<FakeClock, DryRunEnforcer>| {
        s.enforcer()
            .calls()
            .iter()
            .filter(|c| matches!(c, DryRunCall::Verify(_)))
            .count()
    };

    rig.clock
        .advance(VERIFY_EVERY.saturating_sub(Duration::from_secs(1)));
    service.tick();
    assert_eq!(verifies(&service), 0);
    rig.clock.advance(Duration::from_secs(1));
    service.tick();
    assert_eq!(verifies(&service), 1);

    let saved = rig.store().load().expect("loads").expect("saved");
    assert_eq!(
        saved.state.session().map(|s| s.remaining().as_secs()),
        Some(585)
    );
}

#[test]
fn restart_keeps_the_block_and_credits_downtime() {
    let rig = Rig::new();
    {
        let mut service = rig.start();
        add(&mut service, "youtube.com");
        service.handle(Command::StartBlock { secs: 3600 });
    }
    rig.blocked.replace(BTreeSet::new());
    rig.clock.advance(Duration::from_secs(600));

    let service = rig.start();
    assert_eq!(service.status().block.map(|b| b.remaining_secs), Some(3000));
    assert!(
        rig.blocked.blocks_name("youtube.com"),
        "enforced again before serving"
    );
    assert_eq!(
        service.enforcer().calls(),
        [DryRunCall::Apply(BTreeSet::from([d("youtube.com")]))]
    );
}

#[test]
fn restart_credit_is_clamped_and_never_negative() {
    let at = |secs| Timestamp::from_second(START + secs).expect("valid");
    assert_eq!(restart_credit(at(0), at(90)), Duration::from_secs(90));
    assert_eq!(restart_credit(at(0), at(-90)), Duration::ZERO);
    assert_eq!(restart_credit(at(0), at(86_400 * 365)), MAX_RESTART_CREDIT);
}

#[test]
fn a_corrupt_state_file_is_moved_aside() {
    let rig = Rig::new();
    fs::write(rig.store().path(), "garbage").expect("writes");
    let service = rig.start();
    assert!(service.state().blocklist().is_empty());
    assert!(rig.dir.path().join("state.json.corrupt").exists());
}

#[test]
fn potions_end_blocks_and_run_out() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    let (kind, _) = refusal(service.handle(Command::DrinkPotion));
    assert_eq!(kind, ErrorKind::NotBlocking);

    for left in (0..5).rev() {
        service.handle(Command::StartBlock { secs: 3600 });
        let after = status(service.handle(Command::DrinkPotion));
        assert_eq!(after.potions.available, left);
        assert_eq!(after.block, None);
        assert!(!rig.blocked.is_active());
    }
    service.handle(Command::StartBlock { secs: 3600 });
    let (kind, message) = refusal(service.handle(Command::DrinkPotion));
    assert_eq!(kind, ErrorKind::NoPotions);
    assert_eq!(
        message,
        "No sleep potions left; the orc brews more on 2026-11-01"
    );

    rig.clock.set_month(Month::new(2026, 11).expect("valid"));
    assert_eq!(
        status(service.handle(Command::DrinkPotion))
            .potions
            .available,
        4
    );
}

#[test]
fn a_change_that_cannot_be_saved_is_refused() {
    let rig = Rig::new();
    let mut service = rig.start();
    fs::remove_file(rig.store().path()).expect("removes");
    fs::create_dir(rig.store().path()).expect("blocks the path with a directory");

    let (kind, _) = refusal(service.handle(Command::AddSite {
        site: "youtube.com".into(),
    }));
    assert_eq!(kind, ErrorKind::Internal);
    assert!(service.state().blocklist().is_empty());
}

#[derive(Debug, Default)]
struct BrokenEnforcer;

impl Enforcer for BrokenEnforcer {
    fn apply(&mut self, _: &BTreeSet<Domain>) -> Result<(), EnforceError> {
        Err(EnforceError::Program {
            program: "/sbin/pfctl".into(),
            reason: "exit 1".into(),
        })
    }

    fn verify(&mut self, domains: &BTreeSet<Domain>) -> Result<bool, EnforceError> {
        self.apply(domains).map(|()| false)
    }

    fn clear(&mut self) -> Result<(), EnforceError> {
        self.apply(&BTreeSet::new())
    }
}

#[test]
fn enforcement_failures_never_end_a_block() {
    let rig = Rig::new();
    let mut service = rig.start_with(BrokenEnforcer);
    add(&mut service, "youtube.com");
    let started = status(service.handle(Command::StartBlock { secs: 600 }));
    assert!(started.block.is_some());
    rig.clock.advance(VERIFY_EVERY);
    service.tick();
    assert!(service.state().is_blocking());
    assert!(
        rig.blocked.blocks_name("youtube.com"),
        "the sinkhole still answers"
    );
}
