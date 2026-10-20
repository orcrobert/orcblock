//! The per-domain files in `/etc/resolver` that send lookups for a domain
//! and all its subdomains to the sinkhole. Only files that start with
//! [`MARKER`] are ever rewritten or removed; an unmarked file in the way is
//! moved to a backup directory and put back when the block ends.

use std::collections::BTreeSet;
use std::fs::{self, DirBuilder};
use std::io;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use super::EnforceError;
use crate::fsutil;

/// The first line of every file this module owns.
pub(crate) const MARKER: &str = "# managed by orcblockd";

/// The content of a resolver file pointing at the sinkhole on `port`.
pub(crate) fn contents(port: u16) -> String {
    format!("{MARKER}\nnameserver 127.0.0.1\nport {port}\n")
}

/// What is in the directory now.
#[derive(Debug, Default)]
struct Listing {
    /// Marked files and whether their content is current.
    marked: Vec<(String, bool)>,
    /// Regular files without the marker.
    unmarked: BTreeSet<String>,
}

fn list(dir: &Path, port: u16) -> Result<Listing, EnforceError> {
    let mut listing = Listing::default();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(listing),
        Err(source) => return Err(io_err("list", dir, source)),
    };
    let expected = contents(port);
    for entry in entries {
        let entry = entry.map_err(|source| io_err("list", dir, source))?;
        let path = entry.path();
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let is_file = fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_file());
        if !is_file {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        if content.starts_with(MARKER) {
            listing.marked.push((name, content == expected));
        } else {
            listing.unmarked.insert(name);
        }
    }
    Ok(listing)
}

/// Makes `dir` hold a current marked file for every name in `wanted` and no
/// other marked files. Returns `true` when anything changed.
pub(crate) fn sync(
    dir: &Path,
    backup_dir: &Path,
    wanted: &BTreeSet<String>,
    port: u16,
) -> Result<bool, EnforceError> {
    let listing = list(dir, port)?;
    let mut changed = false;

    for (name, _) in listing
        .marked
        .iter()
        .filter(|(name, _)| !wanted.contains(name))
    {
        let path = dir.join(name);
        fsutil::remove_if_exists(&path).map_err(|source| io_err("remove", &path, source))?;
        changed = true;
    }

    let current: BTreeSet<&str> = listing
        .marked
        .iter()
        .filter(|(_, current)| *current)
        .map(|(name, _)| name.as_str())
        .collect();
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|name| !current.contains(name.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(changed);
    }

    make_dir(dir)?;
    let content = contents(port);
    for name in missing {
        let path = dir.join(name);
        if listing.unmarked.contains(name) {
            make_dir(backup_dir)?;
            let backup = backup_dir.join(name);
            fs::rename(&path, &backup).map_err(|source| io_err("back up", &path, source))?;
        }
        fsutil::write_atomic(&path, content.as_bytes(), 0o644)
            .map_err(|source| io_err("write", &path, source))?;
        changed = true;
    }
    Ok(changed)
}

/// True when `dir` does not hold exactly the current marked files for
/// `wanted`.
pub(crate) fn drifted(
    dir: &Path,
    wanted: &BTreeSet<String>,
    port: u16,
) -> Result<bool, EnforceError> {
    let listing = list(dir, port)?;
    let current: BTreeSet<&str> = listing
        .marked
        .iter()
        .filter(|(_, current)| *current)
        .map(|(name, _)| name.as_str())
        .collect();
    let all_marked_wanted = listing.marked.iter().all(|(name, _)| wanted.contains(name));
    Ok(!all_marked_wanted || wanted.iter().any(|name| !current.contains(name.as_str())))
}

/// Removes every marked file and restores any backed-up files. Returns
/// `true` when anything changed.
pub(crate) fn clear(dir: &Path, backup_dir: &Path, port: u16) -> Result<bool, EnforceError> {
    let changed = sync(dir, backup_dir, &BTreeSet::new(), port)?;
    let backups = match fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(changed),
        Err(source) => return Err(io_err("list", backup_dir, source)),
    };
    let mut restored = false;
    for entry in backups {
        let entry = entry.map_err(|source| io_err("list", backup_dir, source))?;
        let target = dir.join(entry.file_name());
        if target.exists() {
            continue;
        }
        fs::rename(entry.path(), &target).map_err(|source| io_err("restore", &target, source))?;
        restored = true;
    }
    Ok(changed || restored)
}

fn make_dir(dir: &Path) -> Result<(), EnforceError> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o755);
    builder
        .create(dir)
        .map_err(|source| io_err("create", dir, source))
}

fn io_err(action: &'static str, path: &Path, source: io::Error) -> EnforceError {
    EnforceError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests;
