use proptest::prelude::*;

use super::*;

#[test]
fn parses_list_names() {
    let forty = "a".repeat(40);
    let cases = [
        ("plain", "Social", "Social"),
        ("trimmed", "  Deep work\n", "Deep work"),
        ("unicode", "Café 🍵", "Café 🍵"),
        ("forty characters", forty.as_str(), forty.as_str()),
    ];
    for (case, input, want) in cases {
        let got = ListName::parse(input).unwrap_or_else(|e| panic!("{case}: {e}"));
        assert_eq!(got.as_str(), want, "{case}");
    }
}

#[test]
fn rejects_bad_list_names() {
    let long = "a".repeat(41);
    let cases = [
        ("empty", "", ListNameError::Empty),
        ("whitespace", " \t ", ListNameError::Empty),
        ("too long", long.as_str(), ListNameError::TooLong),
        ("control", "So\u{7}cial", ListNameError::Unreadable),
        ("bidi override", "So\u{202E}cial", ListNameError::Unreadable),
    ];
    for (case, input, want) in cases {
        assert_eq!(ListName::parse(input), Err(want), "{case}");
    }
}

#[test]
fn names_compare_ignoring_case() {
    let name = |s| ListName::parse(s).expect("valid");
    assert!(name("Social").same_as(&name("SOCIAL")));
    assert!(name("Über").same_as(&name("über")));
    assert!(!name("Social").same_as(&name("Socials")));
}

#[test]
fn stored_names_are_cleaned_not_refused() {
    let long = format!(r#""{}""#, "b".repeat(50));
    let forty = "b".repeat(40);
    let cases = [
        ("valid", r#""Social""#, Some("Social")),
        ("invisible dropped", "\"So\\u202Ecial\"", Some("Social")),
        ("too long cut", long.as_str(), Some(forty.as_str())),
        ("nothing left", "\"\\u200B\"", None),
    ];
    for (case, json, want) in cases {
        let got = serde_json::from_str::<ListName>(json).ok();
        assert_eq!(got.as_ref().map(ListName::as_str), want, "{case}");
    }
}

proptest! {
    #[test]
    fn parsed_names_round_trip(input in "\\PC{0,60}") {
        if let Ok(name) = ListName::parse(&input) {
            prop_assert_eq!(ListName::parse(name.as_str()), Ok(name.clone()));
            let json = serde_json::to_string(&name).expect("serializes");
            prop_assert_eq!(serde_json::from_str::<ListName>(&json).expect("parses"), name);
        }
    }

    #[test]
    fn parse_never_keeps_edge_whitespace(input in "\\PC{0,60}") {
        if let Ok(name) = ListName::parse(&input) {
            prop_assert_eq!(name.as_str().trim(), name.as_str());
            prop_assert!(!name.as_str().is_empty());
        }
    }
}
