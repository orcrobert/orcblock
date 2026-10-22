use jiff::Timestamp;
use orcblock_core::{ListName, Month, State};
use orcblock_ipc::Mode;

use super::*;

fn status(names: &[&str]) -> Status {
    let month = Month::new(2026, 10).expect("valid month");
    let mut state = State::new(month);
    for name in names {
        state
            .create_list(ListName::parse(name).expect("valid name"))
            .expect("creates");
    }
    Status::new(&state, Timestamp::UNIX_EPOCH, month, Mode::DryRun, "0.1.0")
}

#[test]
fn finds_lists_by_name() {
    let one = status(&[]);
    let two = status(&["Work"]);
    let cases = [
        ("the only list by default", &one, None, Ok(ListId::new(1))),
        (
            "by name, any case",
            &two,
            Some(" work "),
            Ok(ListId::new(2)),
        ),
        (
            "several lists need a name",
            &two,
            None,
            Err("Say which list with --list; the lists are My list, Work".to_owned()),
        ),
        (
            "unknown name",
            &two,
            Some("Play"),
            Err("No list named Play; the lists are My list, Work".to_owned()),
        ),
    ];
    for (case, status, name, want) in cases {
        assert_eq!(list(status, name), want, "{case}");
    }
}

#[test]
fn a_block_covers_every_list_unless_named() {
    let two = status(&["Work"]);
    assert_eq!(lists(&two, &[]), Ok(vec![ListId::new(1), ListId::new(2)]));
    assert_eq!(lists(&two, &["work".to_owned()]), Ok(vec![ListId::new(2)]));
}
