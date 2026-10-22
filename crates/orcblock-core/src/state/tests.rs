use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::date;

use super::*;

const NOW: Timestamp = Timestamp::UNIX_EPOCH;
const FIRST: ListId = ListId::new(1);

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid domain")
}

fn october() -> Month {
    Month::new(2026, 10).expect("valid month")
}

fn minutes(n: u64) -> BlockDuration {
    BlockDuration::from_minutes(n).expect("valid duration")
}

fn first(state: &State) -> &TargetList {
    state.list(FIRST).expect("the first list exists")
}

fn listed(sites: &[&str]) -> State {
    let mut state = State::new(october());
    for site in sites {
        state.add_site(FIRST, d(site), NOW).expect("adds");
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
        empty.start_block(&[FIRST], minutes(25), NOW).err(),
        Some(CommandError::EmptyBlocklist)
    );
    assert_eq!(empty, State::new(october()));

    let mut state = listed(&["youtube.com", "reddit.com"]);
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    assert_eq!(
        state.blocked_domains(),
        BTreeSet::from([d("youtube.com"), d("reddit.com")])
    );
    let before = state.clone();
    assert_eq!(
        state.start_block(&[FIRST], minutes(5), NOW).err(),
        Some(CommandError::AlreadyBlocking)
    );
    assert_eq!(state, before);
}

#[test]
fn add_reports_covering_site() {
    let mut state = listed(&["youtube.com"]);
    assert_eq!(
        state.add_site(FIRST, d("m.youtube.com"), NOW),
        Err(CommandError::AlreadyListed {
            domain: d("m.youtube.com"),
            by: d("youtube.com")
        })
    );
}

#[test]
fn sites_added_during_a_block_join_it() {
    let mut state = listed(&["youtube.com"]);
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    state.add_site(FIRST, d("x.com"), NOW).expect("adds");
    assert!(state.blocked_domains().contains(&d("x.com")));
}

#[test]
fn sites_are_locked_during_a_block() {
    let mut state = listed(&["youtube.com", "x.com"]);
    assert_eq!(
        state.remove_site(FIRST, &d("vimeo.com")),
        Err(CommandError::NotListed(d("vimeo.com")))
    );
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    let before = state.clone();
    assert_eq!(
        state.remove_site(FIRST, &d("x.com")),
        Err(CommandError::Locked(d("x.com")))
    );
    assert_eq!(state, before);

    state.advance(Duration::from_mins(25));
    state
        .remove_site(FIRST, &d("x.com"))
        .expect("unlocked after the block");
    assert!(!first(&state).sites().contains(&d("x.com")));
}

#[test]
fn block_ends_when_time_runs_out() {
    let mut state = listed(&["youtube.com"]);
    state
        .start_block(&[FIRST], minutes(1), NOW)
        .expect("starts");
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
        state
            .start_block(&[FIRST], minutes(60), NOW)
            .expect("starts");
        assert_eq!(
            state.drink_potion(october()),
            Ok(SessionEnd::Potion { left })
        );
        assert!(!state.is_blocking());
    }
    state
        .start_block(&[FIRST], minutes(60), NOW)
        .expect("starts");
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
        .add_app(FIRST, id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    state
        .start_block(&[FIRST], minutes(90), NOW)
        .expect("starts");
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
        .add_app(FIRST, id("com.hnc.Discord"), name("Discord"), NOW)
        .expect("adds");
    assert!(state.blocked_apps().is_empty(), "idle list blocks nothing");
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
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
        .add_app(FIRST, id("com.tinyspeck.slackmacgap"), name("Slack"), NOW)
        .expect("adds");
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
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
        .add_app(FIRST, id("com.hnc.discord"), name("Discord"), NOW)
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
            state.add_app(FIRST, id(bundle_id), name(app), NOW),
            Err(want),
            "{case}"
        );
        assert_eq!(state, before, "{case} leaves the state unchanged");
    }
}

