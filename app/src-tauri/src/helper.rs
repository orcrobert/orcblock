//! Installing the bundled daemon with administrator rights.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// SHA-256 of the daemon bundled with this build, or empty without one.
pub(crate) const HELPER_SHA256: &str = env!("ORCBLOCK_HELPER_SHA256");

/// Why installing failed; `Display` is worded for the user.
#[derive(Debug)]
pub(crate) enum InstallError {
    /// The user dismissed the administrator prompt.
    Cancelled,
    /// The prompt could not be shown.
    NoPrompt(std::io::Error),
    /// The install ran and failed, with its error output.
    Failed(String),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("Installation cancelled"),
            Self::NoPrompt(err) => {
                write!(f, "Could not ask for the administrator password: {err}")
            }
            Self::Failed(reason) => write!(f, "Installation failed: {reason}"),
        }
    }
}

/// The daemon shipped next to the app's executable.
pub(crate) fn bundled() -> Option<PathBuf> {
    let name = format!("orcblockd{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|path| path.is_file())
}

/// Runs `helper install` as root through the macOS administrator prompt.
pub(crate) fn install(helper: &Path) -> Result<(), InstallError> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &install_script(helper, HELPER_SHA256)])
        .output()
        .map_err(InstallError::NoPrompt)?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(if stderr.contains("-128") {
        InstallError::Cancelled
    } else {
        InstallError::Failed(stderr.trim().to_owned())
    })
}

/// The `AppleScript` that installs the helper as root. The bundle is
/// writable by the user, so root first copies the helper into its own
/// temp dir, checks the copy against `sha256` and runs only that copy:
/// nothing swapped in before or during the password prompt gets root.
fn install_script(helper: &Path, sha256: &str) -> String {
    let quote = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    let (before, after) = install_shell(sha256);
    format!(
        "do shell script \"{}\" & quoted form of \"{}\" & \"{}\" with prompt \"OrcBlock needs to install its helper to keep blocks in place.\" with administrator privileges",
        quote(before),
        quote(&helper.to_string_lossy()),
        quote(&after),
    )
}

/// The shell that runs as root, split around the single-quoted helper path.
fn install_shell(sha256: &str) -> (&'static str, String) {
    let before = "t=$(/usr/bin/mktemp -d) || exit 1; /bin/cp -- ";
    let after = format!(
        " \"$t/orcblockd\" || {{ /bin/rm -rf \"$t\"; exit 1; }}; \
         if [ \"$(/usr/bin/shasum -a 256 \"$t/orcblockd\" | /usr/bin/cut -c1-64)\" != \"{sha256}\" ]; then \
         /bin/rm -rf \"$t\"; echo \"the helper does not match this copy of OrcBlock\" >&2; exit 1; fi; \
         \"$t/orcblockd\" install; s=$?; /bin/rm -rf \"$t\"; exit $s"
    );
    (before, after)
}

#[cfg(test)]
mod tests;
