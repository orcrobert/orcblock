use std::fmt::Write as _;
use std::path::Path;

use sha2::Digest as _;

use super::*;

#[test]
fn install_script_quotes_the_path() {
    let script = install_script(
        Path::new("/Applications/Orc \"Block\".app/Contents/MacOS/orcblockd"),
        "ab",
    );
    assert!(script.starts_with("do shell script \"t=$(/usr/bin/mktemp -d)"));
    assert!(script.contains(
        r#"& quoted form of "/Applications/Orc \"Block\".app/Contents/MacOS/orcblockd" &"#
    ));
    assert!(script.contains(r#"!= \"ab\""#));
    assert!(script.ends_with("with administrator privileges"));
}

/// Runs the root install shell, unprivileged, on `helper`.
fn run_install_shell(helper: &Path, sha256: &str) -> std::process::Output {
    let (before, after) = install_shell(sha256);
    // What AppleScript's `quoted form of` produces.
    let quoted = format!("'{}'", helper.to_string_lossy().replace('\'', r"'\''"));
    Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("{before}{quoted}{after}"))
        .output()
        .expect("runs sh")
}

#[test]
fn install_shell_runs_only_a_helper_with_the_pinned_hash() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("temp dir");
    let marker = dir.path().join("ran");
    let helper = dir.path().join("it's orcblockd");
    let script = format!("#!/bin/sh\necho \"$@\" > '{}'\n", marker.display());
    std::fs::write(&helper, &script).expect("writes helper");
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let pinned: String = sha2::Sha256::digest(script.as_bytes()).iter().fold(
        String::with_capacity(64),
        |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        },
    );

    let refused = run_install_shell(&helper, &"0".repeat(64));
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("does not match"));
    assert!(!marker.exists(), "a mismatched helper never runs");

    let installed = run_install_shell(&helper, &pinned);
    assert!(installed.status.success(), "{installed:?}");
    assert_eq!(
        std::fs::read_to_string(&marker).expect("helper ran"),
        "install\n"
    );
}
