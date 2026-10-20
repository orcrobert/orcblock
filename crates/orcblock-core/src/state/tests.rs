use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::date;

use super::*;

const NOW: Timestamp = Timestamp::UNIX_EPOCH;

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid domain")
}

fn october() -> Month {
    Month::new(2026, 10).expect("valid month")
}

fn minutes(n: u64) -> BlockDuration {
    BlockDuration::from_minutes(n).expect("valid duration")
}

fn listed(sites: &[&str]) -> State {
    let mut state = State::new(october());
    for site in sites {
        state.add_site(d(site), NOW).expect("adds");
    }
    state
}

#[test]
fn idle_list_blocks_nothing() {
    let state = listed(&["youtube.com"]);
    assert!(!state.is_blocking());
    assert!(state.blocked_domains().is_empty());
}

#[test]
fn start_requires_sites_and_no_running_block() {
    let mut empty = State::new(october());
    assert_eq!(
        empty.start_block(minutes(25), NOW).err(),
        Some(CommandError::EmptyBlocklist)
    );
    assert_eq!(empty, State::new(october()));

    let mut state = listed(&["youtube.com", "reddit.com"]);
    state.start_block(minutes(25), NOW).expect("starts");
    assert_eq!(
        state.blocked_domains(),
        BTreeSet::from([d("youtube.com"), d("reddit.com")])
    );
    let before = state.clone();
    assert_eq!(
        state.start_block(minutes(5), NOW).err(),
        Some(CommandError::AlreadyBlocking)
    );
    assert_eq!(state, before);
}

#[test]
fn add_reports_covering_site() {
    let mut state = listed(&["youtube.com"]);
    assert_eq!(
        state.add_site(d("m.youtube.com"), NOW),
        Err(CommandError::AlreadyListed {
            domain: d("m.youtube.com"),
            by: d("youtube.com")
        })
    );
}

#[test]
fn sites_added_during_a_block_join_it() {
    let mut state = listed(&["youtube.com"]);
    state.start_block(minutes(25), NOW).expect("starts");
    state.add_site(d("x.com"), NOW).expect("adds");
    assert!(state.blocked_domains().contains(&d("x.com")));
}

#[test]
fn sites_are_locked_during_a_block() {
    let mut state = listed(&["youtube.com", "x.com"]);
    assert_eq!(
        state.remove_site(&d("vimeo.com")),
        Err(CommandError::NotListed(d("vimeo.com")))
    );
    state.start_block(minutes(25), NOW).expect("starts");
    let before = state.clone();
    assert_eq!(
        state.remove_site(&d("x.com")),
        Err(CommandError::Locked(d("x.com")))
    );
    assert_eq!(state, before);

    state.advance(Duration::from_mins(25));
    state
        .remove_site(&d("x.com"))
        .expect("unlocked after the block");
    assert!(!state.blocklist().contains(&d("x.com")));
}

#[test]
fn block_ends_when_time_runs_out() {
    let mut state = listed(&["youtube.com"]);
    state.start_block(minutes(1), NOW).expect("starts");
    assert_eq!(state.advance(Duration::from_secs(59)), None);
    assert!(state.is_blocking());
    assert_eq!(
        state.advance(Duration::from_secs(1)),
        Some(SessionEnd::Finished)
    );
    assert!(!state.is_blocking());
    assert_eq!(state.advance(Duration::from_secs(1)), None);
}

#[test]
fn potions_end_a_block_five_times_a_month() {
    let mut state = listed(&["youtube.com"]);
    assert_eq!(
        state.drink_potion(october()),
        Err(CommandError::NotBlocking)
    );
    for left in (0..5).rev() {
        state.start_block(minutes(60), NOW).expect("starts");
        assert_eq!(
            state.drink_potion(october()),
            Ok(SessionEnd::Potion { left })
        );
        assert!(!state.is_blocking());
    }
    state.start_block(minutes(60), NOW).expect("starts");
    let before = state.clone();
    assert_eq!(
        state.drink_potion(october()),
        Err(CommandError::NoPotions {
            refill: date(2026, 11, 1)
        })
    );
    assert_eq!(state, before);

    let november = Month::new(2026, 11).expect("valid month");
    assert_eq!(
        state.drink_potion(november),
        Ok(SessionEnd::Potion { left: 4 })
    );
}

