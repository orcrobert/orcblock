//! File helpers shared by the store and the enforcer.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Replaces `path` with `bytes`: written to a temporary sibling, synced,
/// renamed over the old file and the directory synced, so a crash leaves
/// either the old content or the new. The file ends up with `mode`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let tmp = sibling(path, "tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(&tmp)?;
    file.set_permissions(Permissions::from_mode(mode))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    if let Some(dir) = path.parent() {
        File::open(dir)?.sync_all()?;
    }
    Ok(())
}

/// `path` with `.suffix` appended to its file name.
pub(crate) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

/// Removes `path`, treating "already gone" as success.
pub(crate) fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}
