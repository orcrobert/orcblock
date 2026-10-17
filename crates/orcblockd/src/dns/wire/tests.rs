use proptest::prelude::*;

use super::*;

/// A standard query for `name` with recursion desired.
pub(crate) fn query(id: u16, name: &str, kind: u16) -> Vec<u8> {
    let mut packet = id.to_be_bytes().to_vec();
    packet.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in name.split('.').filter(|l| !l.is_empty()) {
        packet.push(u8::try_from(label.len()).expect("short label"));
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&kind.to_be_bytes());
    packet.extend_from_slice(&CLASS_IN.to_be_bytes());
    packet
}

#[test]
fn parses_a_query() {
    let parsed = Query::parse(&query(7, "www.YouTube.com.", 28)).expect("parses");
    assert_eq!(parsed.name(), "www.YouTube.com");
    assert_eq!(parsed.record_type(), RecordType::Aaaa);
}

#[test]
fn sinkhole_reply_for_a() {
    let packet = query(0xBEEF, "youtube.com", 1);
    let reply = Query::parse(&packet).expect("parses").sinkhole_reply(10);
    assert_eq!(&reply[..2], &[0xBE, 0xEF], "id echoed");
    assert_eq!(reply[2], 0x85, "QR, AA and RD");
    assert_eq!(reply[3], 0, "NOERROR");
    assert_eq!(
        &reply[4..12],
        &[0, 1, 0, 1, 0, 0, 0, 0],
        "one question, one answer"
    );
    assert_eq!(&reply[12..packet.len()], &packet[12..], "question echoed");
    let answer = &reply[packet.len()..];
    assert_eq!(
        answer,
        &[0xC0, 0x0C, 0, 1, 0, 1, 0, 0, 0, 10, 0, 4, 0, 0, 0, 0]
    );
}

#[test]
fn sinkhole_reply_for_aaaa_and_others() {
    let aaaa = Query::parse(&query(1, "youtube.com", 28))
        .expect("parses")
        .sinkhole_reply(10);
    assert_eq!(aaaa.len(), query(1, "youtube.com", 28).len() + 12 + 16);
    assert!(aaaa.ends_with(&[0; 16]));

    let https = Query::parse(&query(1, "youtube.com", 65))
        .expect("parses")
        .sinkhole_reply(10);
    assert_eq!(&https[6..8], &[0, 0], "no answer records");
    assert_eq!(https[3], 0, "NOERROR");
}

#[test]
fn error_replies() {
    let refused = Query::parse(&query(1, "example.org", 1))
        .expect("parses")
        .reply(Rcode::Refused);
    assert_eq!(refused[3], 5);

    assert_eq!(Query::parse(&[0; 5]), Err(ParseError::Unanswerable));
    let mut response = query(1, "a.com", 1);
    response[2] |= 0x80;
    assert_eq!(
        Query::parse(&response),
        Err(ParseError::Unanswerable),
        "responses are ignored"
    );

    let mut notify = query(1, "a.com", 1);
    notify[2] |= 4 << 3;
    assert!(matches!(
        Query::parse(&notify),
        Err(ParseError::NotImplemented { .. })
    ));

    let mut two = query(1, "a.com", 1);
    two[5] = 2;
    assert!(matches!(
        Query::parse(&two),
        Err(ParseError::Malformed { .. })
    ));

    let mut compressed = query(1, "a.com", 1);
    compressed[12] = 0xC0;
    assert!(matches!(
        Query::parse(&compressed),
        Err(ParseError::Malformed { .. })
    ));

    let header: [u8; 12] = query(9, "a.com", 1)[..12].try_into().expect("12 bytes");
    let formerr = error_reply(&header, Rcode::FormErr);
    assert_eq!(formerr, [0, 9, 0x85, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn names_longer_than_255_are_malformed() {
    let long = ["a".repeat(63).as_str(); 5].join(".");
    assert!(matches!(
        Query::parse(&query(1, &long, 1)),
        Err(ParseError::Malformed { .. })
    ));
}

proptest! {
    #[test]
    fn parse_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..600)) {
        if let Ok(parsed) = Query::parse(&bytes) {
            let reply = parsed.sinkhole_reply(10);
            prop_assert_eq!(&reply[..2], &bytes[..2]);
        }
    }

    #[test]
    fn well_formed_queries_round_trip(
        labels in proptest::collection::vec("[a-z0-9-]{1,20}", 1..6),
        id in any::<u16>(),
    ) {
        let name = labels.join(".");
        let parsed = Query::parse(&query(id, &name, 1)).expect("parses");
        prop_assert_eq!(parsed.name(), name.as_str());
    }
}
