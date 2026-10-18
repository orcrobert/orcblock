//! Installing and removing the launchd job.

use std::fs::{self, DirBuilder};
use std::io;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::warn;

use crate::{CommandRunner, EnforceError, Enforcer, LABEL, Layout, Store, StoreError, fsutil};

const LAUNCHCTL: &str = "/bin/launchctl";

/// Installing or uninstalling failed.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// A file could not be written or removed.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was being done.
        action: &'static str,
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// launchctl failed.
    #[error(transparent)]
    Program(#[from] EnforceError),
    /// The state could not be read.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Uninstalling is refused while a block runs.
    #[error("a block is running for another {}; OrcBlock can be uninstalled once it ends", format_remaining(*.0))]
    Blocking(Duration),
}

/// The launchd job definition for the helper under `layout`.
#[must_use]
pub fn plist(layout: &Layout) -> String {
    let helper = escape(&layout.helper().to_string_lossy());
    let log = escape(&layout.log_file().to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{helper}</string>
		<string>serve</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<true/>
	<key>StandardErrorPath</key>
	<string>{log}</string>
</dict>
</plist>
"#
    )
}

/// Copies `exe` to the helper path, writes the launchd job and (re)starts
/// it. Reinstalling during a block is allowed: the new daemon picks the
/// block up from the state file.
pub fn install(
    layout: &Layout,
    runner: &impl CommandRunner,
    exe: &Path,
) -> Result<(), InstallError> {
    let helper = layout.helper();
    let plist_path = layout.plist();
    for dir in [
        helper.parent(),
        plist_path.parent(),
        Some(layout.data_dir().as_path()),
    ]
    .into_iter()
    .flatten()
    {
        make_dir(dir)?;
    }
    if exe != helper {
        let bytes = fs::read(exe).map_err(io_err("read", exe))?;
        fsutil::write_atomic(&helper, &bytes, 0o755).map_err(io_err("write", &helper))?;
    }
    fsutil::write_atomic(&plist_path, plist(layout).as_bytes(), 0o644)
        .map_err(io_err("write", &plist_path))?;

    let service = format!("system/{LABEL}");
    let _ = runner.run(LAUNCHCTL, &["bootout", &service], None);
    runner.run(
        LAUNCHCTL,
        &["bootstrap", "system", &plist_path.to_string_lossy()],
        None,
    )?;
    runner.run(LAUNCHCTL, &["enable", &service], None)?;
    Ok(())
}

/// Stops and removes the helper, refusing while a block runs. The state
/// file stays, so reinstalling does not refill sleep potions.
pub fn uninstall(
    layout: &Layout,
    runner: &impl CommandRunner,
    enforcer: &mut impl Enforcer,
) -> Result<(), InstallError> {
    if let Some(saved) = Store::new(layout.state_file()).load()?
        && let Some(session) = saved.state.session()
    {
        return Err(InstallError::Blocking(session.remaining()));
    }
    let _ = runner.run(LAUNCHCTL, &["bootout", &format!("system/{LABEL}")], None);
    if let Err(err) = enforcer.clear() {
        warn!(%err, "cleanup after stopping the helper was incomplete");
    }
    for path in [layout.plist(), layout.helper(), layout.socket()] {
        fsutil::remove_if_exists(&path).map_err(io_err("remove", &path))?;
    }
    Ok(())
}

fn make_dir(dir: &Path) -> Result<(), InstallError> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o755);
    builder.create(dir).map_err(io_err("create", dir))
}

fn io_err(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> InstallError {
    let path = path.to_path_buf();
    move |source| InstallError::Io {
        action,
        path,
        source,
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_remaining(remaining: Duration) -> String {
    let minutes = remaining.as_secs().div_ceil(60);
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests;
