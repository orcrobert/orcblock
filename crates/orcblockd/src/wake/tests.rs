use super::fake::FakeConsole;
use super::*;
use crate::enforce::runner::Output;
use crate::enforce::runner::fake::FakeRunner;

const WAKE_LINE: &str = "/bin/launchctl asuser 501 /usr/bin/sudo -u #501 /usr/bin/open -g -b com.orcrobert.orcblock --args --shield";

fn waker() -> (Waker, FakeRunner, FakeConsole) {
    let runner = FakeRunner::new(|_| {
        Ok(Output {
            stdout: String::new(),
            stderr: String::new(),
        })
    });
    let console = FakeConsole::default();
    console.set(Some(501));
    (Waker::new(runner.clone(), console.clone()), runner, console)
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn opens_the_app_for_the_console_user() {
    let (mut waker, runner, _) = waker();
    waker.wake(secs(5));
    assert_eq!(runner.lines(), [WAKE_LINE]);
}

#[test]
fn wakes_at_most_once_a_minute() {
    let (mut waker, runner, _) = waker();
    waker.wake(secs(5));
    waker.wake(secs(30));
    waker.wake(secs(64));
    assert_eq!(runner.lines().len(), 1, "within a minute");
    waker.wake(secs(65));
    assert_eq!(runner.lines().len(), 2);
}

#[test]
fn never_wakes_for_root_or_nobody() {
    let (mut waker, runner, console) = waker();
    for user in [Some(0), None] {
        console.set(user);
        waker.wake(secs(5));
    }
    assert_eq!(runner.lines(), Vec::<String>::new());

    console.set(Some(501));
    waker.wake(secs(6));
    assert_eq!(runner.lines(), [WAKE_LINE], "a skipped wake does not count");
}

#[test]
fn a_failed_wake_still_waits_a_minute() {
    let failing = FakeRunner::new(|_| Err("Could not find domain".to_owned()));
    let console = FakeConsole::default();
    console.set(Some(501));
    let mut waker = Waker::new(failing.clone(), console);
    waker.wake(secs(1));
    waker.wake(secs(2));
    assert_eq!(failing.lines().len(), 1);
}
