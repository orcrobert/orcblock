use std::io::Cursor;

use proptest::prelude::*;

use super::*;
use crate::Command;

#[test]
fn encode_appends_one_newline() {
    let line = encode(&Request::new(Command::Status)).expect("encodes");
    assert_eq!(line, b"{\"v\":1,\"cmd\":\"status\"}\n");
}

#[test]
fn decode_request_checks_the_version_first() {
    let cases: [(&str, &[u8], ErrorKind); 3] = [
        (
            "newer client",
            br#"{"v":2,"cmd":"teleport"}"#,
            ErrorKind::Version,
        ),
        (
            "unknown command",
            br#"{"v":1,"cmd":"teleport"}"#,
            ErrorKind::BadRequest,
        ),
        ("garbage", b"not json", ErrorKind::BadRequest),
    ];
    for (name, line, want) in cases {
        let err = decode_request(line).expect_err(name);
        assert_eq!(err.kind, want, "{name}: {err}");
        assert!(
            matches!(Response::from(err), Response::Error { kind, .. } if kind == want),
            "{name}"
        );
    }

    assert_eq!(
        decode_request(b"{\"v\":1,\"cmd\":\"drink_potion\"}\n"),
        Ok(Request::new(Command::DrinkPotion))
    );
}

#[test]
fn read_line_splits_and_stops() {
    let mut reader = Cursor::new(b"one\ntwo\nthree".to_vec());
    let mut buf = Vec::new();
    let mut lines = Vec::new();
    while read_line(&mut reader, &mut buf).expect("reads") {
        lines.push(String::from_utf8(buf.clone()).expect("utf8"));
    }
    assert_eq!(lines, ["one\n", "two\n", "three"]);
}

#[test]
fn oversized_lines_are_refused() {
    let mut reader = Cursor::new(vec![b'a'; MAX_MESSAGE_LEN + 10]);
    let mut buf = Vec::new();
    assert!(matches!(
        read_line(&mut reader, &mut buf),
        Err(CodecError::TooLong)
    ));
    assert!(matches!(
        decode::<Request>(&vec![b' '; MAX_MESSAGE_LEN + 1]),
        Err(CodecError::TooLong)
    ));

    let huge = Command::AddSite {
        site: "a".repeat(MAX_MESSAGE_LEN),
    };
    assert!(matches!(
        encode(&Request::new(huge)),
        Err(CodecError::TooLong)
    ));
}

fn command() -> impl Strategy<Value = Command> {
    prop_oneof![
        Just(Command::Status),
        Just(Command::DrinkPotion),
        ".{0,200}".prop_map(|site| Command::AddSite { site }),
        ".{0,200}".prop_map(|site| Command::RemoveSite { site }),
        any::<u64>().prop_map(|secs| Command::StartBlock { secs }),
    ]
}

proptest! {
    #[test]
    fn requests_round_trip(command in command()) {
        let request = Request::new(command);
        let line = encode(&request).expect("encodes");
        prop_assert_eq!(line.split(|&b| b == b'\n').count(), 2, "one trailing newline");
        prop_assert_eq!(decode_request(&line), Ok(request));
    }

    #[test]
    fn decode_request_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_request(&bytes);
    }
}
