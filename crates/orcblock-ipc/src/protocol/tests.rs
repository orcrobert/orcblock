use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::date;
use orcblock_core::{BlockDuration, ListId};
use serde_json::json;

use super::*;

const FIRST: ListId = ListId::new(1);

fn october() -> Month {
    Month::new(2026, 10).expect("valid month")
}

#[test]
fn requests_have_a_stable_wire_format() {
    let cases = [
        (Command::Status, json!({"v": 2, "cmd": "status"})),
        (
            Command::AddSite {
                site: "https://www.youtube.com/watch".into(),
            },
            json!({"v": 2, "cmd": "add_site", "site": "https://www.youtube.com/watch"}),
        ),
        (
            Command::RemoveSite {
                site: "youtube.com".into(),
            },
            json!({"v": 2, "cmd": "remove_site", "site": "youtube.com"}),
        ),
        (
            Command::StartBlock { secs: 1500 },
            json!({"v": 2, "cmd": "start_block", "secs": 1500}),
        ),
        (Command::DrinkPotion, json!({"v": 2, "cmd": "drink_potion"})),
        (
            Command::AddApp {
                bundle_id: "com.tinyspeck.slackmacgap".into(),
                name: "Slack".into(),
            },
            json!({"v": 2, "cmd": "add_app", "bundle_id": "com.tinyspeck.slackmacgap", "name": "Slack"}),
        ),
        (
            Command::RemoveApp {
                bundle_id: "com.tinyspeck.slackmacgap".into(),
            },
            json!({"v": 2, "cmd": "remove_app", "bundle_id": "com.tinyspeck.slackmacgap"}),
        ),
    ];
    for (command, want) in cases {
        let request = Request::new(command);
        assert_eq!(serde_json::to_value(&request).expect("serializes"), want);
        assert_eq!(
            serde_json::from_value::<Request>(want).expect("parses"),
            request
        );
    }
}

#[test]
fn command_errors_map_to_kinds() {
    let locked = CommandError::Locked(Domain::parse("x.com").expect("valid"));
    assert_eq!(
        Response::from(&locked),
        Response::error(
            ErrorKind::Locked,
            "The orc won't move: x.com stays blocked until the block ends"
        )
    );
    assert_eq!(
        serde_json::to_value(Response::from(&CommandError::NotBlocking)).expect("serializes"),
        json!({"result": "error", "kind": "not_blocking", "message": "No block is running"})
    );
}

#[test]
fn status_reflects_state() {
    let start = Timestamp::from_second(1_800_000_000).expect("valid");
    let mut state = State::new(october());
    state
        .add_site(FIRST, Domain::parse("bücher.de").expect("valid"), start)
        .expect("adds");
    let idle = Status::new(&state, start, october(), Mode::DryRun, "0.1.0");
    assert_eq!(idle.sites[0].domain.as_ascii(), "xn--bcher-kva.de");
    assert_eq!(idle.sites[0].display, "bücher.de");
    assert_eq!(idle.block, None);
    assert_eq!(
        idle.potions,
        Potions {
            available: 5,
            per_month: 5,
            refill_on: date(2026, 11, 1)
        }
    );

    state
        .start_block(
            &[FIRST],
            BlockDuration::from_minutes(30).expect("valid"),
            start,
        )
        .expect("starts");
    state.advance(Duration::from_mins(10));
    let now = start
        .checked_add(Duration::from_mins(10))
        .expect("in range");
    let block = Status::new(&state, now, october(), Mode::Live, "0.1.0")
        .block
        .expect("blocking");
    assert_eq!(block.duration_secs, 1800);
    assert_eq!(block.remaining_secs, 1200);
    assert_eq!(block.started_at, start);
    assert_eq!(
        block.ends_at,
        start
            .checked_add(Duration::from_mins(30))
            .expect("in range")
    );
}

#[test]
fn status_lists_apps_and_their_block() {
    let start = Timestamp::from_second(1_800_000_000).expect("valid");
    let mut state = State::new(october());
    state
        .add_app(
            FIRST,
            BundleId::parse("com.hnc.Discord").expect("valid"),
            AppName::parse("Discord").expect("valid"),
            start,
        )
        .expect("adds");
    state
        .start_block(
            &[FIRST],
            BlockDuration::from_minutes(30).expect("valid"),
            start,
        )
        .expect("starts");
    let status = Status::new(&state, start, october(), Mode::Live, "0.1.0");
    assert_eq!(
        serde_json::to_value(&status.apps).expect("serializes"),
        json!([{"bundle_id": "com.hnc.discord", "name": "Discord", "added_at": "2027-01-15T08:00:00Z"}])
    );
    let block = status.block.expect("blocking");
    assert_eq!(
        block.apps,
        [BundleId::parse("com.hnc.discord").expect("valid")]
    );
    assert_eq!(block.domains, Vec::<Domain>::new());
    assert_eq!(status.blocked_launches, Vec::new());
}

#[test]
fn app_errors_map_to_kinds() {
    let discord = AppName::parse("Discord").expect("valid");
    let cases = [
        (CommandError::AppLocked(discord.clone()), ErrorKind::Locked),
        (
            CommandError::AppAlreadyListed(discord.clone()),
            ErrorKind::AlreadyListed,
        ),
        (
            CommandError::AppNotListed(BundleId::parse("com.hnc.discord").expect("valid")),
            ErrorKind::NotListed,
        ),
        (CommandError::Unblockable(discord), ErrorKind::Unblockable),
    ];
    for (err, want) in cases {
        assert!(
            matches!(Response::from(&err), Response::Error { kind, .. } if kind == want),
            "{err:?}"
        );
    }
    assert_eq!(
        serde_json::to_value(ErrorKind::InvalidApp).expect("serializes"),
        json!("invalid_app")
    );
}

#[test]
fn status_round_trips() {
    let state = State::new(october());
    let status = Status::new(
        &state,
        Timestamp::UNIX_EPOCH,
        october(),
        Mode::Live,
        "0.1.0",
    );
    let response = Response::Ok { status };
    let json = serde_json::to_string(&response).expect("serializes");
    assert_eq!(
        serde_json::from_str::<Response>(&json).expect("parses"),
        response
    );
}
