use std::time::{Duration, Instant};

use super::run_with_timeout;
use crate::enforce::EnforceError;

const LONG: Duration = Duration::from_secs(10);

#[test]
fn feeds_stdin_and_returns_stdout() {
    let output = run_with_timeout("/bin/cat", &[], Some("hello\n"), LONG).expect("runs");
    assert_eq!(output.stdout, "hello\n");
}

#[test]
fn reads_more_output_than_a_pipe_holds() {
    let input = "x".repeat(1 << 20);
    let output = run_with_timeout("/bin/cat", &[], Some(&input), LONG).expect("runs");
    assert_eq!(output.stdout.len(), input.len());
}

#[test]
fn a_failing_program_is_an_error() {
    let err = run_with_timeout("/usr/bin/false", &[], None, LONG).expect_err("fails");
    assert!(matches!(err, EnforceError::Program { .. }), "{err:?}");
}

#[test]
fn a_program_that_hangs_is_killed() {
    let started = Instant::now();
    let err = run_with_timeout("/bin/sleep", &["30"], None, Duration::from_millis(200))
        .expect_err("times out");
    assert!(err.to_string().contains("did not finish"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(5));
}
