use super::*;

#[test]
fn system_monotonic_moves_forward() {
    let clock = SystemClock;
    let first = clock.monotonic().expect("readable");
    let second = clock.monotonic().expect("readable");
    assert!(second >= first);
    assert!(first > Duration::ZERO);
}
