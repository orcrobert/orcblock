use super::*;

#[test]
fn sniffs_raster_images_only() {
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0];
    let cases: [(&[u8], Option<&str>); 7] = [
        (&png, Some("image/png")),
        (&[0, 0, 1, 0, 1, 0], Some("image/x-icon")),
        (&[0xFF, 0xD8, 0xFF, 0xE0], Some("image/jpeg")),
        (b"GIF89a", Some("image/gif")),
        (b"RIFF\0\0\0\0WEBPVP8 ", Some("image/webp")),
        (b"<svg xmlns=", None),
        (b"<!doctype html>", None),
    ];
    for (bytes, want) in cases {
        assert_eq!(sniff(bytes), want, "{bytes:?}");
    }
}

#[test]
fn serves_from_the_cache_without_the_network() {
    let dir = tempfile::tempdir().expect("temp dir");
    let favicons = Favicons::new(dir.path().to_path_buf());
    let domain = Domain::parse("example.com").expect("valid");
    let key = cache_key(&domain);

    fs::write(dir.path().join(format!("{key}.img")), b"GIF89a").expect("writes");
    assert_eq!(
        favicons.get(&domain).as_deref(),
        Some("data:image/gif;base64,R0lGODlh")
    );

    fs::remove_file(dir.path().join(format!("{key}.img"))).expect("removes");
    fs::write(dir.path().join(format!("{key}.miss")), b"").expect("writes");
    assert_eq!(favicons.get(&domain), None, "a recent miss is not retried");
}

