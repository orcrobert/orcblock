//! Updates the app from the newest published release. The endpoint and the
//! public key that every download must be signed with are compiled in from
//! tauri.conf.json (ADR 0007).

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri_plugin_updater::{Error, Update};

/// A newer release, as the UI receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Offer {
    pub(crate) version: String,
    /// The release date as `YYYY-MM-DD`, when the manifest has one.
    pub(crate) date: Option<String>,
}

impl Offer {
    pub(crate) fn of(update: &Update) -> Self {
        Self {
            version: update.version.clone(),
            date: update.date.map(|date| date.date().to_string()),
        }
    }
}

#[derive(Debug)]
enum Slot<T> {
    Idle,
    Ready(T),
    Installing,
}

/// The update found by the last check, kept so that installing takes no
/// arguments from the webview. Once an install starts, nothing can arm a
/// second one until it fails: two installs would race on the same bundle.
#[derive(Debug)]
pub(crate) struct Pending<T>(Mutex<Slot<T>>);

impl<T> Default for Pending<T> {
    fn default() -> Self {
        Self(Mutex::new(Slot::Idle))
    }
}

impl<T> Pending<T> {
    /// Remembers the result of a check; ignored while an install runs.
    pub(crate) fn set(&self, update: Option<T>) {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(*slot, Slot::Installing) {
            *slot = update.map_or(Slot::Idle, Slot::Ready);
        }
    }

    /// Takes the update to install and marks an install as running; `None`
    /// when there is nothing to install or an install already runs.
    pub(crate) fn start(&self) -> Option<T> {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match std::mem::replace(&mut *slot, Slot::Installing) {
            Slot::Ready(update) => Some(update),
            other => {
                *slot = other;
                None
            }
        }
    }

    /// Ends a failed install and offers the same update again.
    pub(crate) fn fail(&self, update: T) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Slot::Ready(update);
    }
}

/// What the UI receives on the `update_progress` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Progress {
    /// Whole percent, or `None` when the server sent no length.
    Downloading { percent: Option<u8> },
    /// Downloaded; checking the signature and swapping the bundle.
    Installing,
}

/// Turns downloaded chunks into progress, reporting only when the whole
/// percent changes so a large download does not flood the webview.
#[derive(Debug, Default)]
pub(crate) struct Download {
    done: u64,
    reported: Option<Progress>,
}

impl Download {
    /// Adds a chunk of `len` bytes out of `total`; `None` when the percent
    /// shown would not change. Clamps at 100 when the server sends more
    /// than it announced.
    pub(crate) fn add(&mut self, len: usize, total: Option<u64>) -> Option<Progress> {
        self.done = self
            .done
            .saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
        let percent = total.filter(|&total| total > 0).map(|total| {
            let percent = self.done.saturating_mul(100) / total;
            u8::try_from(percent.min(100)).unwrap_or(100)
        });
        let progress = Progress::Downloading { percent };
        (self.reported != Some(progress)).then(|| {
            self.reported = Some(progress);
            progress
        })
    }
}

/// One sentence for the snackbar that says what went wrong and what to do.
pub(crate) fn explain(err: &Error) -> String {
    match err {
        Error::Reqwest(_) | Error::Network(_) | Error::ReleaseNotFound => {
            "Could not download the update; check the connection and try again.".into()
        }
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => {
            "The download is not signed by OrcBlock, so it was not installed.".into()
        }
        Error::AuthenticationFailed => {
            "The update needs the administrator password to replace OrcBlock; try again and allow it.".into()
        }
        Error::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied => {
            "OrcBlock is not allowed to replace itself; move it to the Applications folder and try again.".into()
        }
        Error::PreviousAppNotRestored(path) => format!(
            "The update failed and the old OrcBlock was left at {}; move it back to Applications.",
            path.display()
        ),
        other => format!("Could not update OrcBlock: {other}."),
    }
}

#[cfg(test)]
mod tests;
