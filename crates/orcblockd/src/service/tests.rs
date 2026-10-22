use std::collections::BTreeSet;
use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use orcblock_core::{ListId, Month};

use super::*;
use crate::DryRunEnforcer;
use crate::Stop;
use crate::apps::fake::{FakePlists, FakeTerminator};
use crate::clock::fake::FakeClock;
use crate::enforce::runner::fake::FakeRunner;
use crate::enforce::{DryRunCall, EnforceError, Output};

const FIRST: ListId = ListId::new(1);

const START: i64 = 1_800_000_000;

fn october() -> Month {
    Month::new(2026, 10).expect("valid")
}

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid")
}

const DISCORD: &str = "  700   501 /Applications/Discord.app/Contents/MacOS/Discord\n";

struct Rig {
    dir: tempfile::TempDir,
    clock: FakeClock,
    blocked: Blocked,
    processes: Arc<Mutex<String>>,
    runner: FakeRunner,
    terminator: FakeTerminator,
}

impl Rig {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temp dir"),
            clock: FakeClock::new(Timestamp::from_second(START).expect("valid"), october()),
            blocked: Blocked::default(),
            processes: Arc::default(),
            runner: FakeRunner::new(|_| Err("unused".into())),
            terminator: FakeTerminator::default(),
        }
        .with_process_table()
    }

    /// Answers `ps` with [`Rig::run`]'s table and knows Discord's bundle ID.
    fn with_process_table(mut self) -> Self {
        let processes = Arc::clone(&self.processes);
        self.runner = FakeRunner::by_call(move |call| {
            let line = call.line();
            let stdout = if line.contains("/bin/ps ") {
                processes
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
            } else if call.stdin.as_deref() == Some("/Applications/Discord.app") {
                "com.hnc.Discord\n".to_owned()
            } else {
                return Err(format!("unexpected: {line}"));
            };
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        });
        self
    }

    /// Sets what `ps` reports from now on.
    fn run(&self, table: &str) {
        *self
            .processes
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = table.to_owned();
    }

    fn ps_calls(&self) -> usize {
        self.runner
            .lines()
            .iter()
            .filter(|line| line.contains("/bin/ps "))
            .count()
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
            AppGuard::new(self.runner.clone(), self.terminator.clone()).reading(FakePlists),
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
    status(service.handle(Command::AddSite {
        list: FIRST,
        site: site.into(),
    }))
}

#[test]
fn adds_cleaned_sites_and_reports_bad_ones() {
    let rig = Rig::new();
    let mut service = rig.start();
    let after = add(&mut service, "https://www.YouTube.com/watch?v=x");
    assert_eq!(after.lists[0].sites[0].domain, d("youtube.com"));

    let (kind, message) = refusal(service.handle(Command::AddSite {
        list: FIRST,
        site: "localhost".into(),
    }));
    assert_eq!(kind, ErrorKind::InvalidSite);
    assert_eq!(
        message,
        "That is not a full site name (localhost); try something like localhost.com"
    );

    let (kind, _) = refusal(service.handle(Command::StartBlock {
        secs: 5,
        lists: vec![FIRST],
    }));
    assert_eq!(kind, ErrorKind::InvalidDuration);
}

#[test]
fn adds_apps_and_reports_bad_ones() {
    let rig = Rig::new();
    let mut service = rig.start();
    let after = status(service.handle(Command::AddApp {
        list: FIRST,
        bundle_id: "com.tinyspeck.SlackMacGap".into(),
        name: " Slack ".into(),
    }));
    assert_eq!(
        after.lists[0].apps[0].bundle_id.as_str(),
        "com.tinyspeck.slackmacgap"
    );
    assert_eq!(after.lists[0].apps[0].name.as_str(), "Slack");

    let cases = [
        (
            "path as id",
            "../../etc/passwd",
            "Evil",
            ErrorKind::InvalidApp,
        ),
        (
            "control in name",
            "com.hnc.discord",
            "Dis\u{7}cord",
            ErrorKind::InvalidApp,
        ),
        (
            "finder",
            "com.apple.finder",
            "Finder",
            ErrorKind::Unblockable,
        ),
        (
            "duplicate",
            "com.tinyspeck.slackmacgap",
            "Slack",
            ErrorKind::AlreadyListed,
        ),
    ];
    for (name, bundle_id, app, want) in cases {
        let (kind, _) = refusal(service.handle(Command::AddApp {
            list: FIRST,
            bundle_id: bundle_id.into(),
            name: app.into(),
        }));
        assert_eq!(kind, want, "{name}");
    }
    assert_eq!(service.state().lists()[0].apps().entries().len(), 1);

    let (kind, _) = refusal(service.handle(Command::RemoveApp {
        list: FIRST,
        bundle_id: "slack".into(),
    }));
    assert_eq!(kind, ErrorKind::InvalidApp);
    let after = status(service.handle(Command::RemoveApp {
        list: FIRST,
        bundle_id: "com.tinyspeck.slackmacgap".into(),
    }));
    assert_eq!(after.lists[0].apps, Vec::new());
}