#[test]
fn cache_keys_are_hashes_that_never_collide_with_markers() {
    let plain = cache_key(&Domain::parse("x.com").expect("valid"));
    let suffixed = cache_key(&Domain::parse("x.com.none").expect("valid"));
    assert_eq!(plain.len(), 64);
    assert!(plain.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_ne!(plain, suffixed);
}

#[test]
fn resolves_only_to_https_urls_on_public_host_names() {
    let base = "https://www.example.com/a/page";
    let cases = [
        (
            "path",
            "/static/icon.png",
            Some("https://www.example.com/static/icon.png"),
        ),
        (
            "relative",
            "icon.png",
            Some("https://www.example.com/a/icon.png"),
        ),
        (
            "same host",
            "https://example.com/a.ico",
            Some("https://example.com/a.ico"),
        ),
        (
            "other site",
            "https://cdn.test.net/a.ico",
            Some("https://cdn.test.net/a.ico"),
        ),
        (
            "scheme relative",
            "//cdn.test.net/a.ico",
            Some("https://cdn.test.net/a.ico"),
        ),
        (
            "upper case host",
            "https://WWW.Example.com/a.ico",
            Some("https://WWW.Example.com/a.ico"),
        ),
        (
            "default port",
            "https://cdn.test.net:443/a.ico",
            Some("https://cdn.test.net:443/a.ico"),
        ),
        (
            "query only",
            "?v=2",
            Some("https://www.example.com/a/page?v=2"),
        ),
        (
            "colon in query",
            "i.png?v=a:b",
            Some("https://www.example.com/a/i.png?v=a:b"),
        ),
        (
            "punycode tld",
            "https://cdn.xn--p1ai/a.ico",
            Some("https://cdn.xn--p1ai/a.ico"),
        ),
        ("plain http", "http://example.com/a.ico", None),
        ("other port", "https://cdn.test.net:8443/a.ico", None),
        ("link local", "https://169.254.169.254/latest", None),
        ("ipv4", "https://10.0.0.1/a.ico", None),
        ("ipv6", "https://[::1]/a.ico", None),
        ("localhost", "https://localhost/a.ico", None),
        ("localhost subdomain", "https://x.localhost/a.ico", None),
        ("localhost, upper case", "https://x.LOCALHOST/a.ico", None),
        (
            "localhost, default port",
            "https://x.localhost:443/a.ico",
            None,
        ),
        (
            "localhost, trailing dot",
            "https://x.localhost./a.ico",
            None,
        ),
        ("trailing dot", "https://cdn.test.net./a.ico", None),
        ("hex ipv4", "https://0x7f.0x1/a.ico", None),
        ("hex ipv4, one part", "https://0xa.0xa80101/a.ico", None),
        ("numeric labels", "https://10.0.0x1/a.ico", None),
        ("data url", "data:image/png;base64,AAAA", None),
        ("javascript", "javascript:alert(1)", None),
        ("garbage", "not a url", None),
    ];
    for (name, reference, want) in cases {
        assert_eq!(resolve(base, reference).as_deref(), want, "{name}");
    }
}

#[test]
fn finds_raster_icon_links_best_first() {
    let cases: [(&str, &str, &[&str]); 10] = [
        ("none", "<html><head><title>x</title></head>", &[]),
        (
            "apple touch before icon",
            r#"<link rel="icon" href="/a.png"><link rel="apple-touch-icon" sizes="180x180" href="/b.png">"#,
            &["/b.png", "/a.png"],
        ),
        (
            "href first, upper case",
            r#"<LINK HREF="https://cdn.test.net/x.ico" REL="Shortcut Icon" type="image/x-icon"/>"#,
            &["https://cdn.test.net/x.ico"],
        ),
        (
            "single quotes",
            "<link rel='icon' href='/s.png'>",
            &["/s.png"],
        ),
        ("unquoted", "<link rel=icon href=/u.png>", &["/u.png"]),
        (
            "svg skipped",
            r#"<link rel="icon" href="/i.svg"><link rel="icon" type="image/svg+xml" href="/j"><link rel="icon" href="/k.svg?v=2">"#,
            &[],
        ),
        (
            "mask icon and stylesheets skipped",
            r#"<link rel="mask-icon" href="/m.png"><link rel="stylesheet" href="/s.css">"#,
            &[],
        ),
        (
            "entity in href",
            r#"<link rel="icon" href="/i.png?a=1&amp;b=2">"#,
            &["/i.png?a=1&b=2"],
        ),
        (
            "precomposed",
            r#"<link rel="apple-touch-icon-precomposed" href="/p.png">"#,
            &["/p.png"],
        ),
        (
            "unterminated",
            r#"<link rel="icon" href="/t.png"#,
            &["/t.png"],
        ),
    ];
    for (name, html, want) in cases {
        assert_eq!(icon_links(html), want, "{name}");
    }
}

proptest::proptest! {
    #[test]
    fn link_scanning_never_panics(html in "\\PC*") {
        let _ = icon_links(&html);
    }

    #[test]
    fn resolves_only_to_https_on_a_public_name(
        reference in concat!(
            "(https?:)?//(0x[0-9a-f]{1,4}|[0-9]{1,4}|localhost|LocalHost|[a-z]{1,5})",
            "(\\.(0x[0-9a-f]{1,4}|[0-9]{1,3}|localhost|[a-z]{1,5})){0,3}\\.?(:[0-9]{1,4})?/x",
            "|(https?:)?(//)?[a-z0-9x.:/?]{0,30}|\\PC*",
        ),
    ) {
        if let Some(url) = resolve("https://example.com/a", &reference) {
            let uri: ureq::http::Uri = url.parse().expect("resolved URLs parse");
            proptest::prop_assert_eq!(uri.scheme_str(), Some("https"));
            proptest::prop_assert!(uri.port_u16().is_none_or(|port| port == 443));
            let host = uri.host().expect("resolved URLs have a host").to_ascii_lowercase();
            // Every numeric IPv4 spelling the resolver accepts ends in a part
            // that starts with a digit.
            let last = host.rsplit('.').next().unwrap_or_default();
            proptest::prop_assert!(!last.starts_with(|c: char| c.is_ascii_digit()), "{}", url);
            proptest::prop_assert!(host.parse::<std::net::Ipv4Addr>().is_err(), "{}", url);
            proptest::prop_assert!(!host.ends_with('.') && last != "localhost", "{}", url);
        }
    }
}

/// What a fetch should give: the icon bytes or why there is none.
type Wanted = Result<&'static [u8], Failure>;

/// A URL and what the fake answers for it.
type Answer = (&'static str, Result<&'static [u8], Failure>);

/// Answers from a table of URLs; anything else is unreachable.
struct FakeHttp(Vec<Answer>);

impl Http for FakeHttp {
    fn get(&self, url: String, _: Body) -> Result<(String, Vec<u8>), Failure> {
        match self.0.iter().find(|(known, _)| *known == url) {
            Some((_, Ok(bytes))) => Ok((url, bytes.to_vec())),
            Some((_, Err(Failure::Missing))) => Err(Failure::Missing),
            _ => Err(Failure::Unreachable),
        }
    }
}

#[test]
fn fetches_from_paths_then_home_page_then_www() {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const HOME: &[u8] = br#"<link rel="icon" href="https://cdn.test.net/i.png">"#;
    const FOUR_LINKS: &[u8] = br#"<link rel="icon" href="/1.png"><link rel="icon" href="/2.png">
        <link rel="icon" href="/3.png"><link rel="icon" href="/4.png">"#;
    const BOTH_LINKS: &[u8] = br#"<link rel="icon" href="/plain.png">
        <link rel="apple-touch-icon" href="/touch.png">"#;
    let cases: [(&str, FakeHttp, Wanted); 10] = [
        (
            "nothing answers",
            FakeHttp(vec![]),
            Err(Failure::Unreachable),
        ),
        (
            "favicon.ico",
            FakeHttp(vec![
                (
                    "https://example.com/apple-touch-icon.png",
                    Err(Failure::Missing),
                ),
                ("https://example.com/favicon.ico", Ok(PNG)),
            ]),
            Ok(PNG),
        ),
        (
            "html where an icon should be, no home page links",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Ok(b"<!doctype html>")),
                ("https://example.com/", Ok(b"<html></html>")),
            ]),
            Err(Failure::Missing),
        ),
        (
            "home page link",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                ("https://example.com/", Ok(HOME)),
                ("https://cdn.test.net/i.png", Ok(PNG)),
            ]),
            Ok(PNG),
        ),
        (
            "paths answer but the home page does not: not a cached miss",
            FakeHttp(vec![(
                "https://example.com/favicon.ico",
                Err(Failure::Missing),
            )]),
            Err(Failure::Unreachable),
        ),
        (
            "linked icon on an unreachable cdn: not a cached miss",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                ("https://example.com/", Ok(HOME)),
            ]),
            Err(Failure::Unreachable),
        ),
        (
            "link to a path already tried is skipped",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                (
                    "https://example.com/",
                    Ok(br#"<link rel="icon" href="/favicon.ico">"#),
                ),
            ]),
            Err(Failure::Missing),
        ),
        (
            "only the first links are tried",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                ("https://example.com/", Ok(FOUR_LINKS)),
                ("https://example.com/1.png", Err(Failure::Missing)),
                ("https://example.com/2.png", Err(Failure::Missing)),
                ("https://example.com/3.png", Err(Failure::Missing)),
                ("https://example.com/4.png", Ok(PNG)),
            ]),
            Err(Failure::Missing),
        ),
        (
            "apple touch link before plain icon",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                ("https://example.com/", Ok(BOTH_LINKS)),
                ("https://example.com/plain.png", Ok(b"GIF89a")),
                ("https://example.com/touch.png", Ok(PNG)),
            ]),
            Ok(PNG),
        ),
        (
            "only www answers",
            FakeHttp(vec![("https://www.example.com/favicon.ico", Ok(PNG))]),
            Ok(PNG),
        ),
    ];
    let domain = Domain::parse("example.com").expect("valid");
    for (name, http, want) in cases {
        assert_eq!(fetch(&http, &domain), want.map(<[u8]>::to_vec), "{name}");
    }
}

