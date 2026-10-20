use super::*;

#[test]
fn system_monotonic_moves_forward() {
    let clock = SystemClock::new();
    let first = clock.monotonic().expect("readable");
    let second = clock.monotonic().expect("readable");
    assert!(second >= first);
    #[cfg(unix)]
    assert!(first > Duration::ZERO);
}
