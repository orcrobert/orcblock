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