#[test]
fn a_block_is_enforced_counted_down_and_cleared() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    assert!(!rig.blocked.is_active(), "listing alone blocks nothing");

    let started = status(service.handle(Command::StartBlock {
        secs: 120,
        lists: vec![FIRST],
    }));
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
    service.handle(Command::StartBlock {
        secs: 3600,
        lists: vec![FIRST],
    });

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
    service.handle(Command::StartBlock {
        secs: 60,
        lists: vec![FIRST],
    });
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
    service.handle(Command::StartBlock {
        secs: 600,
        lists: vec![FIRST],
    });
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
        service.handle(Command::StartBlock {
            secs: 3600,
            lists: vec![FIRST],
        });
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
    assert!(service.state().lists()[0].sites().is_empty());
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
        service.handle(Command::StartBlock {
            secs: 3600,
            lists: vec![FIRST],
        });
        let after = status(service.handle(Command::DrinkPotion));
        assert_eq!(after.potions.available, left);
        assert_eq!(after.block, None);
        assert!(!rig.blocked.is_active());
    }
    service.handle(Command::StartBlock {
        secs: 3600,
        lists: vec![FIRST],
    });
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
        list: FIRST,
        site: "youtube.com".into(),
    }));
    assert_eq!(kind, ErrorKind::Internal);
    assert!(service.state().lists()[0].sites().is_empty());
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
    let started = status(service.handle(Command::StartBlock {
        secs: 600,
        lists: vec![FIRST],
    }));
    assert!(started.block.is_some());
    rig.clock.advance(VERIFY_EVERY);
    service.tick();
    assert!(service.state().is_blocking());
    assert!(
        rig.blocked.blocks_name("youtube.com"),
        "the sinkhole still answers"
    );
}

fn add_discord(service: &mut Service<FakeClock, impl Enforcer>) {
    status(service.handle(Command::AddApp {
        list: FIRST,
        bundle_id: "com.hnc.discord".into(),
        name: "Discord".into(),
    }));
}

#[test]
fn an_app_block_closes_apps_and_reports_new_launches() {
    let rig = Rig::new();
    let mut service = rig.start();
    add_discord(&mut service);
    rig.run(DISCORD);
    assert_eq!(rig.ps_calls(), 0, "listing alone closes nothing");

    let started = status(service.handle(Command::StartBlock {
        secs: 120,
        lists: vec![FIRST],
    }));
    assert_eq!(rig.terminator.take(), [(700, Stop::Term)]);
    assert_eq!(
        started.blocked_launches,
        Vec::new(),
        "open at start: the warning covered it"
    );

    rig.run("");
    rig.clock.advance(Duration::from_secs(1));
    service.tick();
    rig.run("  701   501 /Applications/Discord.app/Contents/MacOS/Discord\n");
    rig.clock.advance(Duration::from_secs(1));
    service.tick();
    assert_eq!(rig.terminator.take(), [(701, Stop::Term)]);
    let launches = service.status().blocked_launches;
    assert_eq!(launches.len(), 1);
    assert_eq!(launches[0].name.as_str(), "Discord");

    rig.clock.advance(Duration::from_secs(118));
    service.tick();
    assert!(!service.state().is_blocking());
    let calls = rig.ps_calls();
    rig.clock.advance(Duration::from_secs(5));
    service.tick();
    assert_eq!(rig.ps_calls(), calls, "no polling once the block is over");
    assert_eq!(
        service.status().blocked_launches.len(),
        1,
        "kept for the shield"
    );
}

#[test]
fn a_launch_caught_by_an_unrelated_change_still_gets_a_shield() {
    let rig = Rig::new();
    let mut service = rig.start();
    add_discord(&mut service);
    status(service.handle(Command::StartBlock {
        secs: 120,
        lists: vec![FIRST],
    }));

    rig.run(DISCORD);
    let after = add(&mut service, "example.com");
    assert_eq!(rig.terminator.take(), [(700, Stop::Term)]);
    assert_eq!(after.blocked_launches.len(), 1);
}

