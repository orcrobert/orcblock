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
fn follows_redirects_only_within_the_site_over_https() {
    let domain = Domain::parse("example.com").expect("valid");
    let cases = [
        (
            "path",
            "/static/icon.png",
            Some("https://example.com/static/icon.png"),
        ),
        (
            "same host",
            "https://example.com/a.ico",
            Some("https://example.com/a.ico"),
        ),
        (
            "subdomain",
            "https://www.example.com/a.ico",
            Some("https://www.example.com/a.ico"),
        ),
        (
            "upper case host",
            "https://WWW.Example.com/a.ico",
            Some("https://WWW.Example.com/a.ico"),
        ),
        ("plain http", "http://example.com/a.ico", None),
        ("other site", "https://evil.test/a.ico", None),
        ("lookalike suffix", "https://notexample.com/a.ico", None),
        ("scheme relative", "//evil.test/a.ico", None),
        ("link local", "https://169.254.169.254/latest", None),
        ("garbage", "not a url", None),
    ];
    for (name, location, want) in cases {
        assert_eq!(
            same_site_redirect(&domain, location).as_deref(),
            want,
            "{name}"
        );
    }
}

#[test]
fn writes_atomically_over_an_existing_entry() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("entry.img");
    fs::write(&path, b"old").expect("writes");
    write_atomic(dir.path(), &path, b"new").expect("replaces");
    assert_eq!(fs::read(&path).expect("reads"), b"new");
    let leftovers = fs::read_dir(dir.path()).expect("lists").count();
    assert_eq!(leftovers, 1, "no temp file left behind");
}

#[test]
fn only_status_codes_and_oversized_bodies_count_as_missing() {
    assert_eq!(
        Failure::from(ureq::Error::StatusCode(404)),
        Failure::Missing
    );
    assert_eq!(
        Failure::from(ureq::Error::BodyExceedsLimit(1)),
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
