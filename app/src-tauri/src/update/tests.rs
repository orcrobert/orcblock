use std::path::PathBuf;

use super::*;

#[test]
fn download_reports_each_whole_percent_once() {
    let cases = [
        ("no length", vec![10, 10], None, vec![Some(None), None]),
        ("zero length", vec![10], Some(0), vec![Some(None)]),
        ("first chunk", vec![1], Some(3), vec![Some(Some(33))]),
        (
            "same percent twice",
            vec![1, 1, 8],
            Some(1000),
            vec![Some(Some(0)), None, Some(Some(1))],
        ),
        (
            "more than announced",
            vec![900, 900],
            Some(1000),
            vec![Some(Some(90)), Some(Some(100))],
        ),
    ];
    for (name, chunks, total, expected) in cases {
        let mut download = Download::default();
        let reported: Vec<_> = chunks
            .into_iter()
            .map(|len| {
                download.add(len, total).map(|progress| match progress {
                    Progress::Downloading { percent } => percent,
                    Progress::Installing => panic!("{name}: a chunk never means installing"),
                })
            })
            .collect();
        assert_eq!(reported, expected, "{name}");
    }
}

#[test]
fn progress_reads_as_the_ui_expects() {
    let json = |progress| serde_json::to_value(progress).expect("progress serializes");
    assert_eq!(
        json(Progress::Downloading { percent: Some(42) }),
        serde_json::json!({ "type": "downloading", "percent": 42 })
    );
    assert_eq!(
        json(Progress::Installing),
        serde_json::json!({ "type": "installing" })
    );
}

#[test]
fn only_one_install_runs_at_a_time() {
    let pending = Pending::default();
    assert_eq!(pending.start(), None, "nothing found yet");

    pending.set(Some("0.3.1"));
    assert_eq!(pending.start(), Some("0.3.1"));
    assert_eq!(pending.start(), None, "already installing");

    pending.set(Some("0.3.2"));
    assert_eq!(
        pending.start(),
        None,
        "a check during an install does not re-arm it"
    );

    pending.fail("0.3.1");
    assert_eq!(
        pending.start(),
        Some("0.3.1"),
        "a failed install is offered again"
    );
}

#[test]
fn a_check_without_an_update_clears_the_offer() {
    let pending = Pending::default();
    pending.set(Some("0.3.1"));
    pending.set(None);
    assert_eq!(pending.start(), None);
}

#[test]
fn errors_read_as_one_sentence() {
    let cases = [
        (
            "no manifest",
            Error::ReleaseNotFound,
            "Could not download the update",
        ),
        (
            "bad signature",
            Error::MissingSignedVersion,
            "not signed by OrcBlock",
        ),
        (
            "cancelled password",
            Error::AuthenticationFailed,
            "administrator password",
        ),
        (
            "read-only app folder",
            Error::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            "Applications folder",
        ),
        (
            "old app moved aside",
            Error::PreviousAppNotRestored(PathBuf::from("/Users/a/OrcBlock-backup.app")),
            "/Users/a/OrcBlock-backup.app",
        ),
        (
            "anything else",
            Error::TempDirNotOnSameMountPoint,
            "Could not update OrcBlock: temp directory",
        ),
    ];
    for (name, err, expected) in cases {
        let text = explain(&err);
        assert!(text.contains(expected), "{name}: {text}");
        assert!(text.ends_with('.'), "{name}: {text}");
    }
}
