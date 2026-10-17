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

    fs::write(dir.path().join("example.com"), b"GIF89a").expect("writes");
    assert_eq!(
        favicons.get(&domain).as_deref(),
        Some("data:image/gif;base64,R0lGODlh")
    );

    fs::remove_file(dir.path().join("example.com")).expect("removes");
    fs::write(dir.path().join("example.com.none"), b"").expect("writes");
    assert_eq!(favicons.get(&domain), None, "a recent miss is not retried");
}
