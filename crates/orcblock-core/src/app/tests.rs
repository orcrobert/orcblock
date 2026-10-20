use proptest::prelude::*;

use super::*;

#[test]
fn parses_bundle_ids() {
    let cases = [
        (
            "lowercase",
            "com.tinyspeck.slackmacgap",
            "com.tinyspeck.slackmacgap",
        ),
        ("mixed case folded", "com.apple.Safari", "com.apple.safari"),
        ("two labels", "us.zoom", "us.zoom"),
        (
            "hyphen and digits",
            "com.valvesoftware.steam-2",
            "com.valvesoftware.steam-2",
        ),
        (
            "surrounding space",
            "  com.hnc.Discord\n",
            "com.hnc.discord",
        ),
    ];
    for (name, input, want) in cases {
        let got = BundleId::parse(input).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(got.as_str(), want, "{name}");
    }
}

#[test]
fn rejects_bad_bundle_ids() {
    let long = format!("com.{}", "a".repeat(252));
    let cases = [
        ("empty", "", AppError::Empty),
        ("whitespace", "  ", AppError::Empty),
        ("single label", "slack", AppError::InvalidId("slack".into())),
        (
            "empty label",
            "com..slack",
            AppError::InvalidId("com..slack".into()),
        ),
        (
            "trailing dot",
            "com.slack.",
            AppError::InvalidId("com.slack.".into()),
        ),
        (
            "path",
            "../etc/passwd",
            AppError::InvalidId("../etc/passwd".into()),
        ),
        (
            "slash",
            "com/slack",
            AppError::InvalidId("com/slack".into()),
        ),
        (
            "underscore",
            "com.my_app",
            AppError::InvalidId("com.my_app".into()),
        ),
        (
            "space inside",
            "com.my app",
            AppError::InvalidId("com.my app".into()),
        ),
        (
            "unicode",
            "com.café",
            AppError::InvalidId("com.café".into()),
        ),
        (
            "too long, shortened in the error",
            long.as_str(),
            AppError::InvalidId(format!("{}…", &long[..64])),
        ),
    ];
    for (name, input, want) in cases {
        assert_eq!(BundleId::parse(input), Err(want), "{name}");
    }
    let longest = format!("com.{}", "a".repeat(251));
    assert!(BundleId::parse(&longest).is_ok(), "255 characters fit");
}

#[test]
fn protects_macos_and_orcblock() {
    let cases = [
        ("finder", "com.apple.finder", true),
        ("finder any case", "com.apple.Finder", true),
        ("dock", "com.apple.dock", true),
        ("login window", "com.apple.loginwindow", true),
        ("menu bar", "com.apple.systemuiserver", true),
        ("control centre", "com.apple.controlcenter", true),
        (
            "notification centre",
            "com.apple.notificationcenterui",
            true,
        ),
        ("orcblock", "com.orcrobert.orcblock", true),
        ("orcblock helper", "com.orcrobert.orcblockd", true),
        ("own prefix", "com.orcrobert", true),
        ("lookalike prefix", "com.orcrobertsoft.game", false),
        ("safari", "com.apple.Safari", false),
        ("slack", "com.tinyspeck.slackmacgap", false),
    ];
    for (name, input, want) in cases {
        let id = BundleId::parse(input).expect("valid id");
        assert_eq!(id.is_protected(), want, "{name}");
    }
}

#[test]
fn parses_app_names() {
    assert_eq!(AppName::parse("  Slack ").expect("valid").as_str(), "Slack");
    assert_eq!(
        AppName::parse("Visual Studio Code")
            .expect("valid")
            .to_string(),
        "Visual Studio Code"
    );
    let cases = [
        ("empty", String::new()),
        ("blank", "   ".to_owned()),
        ("newline inside", "Sla\nck".to_owned()),
        ("escape", "Slack\u{1b}[31m".to_owned()),
        ("right-to-left override", "Slack\u{202E}exe.ppa".to_owned()),
        ("isolate", "\u{2067}Slack".to_owned()),
        ("zero-width space", "Sla\u{200B}ck".to_owned()),
        ("byte order mark", "\u{FEFF}Slack".to_owned()),
        ("line separator", "Sla\u{2028}ck".to_owned()),
        ("paragraph separator", "Sla\u{2029}ck".to_owned()),
        ("tag character", "Slack\u{E0041}".to_owned()),
        ("too long", "a".repeat(101)),
    ];
    for (name, input) in cases {
        assert_eq!(AppName::parse(&input), Err(AppError::InvalidName), "{name}");
    }
    assert!(
        AppName::parse("Code \u{1F469}\u{200D}\u{1F4BB}").is_ok(),
        "emoji joined with a zero-width joiner"
    );
    assert!(
        AppName::parse(&"é".repeat(100)).is_ok(),
        "counts characters, not bytes"
    );
}

#[test]
fn names_an_app_after_its_bundle_id() {
    let short = BundleId::parse("com.hnc.Discord").expect("valid id");
    assert_eq!(AppName::from_bundle_id(&short).as_str(), "com.hnc.discord");
    let long = BundleId::parse(&format!("com.{}", "a".repeat(200))).expect("valid id");
    let name = AppName::from_bundle_id(&long);
    assert_eq!(AppName::parse(name.as_str()), Ok(name));
}

