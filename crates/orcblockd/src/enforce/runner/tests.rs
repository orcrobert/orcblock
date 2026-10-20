use super::*;

const SHORT: Duration = Duration::from_millis(500);

#[test]
fn returns_what_a_program_prints() {
    let output = run_within("/bin/echo", &["hi"], None, SHORT).expect("echo runs");
    assert_eq!(output.stdout, "hi\n");
}

#[test]
fn a_large_output_and_input_do_not_block() {
    let input = "orc\n".repeat(64 * 1024);
    let output = run_within("/bin/cat", &[], Some(input.as_bytes()), DEADLINE).expect("cat runs");
    assert_eq!(output.stdout.len(), input.len());
}

#[test]
fn a_failing_program_is_an_error() {
    assert!(run_within("/usr/bin/false", &[], None, SHORT).is_err());
}

#[test]
fn a_hanging_program_is_killed_at_the_deadline() {
    let started = Instant::now();
    let err = run_within("/bin/sleep", &["30"], None, Duration::from_millis(100))
        .expect_err("sleep outlives the deadline");
    assert!(err.to_string().contains("killed"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn input_nobody_reads_does_not_outlast_the_deadline() {
    let started = Instant::now();
    let input = vec![b'x'; 1024 * 1024];
    let result = run_within(
        "/bin/sleep",
        &["30"],
        Some(&input),
        Duration::from_millis(100),
    );
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_stopped_program_is_reaped() {
    let child = Command::new("/bin/sleep")
        .arg("30")
        .process_group(0)
        .spawn()
        .expect("sleep starts");
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("pid fits"));
    stop(child, true);
    let started = Instant::now();
    // A zombie still answers signal 0; only a reaped pid is gone.
    while nix::sys::signal::kill(pid, None).is_ok() {
        assert!(started.elapsed() < Duration::from_secs(5), "never reaped");
        thread::sleep(POLL);
    }
}
