//! Decides when the orc shield appears: once for each app the daemon closed
//! during a block.

use orcblock_ipc::BlockedLaunch;

/// Remembers the newest blocked launch already shown.
#[derive(Debug)]
pub(crate) struct ShieldTracker {
    /// The newest launch seen; compared by id and time, since ids repeat
    /// when the daemon restarts and counts from 1 again.
    seen: Option<BlockedLaunch>,
    /// Whether no status has been seen yet.
    first: bool,
    show_first: bool,
}

impl ShieldTracker {
    /// `woken` is true when the daemon started the app for a launch it
    /// closed; that launch is then shown at once. Otherwise launches from
    /// before the app started are not.
    pub(crate) fn new(woken: bool) -> Self {
        Self {
            seen: None,
            first: true,
            show_first: woken,
        }
    }

    /// The launch to show for this status, if it is new.
    pub(crate) fn next<'a>(&mut self, launches: &'a [BlockedLaunch]) -> Option<&'a BlockedLaunch> {
        let newest = launches.last();
        let key = |launch: &BlockedLaunch| (launch.id, launch.at);
        let show = if self.first {
            self.show_first
        } else {
            self.seen.as_ref().map(key) != newest.map(key)
        };
        self.seen = newest.cloned();
        self.first = false;
        newest.filter(|_| show)
    }
}

#[cfg(test)]
mod tests;
