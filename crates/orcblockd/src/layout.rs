use std::path::{Path, PathBuf};

/// The launchd label, also the helper's file name on macOS.
pub const LABEL: &str = "com.orcrobert.orcblockd";

/// The scheduled task that runs the daemon on Windows.
pub const TASK_NAME: &str = r"\OrcBlock\orcblockd";

/// Which system's paths a [`Layout`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// launchd, `/Library`, `/etc/resolver`.
    Mac,
    /// Task Scheduler, `Program Files`, `ProgramData`.
    Windows,
}

/// Where everything the daemon installs or touches lives, relative to a
/// root that is `/` on a real Mac, `C:\` on a real Windows PC and a
/// temporary directory in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
    platform: Platform,
}

impl Layout {
    /// The real system paths.
    #[must_use]
    pub fn system() -> Self {
        #[cfg(windows)]
        return Self::windows_under(r"C:\");
        #[cfg(not(windows))]
        return Self::under("/");
    }

    /// The macOS paths under `root`.
    pub fn under(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            platform: Platform::Mac,
        }
    }

    /// The Windows paths under `root`.
    pub fn windows_under(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            platform: Platform::Windows,
        }
    }

    /// Which system these paths are for.
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    fn join(&self, path: &str) -> PathBuf {
        path.trim_start_matches('/')
            .split('/')
            .fold(self.root.clone(), |dir, part| dir.join(part))
    }

    /// The installed daemon binary.
    #[must_use]
    pub fn helper(&self) -> PathBuf {
        match self.platform {
            Platform::Mac => self.join("/Library/PrivilegedHelperTools").join(LABEL),
            Platform::Windows => self.join("/Program Files/OrcBlock Helper/orcblockd.exe"),
        }
    }

    /// The service definition: the launchd plist, or the task XML that
    /// `schtasks` imports.
    #[must_use]
    pub fn plist(&self) -> PathBuf {
        match self.platform {
            Platform::Mac => self
                .join("/Library/LaunchDaemons")
                .join(format!("{LABEL}.plist")),
            Platform::Windows => self.data_dir().join("orcblockd-task.xml"),
        }
    }

    /// The daemon's own files: state, log, pf token, resolver backups.
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        match self.platform {
            Platform::Mac => self.join("/Library/Application Support/OrcBlock"),
            Platform::Windows => self.join("/ProgramData/OrcBlock"),
        }
    }

    /// The state file.
    #[must_use]
    pub fn state_file(&self) -> PathBuf {
        self.data_dir().join("state.json")
    }

    /// The control socket on macOS. Windows listens on
    /// [`orcblock_ipc::DEFAULT_SOCKET`] instead and has no socket file.
    #[must_use]
    pub fn socket(&self) -> PathBuf {
        self.join("/var/run/orcblockd.sock")
    }

    /// Per-domain resolver configuration read by mDNSResponder.
    #[must_use]
    pub fn resolver_dir(&self) -> PathBuf {
        self.join("/etc/resolver")
    }

    /// The daemon's log: written by launchd on macOS, by the daemon itself
    /// on Windows.
    #[must_use]
    pub fn log_file(&self) -> PathBuf {
        match self.platform {
            Platform::Mac => self.join("/var/log/orcblockd.log"),
            Platform::Windows => self.data_dir().join("orcblockd.log"),
        }
    }

    /// The root everything is relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests;
