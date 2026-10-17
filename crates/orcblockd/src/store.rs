use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use orcblock_core::State;
use serde::{Deserialize, Serialize};

/// Persists [`State`] as JSON in one file, replaced atomically.
#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
}

/// What was last persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    /// The wall-clock time of the save, used to credit the time the daemon
    /// was not running.
    pub saved_at: Timestamp,
    /// The state.
    pub state: State,
}

#[derive(Serialize)]
struct SavedRef<'a> {
    saved_at: Timestamp,
    state: &'a State,
}

/// The state file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Reading failed.
    #[error("read {path}: {source}")]
    Read {
        /// The state file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// Writing failed; the previous file is intact.
    #[error("write {path}: {source}")]
    Write {
        /// The file being written.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The file exists but is not a valid state.
    #[error("corrupt state file {path}: {source}")]
    Corrupt {
        /// The state file.
        path: PathBuf,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
}

impl Store {
    /// A store for the file at `path`; its directory must exist.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The state file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the last save, or `None` when nothing was saved yet.
    pub fn load(&self) -> Result<Option<Saved>, StoreError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(StoreError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|source| StoreError::Corrupt {
                path: self.path.clone(),
                source,
            })
    }

    /// Replaces the file with `state`: written to a temporary file in the
    /// same directory, synced, then renamed over the old one, so a crash
    /// leaves either the old state or the new one. The file is mode 0644.
    pub fn save(&self, state: &State, saved_at: Timestamp) -> Result<(), StoreError> {
        let tmp = self.sibling("tmp");
        let write_err = |path: &Path| {
            let path = path.to_path_buf();
            move |source| StoreError::Write { path, source }
        };
        let mut json = serde_json::to_vec_pretty(&SavedRef { saved_at, state })
            .map_err(io::Error::other)
            .map_err(write_err(&tmp))?;
        json.push(b'\n');

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o644)
            .open(&tmp)
            .map_err(write_err(&tmp))?;
        file.set_permissions(Permissions::from_mode(0o644))
            .and_then(|()| file.write_all(&json))
            .and_then(|()| file.sync_all())
            .map_err(write_err(&tmp))?;
        fs::rename(&tmp, &self.path).map_err(write_err(&self.path))?;
        if let Some(dir) = self.path.parent() {
            File::open(dir)
                .and_then(|dir| dir.sync_all())
                .map_err(write_err(dir))?;
        }
        Ok(())
    }

    /// Moves an unreadable state file aside so the daemon can start fresh,
    /// and returns where it went.
    pub fn quarantine(&self) -> Result<PathBuf, StoreError> {
        let aside = self.sibling("corrupt");
        fs::rename(&self.path, &aside).map_err(|source| StoreError::Write {
            path: aside.clone(),
            source,
        })?;
        Ok(aside)
    }

    fn sibling(&self, suffix: &str) -> PathBuf {
        let mut name = self.path.file_name().unwrap_or_default().to_os_string();
        name.push(".");
        name.push(suffix);
        self.path.with_file_name(name)
    }
}

#[cfg(test)]
mod tests;