#[test]
fn writes_atomically_over_an_existing_entry() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("entry.img");
    fs::write(&path, b"old").expect("writes");
    write_atomic(&dir.path().join(".1.tmp"), &path, b"new").expect("replaces");
    assert_eq!(fs::read(&path).expect("reads"), b"new");
    let leftovers = fs::read_dir(dir.path()).expect("lists").count();
    assert_eq!(leftovers, 1, "no temp file left behind");
}

#[test]
fn only_status_codes_count_as_missing() {
    assert_eq!(
        Failure::from(ureq::Error::StatusCode(404)),
        Failure::Missing
    );
    assert_eq!(
        Failure::from(ureq::Error::HostNotFound),
        Failure::Unreachable
    );
    assert_eq!(
        Failure::from(ureq::Error::ConnectionFailed),
        Failure::Unreachable
    );
}

#[test]
fn caps_icons_and_cuts_pages() {
    let max = usize::try_from(MAX_BYTES).expect("fits");
    let cases = [
        ("small icon", vec![0; 10], Body::Whole, Ok(10)),
        ("icon at the cap", vec![0; max], Body::Whole, Ok(max)),
        (
            "icon over the cap",
            vec![0; max + 1],
            Body::Whole,
            Err(Failure::Missing),
        ),
        ("page over the cap", vec![0; max + 1], Body::Prefix, Ok(max)),
    ];
    for (name, bytes, body, want) in cases {
        assert_eq!(capped(bytes, body).map(|kept| kept.len()), want, "{name}");
    }
}

#[test]
fn caches_icons_and_misses_but_not_unreachable_sites() {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let cases: [(&str, FakeHttp, &str); 3] = [
        (
            "icon",
            FakeHttp(vec![("https://example.com/favicon.ico", Ok(PNG))]),
            "img",
        ),
        (
            "miss",
            FakeHttp(vec![
                ("https://example.com/favicon.ico", Err(Failure::Missing)),
                ("https://example.com/", Ok(b"<html></html>")),
            ]),
            "miss",
        ),
        ("unreachable", FakeHttp(vec![]), ""),
    ];
    let domain = Domain::parse("example.com").expect("valid");
    for (name, http, want) in cases {
        let dir = tempfile::tempdir().expect("temp dir");
        let favicons = Favicons::new(dir.path().to_path_buf());
        let icon = favicons.get_with(&http, &domain);
        assert_eq!(icon.is_some(), want == "img", "{name}");
        let files: Vec<String> = fs::read_dir(dir.path())
            .map(|entries| {
                entries
                    .map(|entry| {
                        entry
                            .expect("lists")
                            .file_name()
                            .into_string()
                            .expect("utf-8")
                    })
                    .collect()
            })
            .unwrap_or_default();
        let kinds: Vec<&str> = files
            .iter()
            .filter_map(|file| file.rsplit_once('.').map(|(_, kind)| kind))
            .collect();
        let expected: &[&str] = if want.is_empty() { &[] } else { &[want] };
        assert_eq!(kinds, expected, "{name}: {files:?}");
    }
}
