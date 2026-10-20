//! File helpers shared by the store and the enforcer.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Replaces `path` with `bytes`: written to a temporary sibling, synced,
/// renamed over the old file and the directory synced, so a crash leaves
/// either the old content or the new. On Unix the file ends up with
/// `mode`; on Windows it inherits its directory's ACL and `mode` is unused.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let tmp = sibling(path, "tmp");
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(mode);
    let mut file = options.open(&tmp)?;
    #[cfg(unix)]
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    #[cfg(windows)]
    let _ = mode;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    // Windows cannot open a directory as a file to sync it; NTFS journals
    // the rename itself.
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        fs::File::open(dir)?.sync_all()?;
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
