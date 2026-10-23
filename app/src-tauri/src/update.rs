//! Updates the app from the newest published release. The endpoint and the
//! public key that every download must be signed with are compiled in from
//! tauri.conf.json (ADR 0007).

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri_plugin_updater::Update;

/// A newer release, as the UI receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Offer {
    pub(crate) version: String,
    /// The release date as `YYYY-MM-DD`, when the manifest has one.
    pub(crate) date: Option<String>,
}

impl Offer {
    fn of(update: &Update) -> Self {
        Self {
            version: update.version.clone(),
            date: update.date.map(|date| date.date().to_string()),
        }
    }
}

/// The update found by the last check, kept so that installing takes no
/// arguments from the webview.
#[derive(Default)]
pub(crate) struct Pending(Mutex<Option<Update>>);

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Pending").finish_non_exhaustive()
    }
}

impl Pending {
    /// Remembers the result of a check and returns what to offer.
    pub(crate) fn set(&self, update: Option<Update>) -> Option<Offer> {
        let offer = update.as_ref().map(Offer::of);
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = update;
        offer
    }

    /// Takes the update to install; `None` while another install runs.
    pub(crate) fn take(&self) -> Option<Update> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }

    /// Puts back an update whose install failed, unless a newer check
    /// already replaced it.
    pub(crate) fn restore(&self, update: Update) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert(update);
    }
}

/// Download progress, as the UI receives it on the `update_progress` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Progress {
    /// Whole percent, or `None` when the server sent no length.
    pub(crate) percent: Option<u8>,
}

/// Counts downloaded bytes.
#[derive(Debug, Default)]
pub(crate) struct Download {
    done: u64,
}

impl Download {
    /// Adds a chunk of `len` bytes out of `total`.
    pub(crate) fn add(&mut self, len: usize, total: Option<u64>) -> Progress {
        self.done = self
            .done
            .saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
        let percent = total.filter(|&total| total > 0).map(|total| {
            let percent = self.done.saturating_mul(100) / total;
            u8::try_from(percent.min(100)).unwrap_or(100)
        });
        Progress { percent }
    }
}

#[cfg(test)]
mod tests;
