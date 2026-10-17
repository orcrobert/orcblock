use std::path::{Path, PathBuf};

/// The launchd label, also the helper's file name.
pub const LABEL: &str = "com.orcrobert.orcblockd";

/// Where everything the daemon installs or touches lives, relative to a
/// root that is `/` on a real Mac and a temporary directory in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// The real system paths.
    #[must_use]
    pub fn system() -> Self {
        Self::under("/")
    }

    /// The same paths under `root`.
    pub fn under(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn join(&self, path: &str) -> PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }

    /// The installed daemon binary.
    #[must_use]
    pub fn helper(&self) -> PathBuf {
        self.join("/Library/PrivilegedHelperTools").join(LABEL)
    }

    /// The launchd job.
    #[must_use]
    pub fn plist(&self) -> PathBuf {
        self.join("/Library/LaunchDaemons")
            .join(format!("{LABEL}.plist"))
    }

    /// The daemon's own files: state, pf token, resolver backups.
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        self.join("/Library/Application Support/OrcBlock")
    }

    /// The state file.
    #[must_use]
    pub fn state_file(&self) -> PathBuf {
        self.data_dir().join("state.json")
    }

    /// The control socket.
    #[must_use]
    pub fn socket(&self) -> PathBuf {
        self.join(orcblock_ipc::DEFAULT_SOCKET)
    }

    /// Per-domain resolver configuration read by mDNSResponder.
    #[must_use]
    pub fn resolver_dir(&self) -> PathBuf {
        self.join("/etc/resolver")
    }

    /// The daemon's log, written by launchd.
    #[must_use]
    pub fn log_file(&self) -> PathBuf {
        self.join("/var/log/orcblockd.log")
    }

    /// The root everything is relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}
