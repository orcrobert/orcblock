use std::collections::BTreeSet;
use std::time::Duration;

use jiff::Timestamp;

use super::*;

#[test]
fn duration_bounds() {
    let cases = [
        ("zero", 0, Err(DurationError::TooShort)),
        ("59 s", 59, Err(DurationError::TooShort)),
        ("one minute", 60, Ok(60)),
        ("ten hours", 36_000, Ok(36_000)),
        ("seven days", 604_800, Ok(604_800)),
        (
            "seven days and a second",
            604_801,
            Err(DurationError::TooLong),
        ),
        ("huge", u64::MAX, Err(DurationError::TooLong)),
    ];
    for (name, secs, want) in cases {
        assert_eq!(
            BlockDuration::from_secs(secs).map(u64::from),
            want,
            "{name}"
        );
    }
    assert_eq!(
        BlockDuration::from_minutes(u64::MAX),
        Err(DurationError::TooLong)
    );
    assert_eq!(
        BlockDuration::from_minutes(25).map(BlockDuration::get),
        Ok(Duration::from_mins(25))
    );
}

fn session(secs: u64) -> Session {
    let domains = BTreeSet::from([Domain::parse("youtube.com").expect("valid")]);
    Session::new(
        domains,
        BlockDuration::from_secs(secs).expect("valid"),
        Timestamp::UNIX_EPOCH,
    )
}

#[test]
fn advance_counts_down_to_zero_and_stops() {
    let mut s = session(120);
    s.advance(Duration::from_secs(119));
    assert_eq!(s.remaining(), Duration::from_secs(1));
    assert!(!s.is_over());
    s.advance(Duration::from_secs(1));
    assert!(s.is_over());
    s.advance(Duration::from_secs(1000));
    assert_eq!(s.remaining(), Duration::ZERO);
}

#[test]
fn blocks_covers_subdomains() {
    let s = session(60);
    assert!(s.blocks(&Domain::parse("m.youtube.com").expect("valid")));
    assert!(!s.blocks(&Domain::parse("vimeo.com").expect("valid")));
}

#[test]
fn serde_uses_seconds_and_validates_duration() {
    let s = session(3600);
    let json = serde_json::to_value(&s).expect("serializes");
    assert_eq!(json["duration"], 3600);
    assert_eq!(json["remaining"], 3600);
    assert_eq!(
        serde_json::from_value::<Session>(json.clone()).expect("round trips"),
        s
    );

    let mut bad = json;
    bad["duration"] = 5.into();
    assert!(serde_json::from_value::<Session>(bad).is_err());
}
