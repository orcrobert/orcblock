//! Installing and removing the daemon on Windows: a scheduled task running
//! the helper as SYSTEM at boot and every 5 minutes if it is not running
//! (ADR 0005), and a data directory only SYSTEM and Administrators can
//! change.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use jiff::Timestamp;
use tracing::warn;

use super::{InstallError, io_err, make_dir, refuse_during_block};
use crate::enforce::windows::powershell;
use crate::{CommandRunner, Enforcer, Layout, TASK_NAME, fsutil};

const SCHTASKS: &str = r"C:\Windows\System32\schtasks.exe";
const ICACLS: &str = r"C:\Windows\System32\icacls.exe";
const WHOAMI: &str = r"C:\Windows\System32\whoami.exe";

/// Mandatory integrity labels of an elevated administrator and of SYSTEM.
/// SIDs, unlike group names, are the same in every language.
const ELEVATED_LABELS: [&str; 2] = ["S-1-16-12288", "S-1-16-16384"];

/// Well-known SIDs for the data directory's ACL.
const SYSTEM: &str = "*S-1-5-18";
const ADMINISTRATORS: &str = "*S-1-5-32-544";
const USERS: &str = "*S-1-5-32-545";

/// SDDL names for the owners a locked data directory may have.
const TRUSTED_OWNERS: [&str; 4] = ["SY", "BA", "S-1-5-18", "S-1-5-32-544"];

/// SDDL access rights that only read and list, as icacls writes `RX`.
const READ_ONLY_RIGHTS: [&str; 5] = ["0x1200a9", "0x120089", "FR", "GR", "GRGX"];

/// Prints the SDDL of the path on the first line of stdin.
const ACL_SCRIPT: &str = r"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
(Get-Acl -LiteralPath @($input)[0]).Sddl
";

/// How long to wait for a stopped helper to let go of its file.
const COPY_ATTEMPTS: u32 = 20;
const COPY_RETRY: Duration = Duration::from_millis(250);

/// Whether `whoami /groups` output lists an elevated integrity label.
#[must_use]
pub fn is_elevated(groups: &str) -> bool {
    ELEVATED_LABELS
        .iter()
        .any(|label| groups.contains(&format!("\"{label}\"")))
}

/// Fails with [`InstallError::NotElevated`] unless this process runs
/// elevated or as SYSTEM.
pub fn require_elevation(runner: &impl CommandRunner) -> Result<(), InstallError> {
    let output = runner.run(WHOAMI, &["/groups", "/fo", "csv", "/nh"], None)?;
    if is_elevated(&output.stdout) {
        Ok(())
    } else {
        Err(InstallError::NotElevated)
    }
}

/// Whether `sddl` describes a directory owned by SYSTEM or Administrators,
/// protected from inheritance, that nobody else may write to.
#[must_use]
pub fn is_locked_sddl(sddl: &str) -> bool {
    let Some(rest) = sddl.trim().strip_prefix("O:") else {
        return false;
    };
    let owner_end = ["G:", "D:"]
        .iter()
        .filter_map(|tag| rest.find(tag))
        .min()
        .unwrap_or(rest.len());
    if !TRUSTED_OWNERS.contains(&&rest[..owner_end]) {
        return false;
    }
    let Some(start) = rest.find("D:") else {
        return false;
    };
    let dacl = &rest[start + 2..];
    let dacl = dacl.find("S:").map_or(dacl, |end| &dacl[..end]);
    let (flags, aces) = dacl.split_at(dacl.find('(').unwrap_or(dacl.len()));
    flags.contains('P')
        && aces
            .split(')')
            .filter(|ace| !ace.is_empty())
            .all(|ace| ace_is_safe(ace.trim_start_matches('(')))
}

fn ace_is_safe(ace: &str) -> bool {
    let [kind, flags, rights, _, _, sid] = ace.split(';').collect::<Vec<_>>()[..] else {
        return false;
    };
    if flags.contains("ID") {
        return false;
    }
    match kind {
        "D" => true,
        "A" => TRUSTED_OWNERS.contains(&sid) || READ_ONLY_RIGHTS.contains(&rights),
        _ => false,
    }
}

