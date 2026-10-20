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
#[cfg(not(windows))]
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

/// Runs `helper install` elevated through the UAC prompt.
#[cfg(windows)]
pub(crate) fn install(helper: &Path) -> Result<(), InstallError> {
    use std::io::Read as _;
    use std::os::windows::fs::OpenOptionsExt as _;
    use std::os::windows::process::CommandExt as _;

    // Starts PowerShell without flashing a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // Others may read and run the file, but not write, rename or delete it.
    const FILE_SHARE_READ: u32 = 0x1;

    let failed = |err: std::io::Error| InstallError::Failed(err.to_string());
    // Held until the elevated install exits, so the file UAC runs is the
    // one whose hash was checked, even if the app sits in a folder the
    // user can write to.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(helper)
        .map_err(failed)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(failed)?;
    if sha256_hex(&bytes) != HELPER_SHA256 {
        return Err(InstallError::Failed(
            "the helper does not match this copy of OrcBlock".into(),
        ));
    }
    let output = Command::new(POWERSHELL)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            encode_command(ELEVATE_SCRIPT).as_str(),
        ])
        .env("ORCBLOCK_HELPER", helper)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(InstallError::NoPrompt)?;
    drop(file);
    outcome(
        output.status.code(),
        &String::from_utf8_lossy(&output.stderr),
    )
}

#[cfg(any(windows, test))]
const POWERSHELL: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";

/// What Windows returns when the user declines the UAC prompt.
#[cfg(any(windows, test))]
const ERROR_CANCELLED: i32 = 1223;

/// Starts the helper in `$env:ORCBLOCK_HELPER` with `install` through UAC,
/// waits, and exits with its exit code, or 1223 when the prompt was
/// declined. The path comes from the environment so no script is ever
/// built from it. Reading `Handle` right away keeps the exit code of an
/// elevated process, which PowerShell 5.1 otherwise reports as `$null`.
#[cfg(any(windows, test))]
const ELEVATE_SCRIPT: &str = r"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
try {
    $process = Start-Process -FilePath $env:ORCBLOCK_HELPER -ArgumentList 'install' -Verb RunAs -WindowStyle Hidden -PassThru
} catch {
    if ($_.Exception.NativeErrorCode -eq 1223 -or $_.Exception.InnerException.NativeErrorCode -eq 1223) { exit 1223 }
    throw
}
$null = $process.Handle
$process.WaitForExit()
if ($null -eq $process.ExitCode) { exit 1 }
exit $process.ExitCode
";

/// Maps the elevation script's exit code and error output to a result.
#[cfg(any(windows, test))]
fn outcome(code: Option<i32>, stderr: &str) -> Result<(), InstallError> {
    match code {
        Some(0) => Ok(()),
        Some(ERROR_CANCELLED) => Err(InstallError::Cancelled),
        code => {
            let reason = match (stderr.trim(), code) {
                ("", Some(code)) => format!("the helper exited with code {code}"),
                ("", None) => "the helper was stopped".to_owned(),
                (stderr, _) => stderr.to_owned(),
            };
            Err(InstallError::Failed(format!(
                "{reason}; see C:\\ProgramData\\OrcBlock\\orcblockd.log"
            )))
        }
    }
}

/// A script as `-EncodedCommand` expects it: base64 of UTF-16LE.
#[cfg(any(windows, test))]
fn encode_command(script: &str) -> String {
    use base64::Engine as _;
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64::engine::general_purpose::STANDARD.encode(utf16)
}

#[cfg(any(windows, test))]
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// The `AppleScript` that installs the helper as root. The bundle is
/// writable by the user, so root first copies the helper into its own
/// temp dir, checks the copy against `sha256` and runs only that copy:
/// nothing swapped in before or during the password prompt gets root.
#[cfg(not(windows))]
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
#[cfg(not(windows))]
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
