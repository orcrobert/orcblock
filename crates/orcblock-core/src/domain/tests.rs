use proptest::prelude::*;

use super::*;

const CLEANED: &[(&str, &str, &str)] = &[
    ("bare", "youtube.com", "youtube.com"),
    ("uppercase", "YouTube.COM", "youtube.com"),
    (
        "https url",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "youtube.com",
    ),
    (
        "http url with port",
        "http://news.ycombinator.com:8080/item?id=1",
        "news.ycombinator.com",
    ),
    ("no scheme with path", "reddit.com/r/rust", "reddit.com"),
    ("protocol relative", "//twitter.com/home", "twitter.com"),
    ("credentials", "https://user:pass@x.com/", "x.com"),
    ("fragment", "example.org#top", "example.org"),
    ("query only", "example.org?a=b", "example.org"),
    ("trailing dot", "youtube.com.", "youtube.com"),
    ("surrounding space", "  instagram.com \n", "instagram.com"),
    ("www stripped", "www.facebook.com", "facebook.com"),
    ("www only once", "www.www.example.com", "www.example.com"),
    ("www as the name kept", "www.com", "www.com"),
    ("subdomain kept", "m.youtube.com", "m.youtube.com"),
    ("idn to punycode", "bücher.de", "xn--bcher-kva.de"),
    ("idn url", "https://Bücher.DE/x", "xn--bcher-kva.de"),
    ("punycode kept", "xn--bcher-kva.de", "xn--bcher-kva.de"),
    ("numeric label", "123movies.to", "123movies.to"),
    (
        "other scheme",
        "ftp://files.example.com/a",
        "files.example.com",
    ),
];

#[test]
fn cleans_input() {
    for &(name, input, want) in CLEANED {
        let got = Domain::parse(input).unwrap_or_else(|e| panic!("{name}: {input:?} failed: {e}"));
        assert_eq!(got.as_ascii(), want, "{name}");
    }
}

#[test]
fn rejects_bad_input() {
    type Check = fn(&DomainError) -> bool;
    let cases: [(&str, &str, Check); 12] = [
        ("empty", "", |e| *e == DomainError::Empty),
        ("whitespace", "   ", |e| *e == DomainError::Empty),
        ("scheme only", "https://", |e| *e == DomainError::Empty),
        ("ipv4", "192.168.1.1", |e| {
            matches!(e, DomainError::IpAddress(_))
        }),
        ("ipv4 url", "http://10.0.0.1:80/", |e| {
            matches!(e, DomainError::IpAddress(_))
        }),
        ("ipv6", "http://[::1]:8080/", |e| {
            matches!(e, DomainError::IpAddress(_))
        }),
        ("single label", "localhost", |e| {
            matches!(e, DomainError::SingleLabel(_))
        }),
        ("space inside", "you tube.com", |e| {
            matches!(e, DomainError::Invalid(_))
        }),
        ("underscore", "my_site.com", |e| {
            matches!(e, DomainError::Invalid(_))
        }),
        ("leading hyphen", "-bad.com", |e| {
            matches!(e, DomainError::Invalid(_))
        }),
        ("empty label", "a..com", |e| {
            matches!(e, DomainError::Invalid(_))
        }),
        ("bad port", "example.com:http", |e| {
            matches!(e, DomainError::Invalid(_))
        }),
    ];
    for (name, input, check) in cases {
        let err = Domain::parse(input).expect_err(name);
        assert!(check(&err), "{name}: {input:?} gave {err:?}");
    }
}

#[test]
fn rejects_too_long_names() {
    let long_label = format!("{}.com", "a".repeat(64));
    assert!(matches!(
        Domain::parse(&long_label),
        Err(DomainError::Invalid(_))
    ));
    let long_name = format!("{}com", "abcdefghi.".repeat(26));
    assert!(matches!(
        Domain::parse(&long_name),
        Err(DomainError::Invalid(_))
    ));
}

#[test]
fn displays_unicode() {
    let domain = Domain::parse("xn--bcher-kva.de").expect("valid");
    assert_eq!(domain.to_string(), "bücher.de");
    assert_eq!(domain.as_ascii(), "xn--bcher-kva.de");
}

#[test]
fn covers_itself_and_subdomains_only() {
    let parent = Domain::parse("youtube.com").expect("valid");
    let cases = [
        ("itself", "youtube.com", true),
        ("subdomain", "m.youtube.com", true),
        ("deep subdomain", "a.b.youtube.com", true),
        ("lookalike suffix", "notyoutube.com", false),
        ("different tld", "youtube.co", false),
        ("parent", "com.youtube.com.evil.net", false),
    ];
    for (name, other, want) in cases {
        let other = Domain::parse(other).expect("valid");
        assert_eq!(parent.covers(&other), want, "{name}");
    }
    assert!(
        !Domain::parse("m.youtube.com")
            .expect("valid")
            .covers(&parent)
    );
}

#[test]
fn matches_query_names() {
    let domain = Domain::parse("youtube.com").expect("valid");
    let cases = [
        ("exact", "youtube.com", true),
        ("trailing dot", "youtube.com.", true),
        ("uppercase", "WWW.YouTube.COM.", true),
        ("subdomain", "i.ytimg.youtube.com", true),
        ("lookalike", "fakeyoutube.com", false),
        ("shorter", "tube.com", false),
        ("empty", "", false),
        ("root", ".", false),
    ];
    for (name, query, want) in cases {
        assert_eq!(domain.matches_name(query), want, "{name}");
    }
}

#[test]
fn serde_round_trips_and_revalidates() {
    let domain = Domain::parse("bücher.de").expect("valid");
    let json = serde_json::to_string(&domain).expect("serializes");
    assert_eq!(json, r#""xn--bcher-kva.de""#);
    let back: Domain = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, domain);

    for bad in [
        r#""YouTube.com""#,
        r#""www.youtube.com""#,
        r#""../etc""#,
        r#""a/b.com""#,
        r#""localhost""#,
    ] {
        assert!(
            serde_json::from_str::<Domain>(bad).is_err(),
            "{bad} must not deserialize"
        );
    }
}

proptest! {
    #[test]
    fn parse_never_panics(input in ".*") {
        let _ = Domain::parse(&input);
    }

    #[test]
    fn parsed_domains_keep_their_invariants(input in "[a-zA-Z0-9.:/@?#_ -]{0,80}") {
        if let Ok(domain) = Domain::parse(&input) {
            let ascii = domain.as_ascii();
            prop_assert!(ascii.len() <= MAX_LEN);
            prop_assert!(ascii.contains('.'));
            prop_assert!(!ascii.starts_with("www.") || ascii.matches('.').count() == 1);
            prop_assert!(ascii.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.'));
            prop_assert_eq!(Domain::parse(ascii), Ok(domain.clone()));
            prop_assert!(domain.covers(&domain));
            prop_assert!(domain.matches_name(ascii));
        }
    }

    #[test]
    fn urls_reduce_to_their_host(
        host in "[a-z][a-z0-9]{0,10}\\.(com|org|dev)",
        path in "(/[a-z0-9]{0,8}){0,3}",
        query in "(\\?[a-z]=[0-9]{1,3})?",
    ) {
        let url = format!("https://{host}{path}{query}");
        prop_assert_eq!(Domain::parse(&url).map(|d| d.as_ascii().to_owned()), Ok(host));
    }
}
