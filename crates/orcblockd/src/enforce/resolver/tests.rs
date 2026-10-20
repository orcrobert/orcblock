use std::collections::BTreeSet;
use std::fs;

use super::*;

const PORT: u16 = 53535;

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|&n| n.to_owned()).collect()
}

fn files(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| e.expect("entry").file_name().into_string().expect("utf8"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

#[test]
fn sync_writes_rewrites_and_removes_only_marked_files() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (dir, backup) = (tmp.path().join("resolver"), tmp.path().join("backup"));

    assert!(sync(&dir, &backup, &names(&["a.com", "b.com"]), PORT).expect("syncs"));
    assert_eq!(files(&dir), ["a.com", "b.com"]);
    assert_eq!(
        fs::read_to_string(dir.join("a.com")).expect("reads"),
        contents(PORT)
    );
    assert!(
        !sync(&dir, &backup, &names(&["a.com", "b.com"]), PORT).expect("syncs"),
        "idempotent"
    );

    fs::write(dir.join("corp.example"), "nameserver 10.0.0.1\n").expect("writes");
    fs::write(dir.join("a.com"), format!("{MARKER}\nnameserver 8.8.8.8\n")).expect("tampers");
    assert!(drifted(&dir, &names(&["a.com", "b.com"]), PORT).expect("checks"));
    assert!(sync(&dir, &backup, &names(&["a.com"]), PORT).expect("syncs"));
    assert_eq!(files(&dir), ["a.com", "corp.example"]);
    assert_eq!(
        fs::read_to_string(dir.join("a.com")).expect("reads"),
        contents(PORT)
    );
    assert!(!drifted(&dir, &names(&["a.com"]), PORT).expect("checks"));
}

#[test]
fn deleted_files_are_drift() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (dir, backup) = (tmp.path().join("resolver"), tmp.path().join("backup"));
    sync(&dir, &backup, &names(&["a.com"]), PORT).expect("syncs");
    fs::remove_file(dir.join("a.com")).expect("removes");
    assert!(drifted(&dir, &names(&["a.com"]), PORT).expect("checks"));
}

#[test]
fn unmarked_files_in_the_way_are_backed_up_and_restored() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (dir, backup) = (tmp.path().join("resolver"), tmp.path().join("backup"));
    fs::create_dir(&dir).expect("creates");
    fs::write(dir.join("youtube.com"), "nameserver 8.8.8.8\n").expect("writes");

    sync(&dir, &backup, &names(&["youtube.com"]), PORT).expect("syncs");
    assert_eq!(
        fs::read_to_string(dir.join("youtube.com")).expect("reads"),
        contents(PORT)
    );
    assert_eq!(
        fs::read_to_string(backup.join("youtube.com")).expect("reads"),
        "nameserver 8.8.8.8\n"
    );

    assert!(clear(&dir, &backup, PORT).expect("clears"));
    assert_eq!(
        fs::read_to_string(dir.join("youtube.com")).expect("reads"),
        "nameserver 8.8.8.8\n"
    );
    assert_eq!(files(&backup), Vec::<String>::new());
}

#[cfg(unix)]
#[test]
fn symlinks_are_left_alone() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (dir, backup) = (tmp.path().join("resolver"), tmp.path().join("backup"));
    fs::create_dir(&dir).expect("creates");
    let outside = tmp.path().join("outside");
    fs::write(&outside, format!("{MARKER}\n")).expect("writes");
    std::os::unix::fs::symlink(&outside, dir.join("link.com")).expect("links");

    clear(&dir, &backup, PORT).expect("clears");
    assert!(outside.exists());
    assert!(dir.join("link.com").exists());
}