#[test]
fn serde_round_trips() {
    let mut state = listed(&["youtube.com", "reddit.com"]);
    state
        .add_app(id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    state.start_block(minutes(90), NOW).expect("starts");
    state.advance(Duration::from_secs(600));
    let json = serde_json::to_string(&state).expect("serializes");
    assert_eq!(serde_json::from_str::<State>(&json).expect("parses"), state);
}

#[test]
fn errors_read_well() {
    assert_eq!(
        CommandError::Locked(d("youtube.com")).to_string(),
        "The orc won't move: youtube.com stays blocked until the block ends"
    );
    assert_eq!(
        CommandError::NoPotions {
            refill: date(2026, 11, 1)
        }
        .to_string(),
        "No sleep potions left; the orc brews more on 2026-11-01"
    );
}

fn id(s: &str) -> BundleId {
    BundleId::parse(s).expect("valid bundle id")
}

fn name(s: &str) -> AppName {
    AppName::parse(s).expect("valid app name")
}

#[test]
fn apps_alone_can_start_a_block() {
    let mut state = State::new(october());
    state
        .add_app(id("com.hnc.Discord"), name("Discord"), NOW)
        .expect("adds");
    assert!(state.blocked_apps().is_empty(), "idle list blocks nothing");
    state.start_block(minutes(25), NOW).expect("starts");
    assert_eq!(
        state.blocked_apps(),
        BTreeSet::from([id("com.hnc.discord")])
    );
    assert!(state.blocked_domains().is_empty());
}

#[test]
fn sites_and_apps_share_one_block() {
    let mut state = listed(&["youtube.com"]);
    state
        .add_app(id("com.tinyspeck.slackmacgap"), name("Slack"), NOW)
        .expect("adds");
    state.start_block(minutes(25), NOW).expect("starts");
    assert_eq!(state.blocked_domains(), BTreeSet::from([d("youtube.com")]));
    assert_eq!(
        state.blocked_apps(),
        BTreeSet::from([id("com.tinyspeck.slackmacgap")])
    );

    assert_eq!(
        state.drink_potion(october()),
        Ok(SessionEnd::Potion { left: 4 })
    );
    assert!(state.blocked_apps().is_empty());
    assert!(state.blocked_domains().is_empty());
}

#[test]
fn add_app_refuses_duplicates_and_protected_apps() {
    let mut state = State::new(october());
    state
        .add_app(id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    let before = state.clone();
    let cases = [
        (
            "duplicate in another case",
            "com.hnc.Discord",
            "Discord 2",
            CommandError::AppAlreadyListed(name("Discord")),
        ),
        (
            "finder",
            "com.apple.finder",
            "Finder",
            CommandError::Unblockable(name("Finder")),
        ),
        (
            "orcblock",
            "com.orcrobert.orcblock",
            "OrcBlock",
            CommandError::Unblockable(name("OrcBlock")),
        ),
    ];
    for (case, bundle_id, app, want) in cases {
        assert_eq!(
            state.add_app(id(bundle_id), name(app), NOW),
            Err(want),
            "{case}"
        );
        assert_eq!(state, before, "{case} leaves the state unchanged");
    }
}

#[test]
fn apps_added_during_a_block_join_it() {
    let mut state = listed(&["youtube.com"]);
    state.start_block(minutes(25), NOW).expect("starts");
    state
        .add_app(id("com.valvesoftware.steam"), name("Steam"), NOW)
        .expect("adds");
    assert!(
        state
            .blocked_apps()
            .contains(&id("com.valvesoftware.steam"))
    );
}

#[test]
fn apps_are_locked_during_a_block() {
    let mut state = State::new(october());
    state
        .add_app(id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    assert_eq!(
        state.remove_app(&id("com.valvesoftware.steam")),
        Err(CommandError::AppNotListed(id("com.valvesoftware.steam")))
    );
    state.start_block(minutes(25), NOW).expect("starts");
    let before = state.clone();
    assert_eq!(
        state.remove_app(&id("com.hnc.discord")),
        Err(CommandError::AppLocked(name("Discord")))
    );
    assert_eq!(state, before);

    state.advance(Duration::from_mins(25));
    state
        .remove_app(&id("com.hnc.discord"))
        .expect("unlocked after the block");
    assert!(state.apps().is_empty());
}

#[test]
fn hand_edited_protected_apps_are_never_blocked() {
    let json = r#"{
        "blocklist": [],
        "apps": [{"bundle_id": "com.apple.finder", "name": "Finder", "added_at": "2026-10-01T09:00:00Z"}],
        "session": {"domains": [], "apps": ["com.apple.finder", "com.hnc.discord"], "started_at": "2026-10-20T10:00:00Z", "duration": 600, "remaining": 600},
        "potions": {"month": {"year": 2026, "month": 10}, "used": 0}
    }"#;
    let state: State = serde_json::from_str(json).expect("parses");
    assert_eq!(
        state.blocked_apps(),
        BTreeSet::from([id("com.hnc.discord")])
    );
}

#[test]
fn state_files_from_before_apps_still_load() {
    let json = include_str!("../../testdata/state-v1.json");
    let state: State = serde_json::from_str(json).expect("old state loads");
    assert!(state.apps().is_empty());
    assert!(state.blocked_apps().is_empty());
    assert_eq!(
        state.blocked_domains(),
        BTreeSet::from([d("youtube.com"), d("reddit.com")])
    );
    assert_eq!(
        state.session().map(Session::remaining),
        Some(Duration::from_mins(80))
    );
}

#[test]
fn app_errors_read_well() {
    assert_eq!(
        CommandError::AppLocked(name("Slack")).to_string(),
        "The orc won't move: Slack stays blocked until the block ends"
    );
    assert_eq!(
        CommandError::Unblockable(name("Finder")).to_string(),
        "The orc won't block Finder: your Mac or OrcBlock needs it"
    );
    assert_eq!(
        CommandError::EmptyBlocklist.to_string(),
        "Add a site or an app before starting a block"
    );
}