#[test]
fn apps_added_during_a_block_join_it() {
    let mut state = listed(&["youtube.com"]);
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    state
        .add_app(FIRST, id("com.valvesoftware.steam"), name("Steam"), NOW)
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
        .add_app(FIRST, id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    assert_eq!(
        state.remove_app(FIRST, &id("com.valvesoftware.steam")),
        Err(CommandError::AppNotListed(id("com.valvesoftware.steam")))
    );
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    let before = state.clone();
    assert_eq!(
        state.remove_app(FIRST, &id("com.hnc.discord")),
        Err(CommandError::AppLocked(name("Discord")))
    );
    assert_eq!(state, before);

    state.advance(Duration::from_mins(25));
    state
        .remove_app(FIRST, &id("com.hnc.discord"))
        .expect("unlocked after the block");
    assert!(first(&state).apps().is_empty());
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
    assert!(first(&state).apps().is_empty());
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

fn list_name(s: &str) -> ListName {
    ListName::parse(s).expect("valid list name")
}

/// A state with "My list" (youtube.com) and "Work" (slack.com).
fn two_lists() -> (State, ListId) {
    let mut state = listed(&["youtube.com"]);
    let work = state.create_list(list_name("Work")).expect("creates");
    state.add_site(work, d("slack.com"), NOW).expect("adds");
    (state, work)
}

#[test]
fn a_new_state_has_one_empty_list() {
    let state = State::new(october());
    assert_eq!(state.lists().len(), 1);
    assert_eq!(first(&state).name().as_str(), "My list");
    assert!(first(&state).is_empty());
}

#[test]
fn create_list_refuses_taken_names_and_too_many_lists() {
    let (mut state, work) = two_lists();
    assert_ne!(work, FIRST);
    let before = state.clone();
    assert_eq!(
        state.create_list(list_name("WORK")),
        Err(CommandError::ListNameTaken(list_name("WORK")))
    );
    assert_eq!(state, before);

    for i in state.lists().len()..MAX_LISTS {
        state
            .create_list(list_name(&format!("List {i}")))
            .expect("creates");
    }
    let before = state.clone();
    assert_eq!(
        state.create_list(list_name("One more")),
        Err(CommandError::TooManyLists)
    );
    assert_eq!(state, before);
}

#[test]
fn list_ids_are_never_reused() {
    let (mut state, work) = two_lists();
    state.delete_list(work).expect("deletes");
    let again = state.create_list(list_name("Work")).expect("creates");
    assert!(again > work);
    assert_eq!(
        state.add_site(work, d("x.com"), NOW),
        Err(CommandError::ListNotFound(work))
    );
}

#[test]
fn rename_list_keeps_names_unique() {
    let (mut state, work) = two_lists();
    state
        .rename_list(work, list_name("work"))
        .expect("a list may change its own case");
    assert_eq!(
        state.rename_list(work, list_name("my LIST")),
        Err(CommandError::ListNameTaken(list_name("my LIST")))
    );
    assert_eq!(
        state.rename_list(ListId::new(99), list_name("Other")),
        Err(CommandError::ListNotFound(ListId::new(99)))
    );
}

#[test]
fn the_last_list_cannot_be_deleted() {
    let mut state = State::new(october());
    assert_eq!(state.delete_list(FIRST), Err(CommandError::LastList));
    assert_eq!(state.lists().len(), 1);
}

#[test]
fn a_block_covers_only_the_chosen_lists() {
    let (mut state, work) = two_lists();
    state
        .start_block(&[work], minutes(25), NOW)
        .expect("starts");
    assert_eq!(state.blocked_domains(), BTreeSet::from([d("slack.com")]));
    assert_eq!(
        state.session().map(Session::lists),
        Some(&BTreeSet::from([work]))
    );
}

#[test]
fn start_refuses_unknown_and_empty_lists() {
    let mut state = listed(&["youtube.com"]);
    let empty = state.create_list(list_name("Empty")).expect("creates");
    let before = state.clone();
    let cases = [
        ("no lists", vec![], CommandError::EmptyBlocklist),
        ("empty list", vec![empty], CommandError::EmptyBlocklist),
        (
            "unknown list",
            vec![FIRST, ListId::new(99)],
            CommandError::ListNotFound(ListId::new(99)),
        ),
    ];
    for (case, lists, want) in cases {
        assert_eq!(
            state.start_block(&lists, minutes(25), NOW).err(),
            Some(want),
            "{case}"
        );
        assert_eq!(state, before, "{case} leaves the state unchanged");
    }
}

#[test]
fn the_union_drops_subdomains_another_list_covers() {
    let (mut state, work) = two_lists();
    state.add_site(work, d("m.youtube.com"), NOW).expect("adds");
    state
        .start_block(&[FIRST, work], minutes(25), NOW)
        .expect("starts");
    assert_eq!(
        state.blocked_domains(),
        BTreeSet::from([d("youtube.com"), d("slack.com")])
    );
}

#[test]
fn only_lists_in_the_block_are_locked() {
    let (mut state, work) = two_lists();
    state
        .add_app(work, id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    state
        .add_app(FIRST, id("com.hnc.discord"), name("Discord"), NOW)
        .expect("adds");
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    let before = state.clone();
    let refusals = [
        (
            "site",
            state.clone().remove_site(FIRST, &d("youtube.com")),
            CommandError::Locked(d("youtube.com")),
        ),
        (
            "app",
            state.clone().remove_app(FIRST, &id("com.hnc.discord")),
            CommandError::AppLocked(name("Discord")),
        ),
        (
            "list",
            state.clone().delete_list(FIRST),
            CommandError::ListLocked(list_name("My list")),
        ),
    ];
    for (case, got, want) in refusals {
        assert_eq!(got, Err(want), "{case}");
    }
    assert_eq!(state, before);

    state
        .remove_site(work, &d("slack.com"))
        .expect("not in the block");
    state
        .remove_app(work, &id("com.hnc.discord"))
        .expect("not in the block");
    state
        .rename_list(FIRST, list_name("Renamed"))
        .expect("renaming blocks nothing");
    state.delete_list(work).expect("not in the block");
    assert_eq!(state.blocked_domains(), BTreeSet::from([d("youtube.com")]));
    assert_eq!(
        state.blocked_apps(),
        BTreeSet::from([id("com.hnc.discord")])
    );
}

#[test]
fn removing_from_another_list_never_unblocks() {
    let (mut state, work) = two_lists();
    state.add_site(work, d("youtube.com"), NOW).expect("adds");
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    state.remove_site(work, &d("youtube.com")).expect("removes");
    state.delete_list(work).expect("deletes");
    assert!(state.blocked_domains().contains(&d("youtube.com")));
}

#[test]
fn additions_join_the_block_only_through_its_lists() {
    let (mut state, work) = two_lists();
    state
        .start_block(&[FIRST], minutes(25), NOW)
        .expect("starts");
    state.add_site(work, d("x.com"), NOW).expect("adds");
    state
        .add_app(work, id("com.valvesoftware.steam"), name("Steam"), NOW)
        .expect("adds");
    assert!(!state.blocked_domains().contains(&d("x.com")));
    assert!(state.blocked_apps().is_empty());

    state.add_site(FIRST, d("x.com"), NOW).expect("adds");
    assert!(state.blocked_domains().contains(&d("x.com")));
}

#[test]
fn state_files_from_before_lists_load_into_one_list() {
    let json = include_str!("../../testdata/state-v2.json");
    let mut state: State = serde_json::from_str(json).expect("0.2.0 state loads");
    assert_eq!(state.lists().len(), 1);
    assert_eq!(first(&state).name().as_str(), "My list");
    assert!(first(&state).sites().contains(&d("youtube.com")));
    assert!(first(&state).apps().get(&id("com.hnc.discord")).is_some());

    let session = state.session().expect("the block survives");
    assert_eq!(session.lists(), &BTreeSet::from([FIRST]));
    assert_eq!(session.remaining(), Duration::from_mins(20));
    assert_eq!(state.blocked_domains(), BTreeSet::from([d("youtube.com")]));
    assert_eq!(
        state.blocked_apps(),
        BTreeSet::from([id("com.hnc.discord")])
    );
    assert_eq!(
        state.remove_site(FIRST, &d("youtube.com")),
        Err(CommandError::Locked(d("youtube.com")))
    );

    let next = state.create_list(list_name("Work")).expect("creates");
    assert_eq!(next, ListId::new(2));
}

#[test]
fn stored_lists_keep_ids_ahead_of_every_list() {
    let json = r#"{
        "lists": [{"id": 7, "name": "Seven"}],
        "next_list_id": 3,
        "session": null,
        "potions": {"month": {"year": 2026, "month": 10}, "used": 0}
    }"#;
    let mut state: State = serde_json::from_str(json).expect("parses");
    let next = state.create_list(list_name("Next")).expect("creates");
    assert_eq!(next, ListId::new(8));

    let twice = r#"{
        "lists": [{"id": 1, "name": "A"}, {"id": 1, "name": "B"}],
        "session": null,
        "potions": {"month": {"year": 2026, "month": 10}, "used": 0}
    }"#;
    assert!(serde_json::from_str::<State>(twice).is_err());
}

#[test]
fn list_errors_read_well() {
    assert_eq!(
        CommandError::ListNameTaken(list_name("Social")).to_string(),
        "There is already a list named Social"
    );
    assert_eq!(
        CommandError::ListLocked(list_name("Social")).to_string(),
        "The orc won't move: Social is part of the running block"
    );
}
