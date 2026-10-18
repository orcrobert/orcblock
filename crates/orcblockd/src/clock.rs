use std::io;
use std::time::Duration;

use jiff::{Timestamp, Zoned};
#[cfg(unix)]
use nix::time::{ClockId, clock_gettime};
use orcblock_core::Month;

/// The daemon's sources of time.
pub trait Clock: Send {
    /// Wall-clock time, for display and timestamps only.
    fn now(&self) -> Timestamp;

    /// The current calendar month in the system time zone.
    fn month(&self) -> Month;

    /// A monotonic reading that keeps counting while the Mac sleeps and
    /// cannot be changed by setting the clock. Only differences between
    /// readings mean anything.
    fn monotonic(&self) -> io::Result<Duration>;
}

/// The real clocks.
///
/// On Unix `monotonic` is `CLOCK_MONOTONIC`, which on macOS is
/// `mach_continuous_time` and includes sleep; `std::time::Instant` uses
/// `CLOCK_UPTIME_RAW` there and stops while asleep, so it would stretch
/// every block by the time the lid was closed. On Windows `Instant` is
/// `QueryPerformanceCounter`, which includes sleep, measured from when the
/// clock was made.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    #[cfg(windows)]
    origin: std::time::Instant,
}

impl SystemClock {
    /// The system's clocks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            origin: std::time::Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    fn month(&self) -> Month {
        Month::of(Zoned::now().date())
    }

    #[cfg(unix)]
    fn monotonic(&self) -> io::Result<Duration> {
        let spec = clock_gettime(ClockId::CLOCK_MONOTONIC).map_err(io::Error::from)?;
        Ok(Duration::from(spec))
    }

    #[cfg(windows)]
    fn monotonic(&self) -> io::Result<Duration> {
        Ok(self.origin.elapsed())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::io;
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    use jiff::Timestamp;
    use orcblock_core::Month;

    use super::Clock;

    #[derive(Debug)]
    struct Inner {
        now: Timestamp,
        month: Month,
        monotonic: Option<Duration>,
    }

    /// A clock the test moves by hand; clones share the same time.
    #[derive(Debug, Clone)]
    pub(crate) struct FakeClock(Arc<Mutex<Inner>>);

    impl FakeClock {
        pub(crate) fn new(now: Timestamp, month: Month) -> Self {
            Self(Arc::new(Mutex::new(Inner {
                now,
                month,
                monotonic: Some(Duration::from_secs(1000)),
            })))
        }

        fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
            f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
        }

        /// Moves both clocks forward.
        pub(crate) fn advance(&self, by: Duration) {
            self.with(|inner| {
                inner.now = inner.now.checked_add(by).expect("in range");
                inner.monotonic = inner.monotonic.map(|m| m + by);
            });
        }

        /// Moves only the wall clock, as if the user changed it.
        pub(crate) fn set_wall(&self, now: Timestamp) {
            self.with(|inner| inner.now = now);
        }

        pub(crate) fn set_month(&self, month: Month) {
            self.with(|inner| inner.month = month);
        }

        /// Makes `monotonic` fail from now on.
        pub(crate) fn break_monotonic(&self) {
            self.with(|inner| inner.monotonic = None);
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Timestamp {
            self.with(|inner| inner.now)
        }

        fn month(&self) -> Month {
            self.with(|inner| inner.month)
        }

        fn monotonic(&self) -> io::Result<Duration> {
            self.with(|inner| inner.monotonic)
                .ok_or_else(|| io::Error::other("clock broken by the test"))
        }
    }
}

#[cfg(test)]
mod tests;
