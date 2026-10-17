use std::path::Path;
use std::time::Duration;

use super::*;

#[test]
fn system_runner_returns_output() {
    let out = SystemRunner
        .run(
            Path::new("."),
            "sh",
            &["-c", "echo hi"],
            Duration::from_secs(5),
        )
        .expect("sh runs");
    assert_eq!(out.trim(), "hi");
}

#[test]
fn system_runner_reports_failure_with_output() {
    let err = SystemRunner
        .run(
            Path::new("."),
            "sh",
            &["-c", "echo bad >&2; exit 3"],
            Duration::from_secs(5),
        )
        .expect_err("non-zero exit");
    assert_eq!(err, RunError::Failed("bad\n".into()));
}

#[test]
fn system_runner_kills_on_timeout() {
    let err = SystemRunner
        .run(
            Path::new("."),
            "sh",
            &["-c", "sleep 5"],
            Duration::from_millis(100),
        )
        .expect_err("times out");
    assert_eq!(err, RunError::TimedOut);
}

#[test]
fn system_runner_reports_missing_program() {
    let err = SystemRunner
        .run(
            Path::new("."),
            "definitely-not-a-program-xyz",
            &[],
            Duration::from_secs(1),
        )
        .expect_err("spawn fails");
    assert!(matches!(err, RunError::Spawn(_)));
}
