use super::*;

fn launch(id: u64) -> BlockedLaunch {
    launch_at(id, "2026-10-20T12:41:00Z")
}

fn launch_at(id: u64, at: &str) -> BlockedLaunch {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "bundle_id": "com.tinyspeck.slackmacgap",
        "name": "Slack",
        "at": at,
    }))
    .expect("a launch as the daemon sends it")
}

fn shown(tracker: &mut ShieldTracker, ids: &[u64]) -> Option<u64> {
    let launches: Vec<BlockedLaunch> = ids.iter().copied().map(launch).collect();
    tracker.next(&launches).map(|launch| launch.id)
}

#[test]
fn shows_each_new_launch_once() {
    let mut tracker = ShieldTracker::new(false);
    assert_eq!(
        shown(&mut tracker, &[1, 2]),
        None,
        "from before the app started"
    );
    assert_eq!(shown(&mut tracker, &[1, 2]), None);
    assert_eq!(shown(&mut tracker, &[1, 2, 3]), Some(3));
    assert_eq!(shown(&mut tracker, &[1, 2, 3]), None, "shown once");
    assert_eq!(
        shown(&mut tracker, &[2, 3, 4, 5]),
        Some(5),
        "the newest of several"
    );
}

#[test]
fn shows_the_first_launch_after_an_empty_start() {
    let mut tracker = ShieldTracker::new(false);
    assert_eq!(shown(&mut tracker, &[]), None);
    assert_eq!(shown(&mut tracker, &[1]), Some(1));
}

#[test]
fn shows_the_launch_it_was_woken_for() {
    let mut tracker = ShieldTracker::new(true);
    assert_eq!(shown(&mut tracker, &[1, 2]), Some(2));
    assert_eq!(shown(&mut tracker, &[1, 2]), None);

    let mut early = ShieldTracker::new(true);
    assert_eq!(shown(&mut early, &[]), None);
    assert_eq!(
        shown(&mut early, &[]),
        None,
        "only the first status counts as woken"
    );
}

#[test]
fn a_restarted_daemon_counts_again() {
    let mut tracker = ShieldTracker::new(false);
    assert_eq!(shown(&mut tracker, &[6, 7]), None);
    assert_eq!(shown(&mut tracker, &[]), None, "restarted, nothing yet");
    assert_eq!(shown(&mut tracker, &[1]), Some(1));
}

#[test]
fn a_restarted_daemon_reusing_the_last_id_is_still_new() {
    let mut tracker = ShieldTracker::new(false);
    assert_eq!(shown(&mut tracker, &[]), None);
    assert_eq!(shown(&mut tracker, &[1]), Some(1));
    let again = [launch_at(1, "2026-10-20T13:05:00Z")];
    assert_eq!(tracker.next(&again).map(|launch| launch.id), Some(1));
}