#[test]
fn serde_folds_and_revalidates() {
    let id: BundleId = serde_json::from_str(r#""com.apple.Safari""#).expect("parses");
    assert_eq!(
        serde_json::to_string(&id).expect("serializes"),
        r#""com.apple.safari""#
    );
    for bad in [r#""../etc""#, r#""slack""#, r#""""#] {
        assert!(serde_json::from_str::<BundleId>(bad).is_err(), "{bad}");
    }
}

#[test]
fn stored_names_load_cleaned_instead_of_failing() {
    let cases = [
        ("control character", r#""Sla\nck""#, "Slack"),
        ("invisible mark", r#""\u200bSlack\u202e""#, "Slack"),
        (
            "too long",
            &format!(r#""{}""#, "a".repeat(150)),
            &"a".repeat(100),
        ),
    ];
    for (name, json, expected) in cases {
        let loaded: AppName = serde_json::from_str(json).expect(name);
        assert_eq!(loaded.as_str(), expected, "{name}");
    }
    for nothing_left in [r#""""#, r#""\u202e\n""#] {
        assert!(
            serde_json::from_str::<AppName>(nothing_left).is_err(),
            "{nothing_left}"
        );
    }
}

#[test]
fn errors_read_well() {
    assert_eq!(
        AppError::InvalidId("slack".into()).to_string(),
        "That is not an app's bundle ID (slack); pick the app from the list instead"
    );
}

#[test]
fn finds_the_outermost_bundle() {
    let cases = [
        (
            "main binary",
            "/Applications/Slack.app/Contents/MacOS/Slack",
            Some("/Applications/Slack.app"),
        ),
        (
            "nested helper",
            "/Applications/Slack.app/Contents/Frameworks/Slack Helper.app/Contents/MacOS/Slack Helper",
            Some("/Applications/Slack.app"),
        ),
        (
            "spaces",
            "/Users/sam/Desktop/Copy of Calculator.app/Contents/MacOS/Calculator",
            Some("/Users/sam/Desktop/Copy of Calculator.app"),
        ),
        (
            "upper case suffix",
            "/Applications/Game.APP/Contents/MacOS/Game",
            Some("/Applications/Game.APP"),
        ),
        ("not in a bundle", "/opt/homebrew/bin/fish", None),
        ("relative", "Slack.app/Contents/MacOS/Slack", None),
        ("login shell", "-fish", None),
        ("bare suffix", "/tmp/.app/x", None),
        ("suffix inside a name", "/tmp/my.application/x", None),
        (
            "unicode before suffix",
            "/Applications/Café.app/Contents/MacOS/Café",
            Some("/Applications/Café.app"),
        ),
        ("unicode tail", "/tmp/aé", None),
    ];
    for (case, path, want) in cases {
        assert_eq!(outermost_bundle(path), want, "{case}");
    }
}

#[test]
fn lists_every_bundle_in_a_path() {
    let cases: [(&str, &str, &[&str]); 4] = [
        (
            "wrapped copy",
            "/Users/sam/w.app/Slack.app/Contents/MacOS/Slack",
            &["/Users/sam/w.app", "/Users/sam/w.app/Slack.app"],
        ),
        (
            "nested helper",
            "/Applications/Slack.app/Contents/Frameworks/Slack Helper.app/Contents/MacOS/Slack Helper",
            &[
                "/Applications/Slack.app",
                "/Applications/Slack.app/Contents/Frameworks/Slack Helper.app",
            ],
        ),
        ("relative", "w.app/Slack.app/x", &[]),
        ("none", "/opt/homebrew/bin/fish", &[]),
    ];
    for (case, path, want) in cases {
        assert_eq!(bundles(path).collect::<Vec<_>>(), want, "{case}");
    }
}

proptest! {
    #[test]
    fn outermost_bundle_is_a_prefix_ending_in_app(path in "(/[a-zA-Zé .]{0,8}(\\.app)?){0,5}") {
        if let Some(bundle) = outermost_bundle(&path) {
            prop_assert!(path.starts_with(bundle));
            prop_assert!(bundle.to_ascii_lowercase().ends_with(".app"));
            prop_assert_eq!(outermost_bundle(bundle), Some(bundle));
        }
        for bundle in bundles(&path) {
            prop_assert!(path.starts_with(bundle));
            prop_assert_eq!(bundles(bundle).last(), Some(bundle));
        }
    }

    #[test]
    fn parse_never_panics(input in ".*") {
        let _ = BundleId::parse(&input);
        let _ = AppName::parse(&input);
        let _ = outermost_bundle(&input);
    }

    #[test]
    fn parsed_ids_keep_their_invariants(input in "[a-zA-Z0-9._/ -]{0,40}") {
        if let Ok(id) = BundleId::parse(&input) {
            let s = id.as_str();
            prop_assert!(s.len() <= MAX_ID_LEN);
            prop_assert!(s.contains('.'));
            prop_assert!(!s.contains(['/', ' ', '_']));
            prop_assert!(s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.'));
            prop_assert_eq!(BundleId::parse(s), Ok(id.clone()));
            prop_assert_eq!(BundleId::parse(&s.to_ascii_uppercase()), Ok(id));
        }
    }
}