/// Whether `dir` is a real directory locked down as [`install`] leaves it.
pub fn is_locked_dir(runner: &impl CommandRunner, dir: &Path) -> bool {
    let real_dir =
        fs::symlink_metadata(dir).is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink());
    real_dir
        && powershell(runner, ACL_SCRIPT, Some(&format!("{}\n", dir.display())))
            .is_ok_and(|sddl| is_locked_sddl(&sddl))
}

/// The task definition for the helper under `layout`.
#[must_use]
pub fn task_xml(layout: &Layout) -> String {
    let helper = escape(&layout.helper().to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Keeps OrcBlock blocks in place whether or not the app is running.</Description>
    <URI>{TASK_NAME}</URI>
  </RegistrationInfo>
  <Triggers>
    <BootTrigger>
      <Enabled>true</Enabled>
    </BootTrigger>
    <TimeTrigger>
      <StartBoundary>2026-01-01T00:00:00</StartBoundary>
      <Enabled>true</Enabled>
      <Repetition>
        <Interval>PT5M</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="System">
      <UserId>S-1-5-18</UserId>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>999</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="System">
    <Exec>
      <Command>{helper}</Command>
      <Arguments>serve</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

/// `text` as UTF-16LE with a byte order mark, the encoding `schtasks /XML`
/// reads reliably.
#[must_use]
pub fn utf16_with_bom(text: &str) -> Vec<u8> {
    [0xFEFF_u16]
        .into_iter()
        .chain(text.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Locks the data directory down, copies `exe` to the helper path, and
/// registers and starts the task. Reinstalling during a block is allowed:
/// the new daemon picks the block up from the state file.
pub fn install(
    layout: &Layout,
    runner: &impl CommandRunner,
    exe: &Path,
) -> Result<(), InstallError> {
    require_elevation(runner)?;
    prepare_data_dir(runner, &layout.data_dir())?;

    let helper = layout.helper();
    if let Some(dir) = helper.parent() {
        make_dir(dir)?;
    }
    // A running helper keeps its executable open; stop it before replacing.
    let _ = runner.run(SCHTASKS, &["/End", "/TN", TASK_NAME], None);
    // Reinstalling from the installed helper itself copies nothing; paths
    // differ only in case or form there, so compare them resolved.
    let same = matches!(
        (fs::canonicalize(exe), fs::canonicalize(&helper)),
        (Ok(exe), Ok(helper)) if exe == helper
    );
    if !same {
        let bytes = fs::read(exe).map_err(io_err("read", exe))?;
        copy_with_retries(&helper, &bytes)?;
    }

    let xml_path = layout.plist();
    fsutil::write_atomic(&xml_path, &utf16_with_bom(&task_xml(layout)), 0o644)
        .map_err(io_err("write", &xml_path))?;
    let xml = xml_path.to_string_lossy();
    runner.run(
        SCHTASKS,
        &["/Create", "/TN", TASK_NAME, "/XML", &xml, "/F"],
        None,
    )?;
    runner.run(SCHTASKS, &["/Run", "/TN", TASK_NAME], None)?;
    Ok(())
}

/// Stops and removes the task and the helper, refusing while a block runs.
/// The state file stays, so reinstalling does not refill sleep potions.
pub fn uninstall(
    layout: &Layout,
    runner: &impl CommandRunner,
    enforcer: &mut impl Enforcer,
) -> Result<(), InstallError> {
    require_elevation(runner)?;
    // Stopped first, so no block can start between the check and the end.
    let _ = runner.run(SCHTASKS, &["/End", "/TN", TASK_NAME], None);
    if let Err(err) = refuse_during_block(layout) {
        let _ = runner.run(SCHTASKS, &["/Run", "/TN", TASK_NAME], None);
        return Err(err);
    }
    let _ = runner.run(SCHTASKS, &["/Delete", "/TN", TASK_NAME, "/F"], None);
    if let Err(err) = enforcer.clear() {
        warn!(%err, "cleanup after stopping the helper was incomplete");
    }
    let helper = layout.helper();
    for path in [helper.clone(), layout.plist()] {
        fsutil::remove_if_exists(&path).map_err(io_err("remove", &path))?;
    }
    if let Some(dir) = helper.parent() {
        // Only if empty; anything else in there is not ours to delete.
        let _ = fs::remove_dir(dir);
    }
    Ok(())
}

/// Leaves `dir` locked down and holding only what the daemon wrote there.
/// Any user may create a directory in `ProgramData`, so one that is a link or
/// that is not locked is moved aside instead of repaired: anything planted
/// inside could be a link that turns the daemon's writes elsewhere.
fn prepare_data_dir(runner: &impl CommandRunner, dir: &Path) -> Result<(), InstallError> {
    if fs::symlink_metadata(dir).is_ok() {
        if is_locked_dir(runner, dir) {
            return lock_down(runner, dir);
        }
        let aside = set_aside(dir)?;
        warn!(path = %aside.display(), "moved an unlocked data directory aside");
    }
    if let Some(parent) = dir.parent() {
        make_dir(parent)?;
    }
    // create_dir, not create_dir_all: a directory someone made in between
    // must not be taken over.
    fs::create_dir(dir).map_err(io_err("create", dir))?;
    lock_down(runner, dir)?;
    // A new directory in `ProgramData` lets users add files until it is
    // locked.
    if fs::read_dir(dir)
        .map_err(io_err("read", dir))?
        .next()
        .is_some()
    {
        return Err(InstallError::Tampered(set_aside(dir)?));
    }
    Ok(())
}

/// Renames `dir` to a free `<name>.untrusted-<seconds>` next to it.
fn set_aside(dir: &Path) -> Result<std::path::PathBuf, InstallError> {
    let name = dir
        .file_name()
        .map_or_else(Default::default, |name| name.to_string_lossy());
    let stamp = Timestamp::now().as_second();
    let mut attempt = 0;
    loop {
        let aside = dir.with_file_name(format!("{name}.untrusted-{stamp}-{attempt}"));
        if fs::symlink_metadata(&aside).is_err() {
            fs::rename(dir, &aside).map_err(io_err("move aside", dir))?;
            return Ok(aside);
        }
        attempt += 1;
    }
}

/// Makes SYSTEM the owner of `dir`, with full control for SYSTEM and
/// Administrators and read-only access for Users, inherited by what the
/// daemon creates inside. Not recursive: a link inside must not carry the
/// change elsewhere.
fn lock_down(runner: &impl CommandRunner, dir: &Path) -> Result<(), InstallError> {
    let dir = dir.to_string_lossy();
    runner.run(ICACLS, &[&dir, "/setowner", SYSTEM, "/C", "/Q"], None)?;
    let system = format!("{SYSTEM}:(OI)(CI)F");
    let administrators = format!("{ADMINISTRATORS}:(OI)(CI)F");
    let users = format!("{USERS}:(OI)(CI)RX");
    runner.run(
        ICACLS,
        &[
            &dir,
            "/inheritance:r",
            "/grant:r",
            &system,
            &administrators,
            &users,
            "/C",
            "/Q",
        ],
        None,
    )?;
    Ok(())
}

fn copy_with_retries(helper: &Path, bytes: &[u8]) -> Result<(), InstallError> {
    let mut attempt = 1;
    loop {
        match fsutil::write_atomic(helper, bytes, 0o755) {
            Ok(()) => return Ok(()),
            Err(err) if attempt < COPY_ATTEMPTS => {
                warn!(%err, attempt, "the helper is still in use; retrying");
                attempt += 1;
                thread::sleep(COPY_RETRY);
            }
            Err(err) => return Err(io_err("write", helper)(err)),
        }
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests;
