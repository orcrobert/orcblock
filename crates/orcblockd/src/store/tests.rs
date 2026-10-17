use std::fs;
use std::os::unix::fs::PermissionsExt;

use orcblock_core::{Domain, Month};

use super::*;

fn state() -> State {
    let mut state = State::new(Month::new(2026, 10).expect("valid"));
    state
        .add_site(
            Domain::parse("youtube.com").expect("valid"),
            Timestamp::UNIX_EPOCH,
        )
        .expect("adds");
    state
}

#[test]
fn missing_file_loads_as_none() {
    let dir = tempfile::tempdir().expect("temp dir");
    assert_eq!(
        Store::new(dir.path().join("state.json"))
            .load()
            .expect("loads"),
        None
    );
}

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::new(dir.path().join("state.json"));
    let saved_at = Timestamp::from_second(1_800_000_000).expect("valid");
    store.save(&state(), saved_at).expect("saves");
    store.save(&state(), saved_at).expect("overwrites");

    assert_eq!(
        store.load().expect("loads"),
        Some(Saved {
            saved_at,
            state: state()
        })
    );
    let mode = fs::metadata(store.path())
        .expect("exists")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
    let names: Vec<_> = fs::read_dir(dir.path())
        .expect("lists")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(names, ["state.json"], "no temporary file left behind");
}

#[test]
fn corrupt_file_is_reported_and_can_be_moved_aside() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::new(dir.path().join("state.json"));
    fs::write(store.path(), "{not json").expect("writes");
    assert!(matches!(store.load(), Err(StoreError::Corrupt { .. })));

    let aside = store.quarantine().expect("moves");
    assert_eq!(aside, dir.path().join("state.json.corrupt"));
    assert!(aside.exists());
    assert_eq!(store.load().expect("loads"), None);
}

#[test]
fn save_into_a_missing_directory_fails_cleanly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::new(dir.path().join("nope").join("state.json"));
    assert!(matches!(
        store.save(&state(), Timestamp::UNIX_EPOCH),
        Err(StoreError::Write { .. })
    ));
}
