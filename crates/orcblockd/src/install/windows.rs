//! Installing and removing the daemon on Windows: a scheduled task running
//! the helper as SYSTEM at boot and every 5 minutes if it is not running
//! (ADR 0005), and a data directory only SYSTEM and Administrators can
//! change.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use tracing::warn;

use super::{InstallError, io_err, make_dir, refuse_during_block};
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

/// How long to wait for a stopped helper to let go of its file.
const COPY_ATTEMPTS: u32 = 20;
const COPY_RETRY: Duration = Duration::from_millis(250);

/// Whether `whoami /groups` output lists an elevated integrity label.
#[must_use]
pub fn is_elevated(groups: &str) -> bool {
    ELEVATED_LABELS.iter().any(|label| groups.contains(label))
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
    let data_dir = layout.data_dir();
    make_dir(&data_dir)?;
    lock_down(runner, &data_dir)?;

    let helper = layout.helper();
    if let Some(dir) = helper.parent() {
        make_dir(dir)?;
    }
    // A running helper keeps its executable open; stop it before replacing.
    let _ = runner.run(SCHTASKS, &["/End", "/TN", TASK_NAME], None);
    if exe != helper {
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
    refuse_during_block(layout)?;
    let _ = runner.run(SCHTASKS, &["/End", "/TN", TASK_NAME], None);
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

/// Makes SYSTEM the owner of `dir` and everything in it, with full control
/// for SYSTEM and Administrators and read-only access for Users, so a
/// directory a user created before the install does not stay theirs.
fn lock_down(runner: &impl CommandRunner, dir: &Path) -> Result<(), InstallError> {
    let dir = dir.to_string_lossy();
    runner.run(ICACLS, &[&dir, "/setowner", SYSTEM, "/T", "/C", "/Q"], None)?;
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
            "/T",
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
