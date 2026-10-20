//! Wakes `OrcBlock.app` to show the shield when the daemon closed a blocked
//! app and the app is not running (ADR 0006).

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::CommandRunner;

/// The app's bundle ID; a process of it means the app is open.
pub const APP_BUNDLE_ID: &str = "com.orcrobert.orcblock";

/// The least time between two wakes, so a relaunch loop cannot spawn the
/// app over and over.
pub const WAKE_EVERY: Duration = Duration::from_mins(1);

const LAUNCHCTL: &str = "/bin/launchctl";
const SUDO: &str = "/usr/bin/sudo";
const OPEN: &str = "/usr/bin/open";
const CONSOLE: &str = "/dev/console";

/// Tells who is logged in at the screen.
pub trait Console: Send {
    /// The uid owning the console, or `None` when it cannot be read.
    fn user(&self) -> Option<u32>;
}

/// Reads the owner of `/dev/console`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemConsole;

impl Console for SystemConsole {
    fn user(&self) -> Option<u32> {
        fs::metadata(CONSOLE).map(|meta| meta.uid()).ok()
    }
}

/// Opens the app as the console user, in their session, with `--shield`.
pub struct Waker {
    runner: Box<dyn CommandRunner>,
    console: Box<dyn Console>,
    last: Option<Duration>,
}

impl std::fmt::Debug for Waker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Waker")
            .field("last", &self.last)
            .finish_non_exhaustive()
    }
}

impl Waker {
    /// A waker that runs `launchctl` through `runner` for the user `console`
    /// reports.
    #[must_use]
    pub fn new(runner: impl CommandRunner + 'static, console: impl Console + 'static) -> Self {
        Self {
            runner: Box::new(runner),
            console: Box::new(console),
            last: None,
        }
    }

    /// Opens the app unless it was woken less than [`WAKE_EVERY`] ago or
    /// nobody is logged in. `monotonic` is the daemon's clock reading.
    pub(crate) fn wake(&mut self, monotonic: Duration) {
        if self
            .last
            .is_some_and(|at| monotonic.saturating_sub(at) < WAKE_EVERY)
        {
            return;
        }
        // Root owns the console at the login window; there is no session
        // to show the shield in.
        let Some(uid) = self.console.user().filter(|&uid| uid != 0) else {
            debug!("nobody at the console; not waking the app");
            return;
        };
        self.last = Some(monotonic);
        let uid = uid.to_string();
        let as_user = format!("#{uid}");
        // `asuser` only moves into the user's session; the process is still
        // root. `sudo -u` drops to the user, so whatever LaunchServices
        // finds under the bundle ID runs as that user, never as root.
        let args = [
            "asuser",
            uid.as_str(),
            SUDO,
            "-u",
            as_user.as_str(),
            OPEN,
            "-g",
            "-b",
            APP_BUNDLE_ID,
            "--args",
            "--shield",
        ];
        match self.runner.run(LAUNCHCTL, &args, None) {
            Ok(_) => info!(uid, "woke the app to show the shield"),
            Err(err) => warn!(%err, "could not wake the app"),
        }
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::Console;

    /// A console whose user the test sets.
    #[derive(Debug, Clone, Default)]
    pub(crate) struct FakeConsole {
        user: Arc<Mutex<Option<u32>>>,
    }

    impl FakeConsole {
        pub(crate) fn set(&self, user: Option<u32>) {
            *self.user.lock().unwrap_or_else(PoisonError::into_inner) = user;
        }
    }

    impl Console for FakeConsole {
        fn user(&self) -> Option<u32> {
            *self.user.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }
}

#[cfg(test)]
mod tests;
