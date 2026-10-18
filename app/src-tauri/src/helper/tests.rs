#[cfg(not(windows))]
use std::path::Path;

use super::*;

#[cfg(not(windows))]
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
#[cfg(not(windows))]
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

#[cfg(not(windows))]
#[test]
fn install_shell_runs_only_a_helper_with_the_pinned_hash() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("temp dir");
    let marker = dir.path().join("ran");
    let helper = dir.path().join("it's orcblockd");
    let script = format!("#!/bin/sh\necho \"$@\" > '{}'\n", marker.display());
    std::fs::write(&helper, &script).expect("writes helper");
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let pinned = sha256_hex(script.as_bytes());

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

#[test]
fn hashes_as_lowercase_hex() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn elevation_outcomes() {
    let cases = [
        ("installed", Some(0), "", None),
        ("prompt declined", Some(1223), "", Some("Installation cancelled".to_owned())),
        (
            "helper failed",
            Some(1),
            "",
            Some(r"Installation failed: the helper exited with code 1; see C:\ProgramData\OrcBlock\orcblockd.log".to_owned()),
        ),
        (
            "powershell error",
            Some(1),
            "Start-Process : This command cannot be run.\r\n",
            Some(r"Installation failed: Start-Process : This command cannot be run.; see C:\ProgramData\OrcBlock\orcblockd.log".to_owned()),
        ),
    ];
    for (name, code, stderr, want) in cases {
        assert_eq!(
            outcome(code, stderr).err().map(|err| err.to_string()),
            want,
            "{name}"
        );
    }
}

#[test]
fn elevation_script_takes_the_path_from_the_environment() {
    assert!(
        ELEVATE_SCRIPT
            .contains("-FilePath $env:ORCBLOCK_HELPER -ArgumentList 'install' -Verb RunAs")
    );
    assert!(ELEVATE_SCRIPT.contains("exit 1223"));
    assert_eq!(encode_command("dir"), "ZABpAHIA");
    assert!(POWERSHELL.ends_with(r"\powershell.exe"));
}