#[test]
fn sweeps_at_most_once_a_second() {
    let rig = Rig::new();
    let mut service = rig.start();
    add_discord(&mut service);
    status(service.handle(Command::StartBlock {
        secs: 120,
        lists: vec![FIRST],
    }));
    let calls = rig.ps_calls();
    for _ in 0..5 {
        status(service.handle(Command::Status));
    }
    assert_eq!(rig.ps_calls(), calls, "status requests do not poll");
    rig.clock.advance(Duration::from_millis(1500));
    service.tick();
    assert_eq!(rig.ps_calls(), calls + 1);
}

#[test]
fn a_restart_mid_block_closes_apps_again() {
    let rig = Rig::new();
    {
        let mut service = rig.start();
        add_discord(&mut service);
        status(service.handle(Command::StartBlock {
            secs: 600,
            lists: vec![FIRST],
        }));
        service.save().expect("saves");
    }
    rig.terminator.take();
    rig.run(DISCORD);
    let service = rig.start();
    assert!(service.state().is_blocking());
    assert_eq!(rig.terminator.take(), [(700, Stop::Term)]);
}

#[test]
fn a_sites_only_block_never_lists_processes() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    status(service.handle(Command::StartBlock {
        secs: 120,
        lists: vec![FIRST],
    }));
    rig.clock.advance(Duration::from_secs(10));
    service.tick();
    assert_eq!(rig.ps_calls(), 0);
}

fn create(service: &mut Service<FakeClock, impl Enforcer>, name: &str) -> ListId {
    let after = status(service.handle(Command::CreateList { name: name.into() }));
    let list = after.lists.last().expect("created");
    assert_eq!(list.name.as_str(), name.trim());
    list.id
}

#[test]
fn manages_lists_and_reports_bad_names() {
    let rig = Rig::new();
    let mut service = rig.start();
    let work = create(&mut service, " Work ");

    let cases = [
        ("empty name", "  ", ErrorKind::InvalidList),
        ("taken name", "WORK", ErrorKind::AlreadyListed),
    ];
    for (case, name, want) in cases {
        let (kind, _) = refusal(service.handle(Command::CreateList { name: name.into() }));
        assert_eq!(kind, want, "{case}");
    }

    let after = status(service.handle(Command::RenameList {
        list: work,
        name: "Deep work".into(),
    }));
    assert_eq!(after.lists[1].name.as_str(), "Deep work");
    let after = status(service.handle(Command::DeleteList { list: work }));
    assert_eq!(after.lists.len(), 1);

    let (kind, _) = refusal(service.handle(Command::DeleteList { list: work }));
    assert_eq!(kind, ErrorKind::ListNotFound);
    let (kind, _) = refusal(service.handle(Command::DeleteList { list: FIRST }));
    assert_eq!(kind, ErrorKind::Locked, "the last list stays");
}

#[test]
fn a_block_enforces_only_its_lists() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    let work = create(&mut service, "Work");
    status(service.handle(Command::AddSite {
        list: work,
        site: "slack.com".into(),
    }));

    let started = status(service.handle(Command::StartBlock {
        secs: 600,
        lists: vec![work],
    }));
    assert_eq!(started.block.map(|block| block.lists), Some(vec![work]));
    assert!(rig.blocked.blocks_name("slack.com"));
    assert!(!rig.blocked.blocks_name("youtube.com"));

    add(&mut service, "x.com");
    assert!(!rig.blocked.blocks_name("x.com"), "not a list in the block");
    status(service.handle(Command::AddSite {
        list: work,
        site: "x.com".into(),
    }));
    assert!(rig.blocked.blocks_name("x.com"), "joins the block at once");

    let (kind, _) = refusal(service.handle(Command::DeleteList { list: work }));
    assert_eq!(kind, ErrorKind::Locked);
    status(service.handle(Command::RemoveSite {
        list: FIRST,
        site: "x.com".into(),
    }));
    assert!(rig.blocked.blocks_name("x.com"), "still on a blocked list");
}

#[test]
fn a_block_over_an_unknown_list_is_refused() {
    let rig = Rig::new();
    let mut service = rig.start();
    add(&mut service, "youtube.com");
    let (kind, _) = refusal(service.handle(Command::StartBlock {
        secs: 600,
        lists: vec![ListId::new(9)],
    }));
    assert_eq!(kind, ErrorKind::ListNotFound);
    assert!(!rig.blocked.is_active());
}
