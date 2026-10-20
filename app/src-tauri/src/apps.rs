//! Apps for the picker: the installed ones in the usual folders, one the
//! user chose or dropped, which of them are open, and their icons.
//!
//! Bundles are read with `plutil` and icons converted with `sips`, both run
//! by absolute path without a shell. Only the daemon decides what a bundle
//! ID blocks; this side just finds IDs and names to send it.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use orcblock_core::{AppName, BundleId, outermost_bundle};
use serde::Serialize;
use serde_json::Value;

use crate::favicon::{hex_digest, write_atomic};

const PLUTIL: &str = "/usr/bin/plutil";
const SIPS: &str = "/usr/bin/sips";
/// `ps` runs through `env` for a UTF-8 locale; in the empty environment
/// it escapes non-ASCII bytes and apps with such names look closed.
const ENV: &str = "/usr/bin/env";
const PS_ARGS: [&str; 5] = ["LC_CTYPE=UTF-8", "/bin/ps", "-axww", "-o", "comm="];
const OSASCRIPT: &str = "/usr/bin/osascript";

/// Constant: nothing from the user or the webview is ever put into it.
const CHOOSE_APP: &str = r#"POSIX path of (choose file of type {"com.apple.application-bundle"} with prompt "Choose an app to block" default location (path to applications folder))"#;

const ICON_PX: &str = "64";
const SCAN_THREADS: usize = 8;

/// An app the user can add.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct InstalledApp {
    /// The lowercase bundle ID, as the daemon stores it.
    pub(crate) bundle_id: String,
    pub(crate) name: String,
    pub(crate) path: String,
}

/// Runs a program and returns its standard output, or `None` when it could
/// not start or exited non-zero.
pub(crate) trait Runner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Option<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(program)
            .args(args)
            .env_clear()
            .stdin(Stdio::null())
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Finds apps and caches what it found for the session.
pub(crate) struct Apps<R> {
    runner: R,
    folders: Vec<PathBuf>,
    icon_dir: PathBuf,
    scanned: Mutex<Option<Vec<InstalledApp>>>,
    /// Apps chosen or dropped from outside the scanned folders, by ID.
    extra: Mutex<HashMap<String, InstalledApp>>,
    writes: AtomicU64,
}

impl<R> std::fmt::Debug for Apps<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Apps")
            .field("folders", &self.folders)
            .finish_non_exhaustive()
    }
}

/// Where macOS keeps apps: the system's, the admin-installed ones and the
/// user's own.
pub(crate) fn app_folders(home: Option<&Path>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = [
        "/Applications",
        "/Applications/Utilities",
        "/System/Applications",
        "/System/Applications/Utilities",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    folders.extend(home.map(|home| home.join("Applications")));
    folders
}

impl<R: Runner> Apps<R> {
    pub(crate) fn new(runner: R, folders: Vec<PathBuf>, icon_dir: PathBuf) -> Self {
        Self {
            runner,
            folders,
            icon_dir,
            scanned: Mutex::new(None),
            extra: Mutex::new(HashMap::new()),
            writes: AtomicU64::new(0),
        }
    }

    /// Every app found, sorted by name; scanned once per session.
    pub(crate) fn installed(&self) -> Vec<InstalledApp> {
        let mut scanned = self.scanned.lock().unwrap_or_else(PoisonError::into_inner);
        let apps = scanned.get_or_insert_with(|| self.scan());
        let mut all = apps.clone();
        let extra = self.extra.lock().unwrap_or_else(PoisonError::into_inner);
        all.extend(
            extra
                .values()
                .filter(|app| !apps.iter().any(|a| a.bundle_id == app.bundle_id))
                .cloned(),
        );
        sort_by_name(&mut all);
        all
    }

    /// The app at `path`, if it is an app bundle with a valid bundle ID.
    /// It is remembered, so its icon can be found later.
    pub(crate) fn at_path(&self, path: &Path) -> Option<InstalledApp> {
        let app = self.read(path)?;
        self.extra
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(app.bundle_id.clone(), app.clone());
        Some(app)
    }

    /// Asks the user to pick an app in the standard macOS file dialog;
    /// `None` when they cancel.
    pub(crate) fn pick(&self) -> Option<InstalledApp> {
        let chosen = self.runner.run(OSASCRIPT, &["-e", CHOOSE_APP])?;
        let path = chosen.trim().trim_end_matches('/');
        self.at_path(Path::new(path))
    }

    /// The bundle IDs of apps that are open now.
    pub(crate) fn running(&self) -> Vec<String> {
        let Some(table) = self.runner.run(ENV, &PS_ARGS) else {
            return Vec::new();
        };
        let known: HashMap<String, String> = self
            .installed()
            .into_iter()
            .map(|app| (app.path, app.bundle_id))
            .collect();
        let mut bundles: Vec<&str> = table
            .lines()
            .filter_map(|l| outermost_bundle(l.trim()))
            .collect();
        bundles.sort_unstable();
        bundles.dedup();
        let mut ids: Vec<String> = bundles
            .into_iter()
            .filter_map(|bundle| {
                known
                    .get(bundle)
                    .cloned()
                    .or_else(|| self.read(Path::new(bundle)).map(|app| app.bundle_id))
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// The app's icon as a PNG data URL, or `None` for a monogram.
    pub(crate) fn icon(&self, bundle_id: &str) -> Option<String> {
        let bundle_id = BundleId::parse(bundle_id).ok()?;
        let app = self.find(bundle_id.as_str())?;
        let cached = self
            .icon_dir
            .join(format!("{}.png", hex_digest(app.path.as_bytes())));
        if let Ok(bytes) = fs::read(&cached) {
            return Some(png_data_url(&bytes));
        }
        let source = self.icon_file(Path::new(&app.path))?;
        let temp = self.icon_dir.join(format!(
            ".{}.{}.png",
            std::process::id(),
            self.writes.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&self.icon_dir).ok()?;
        let converted = self.runner.run(
            SIPS,
            &[
                "-s",
                "format",
                "png",
                "-Z",
                ICON_PX,
                &source.to_string_lossy(),
                "--out",
                &temp.to_string_lossy(),
            ],
        );
        let bytes = converted.and_then(|_| fs::read(&temp).ok());
        let _ = fs::remove_file(&temp);
        let bytes = bytes.filter(|bytes| bytes.starts_with(b"\x89PNG"))?;
        let _ = write_atomic(&temp, &cached, &bytes);
        Some(png_data_url(&bytes))
    }

    /// The app with `bundle_id`, without copying the whole list.
    fn find(&self, bundle_id: &str) -> Option<InstalledApp> {
        let mut scanned = self.scanned.lock().unwrap_or_else(PoisonError::into_inner);
        let apps = scanned.get_or_insert_with(|| self.scan());
        if let Some(app) = apps.iter().find(|app| app.bundle_id == bundle_id) {
            return Some(app.clone());
        }
        drop(scanned);
        self.extra
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(bundle_id)
            .cloned()
    }

    fn scan(&self) -> Vec<InstalledApp> {
        let bundles: Vec<PathBuf> = self
            .folders
            .iter()
            .filter_map(|folder| fs::read_dir(folder).ok())
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_app_path(path))
            .collect();
        let chunk = bundles.len().div_ceil(SCAN_THREADS).max(1);
        let mut apps: Vec<InstalledApp> = thread::scope(|scope| {
            let workers: Vec<_> = bundles
                .chunks(chunk)
                .map(|paths| {
                    scope.spawn(move || {
                        paths
                            .iter()
                            .filter_map(|path| self.read(path))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .filter_map(|worker| worker.join().ok())
                .flatten()
                .collect()
        });
        // The chunks are contiguous and joined in order, so the scan keeps
        // folder order and the first folder wins for an app installed twice.
        let mut seen = std::collections::HashSet::new();
        apps.retain(|app| seen.insert(app.bundle_id.clone()));
        sort_by_name(&mut apps);
        apps
    }

    fn info(&self, bundle: &Path) -> Option<Value> {
        let plist = bundle.join("Contents/Info.plist");
        let json = self.runner.run(
            PLUTIL,
            &["-convert", "json", "-o", "-", &plist.to_string_lossy()],
        )?;
        serde_json::from_str(&json).ok()
    }

    fn read(&self, bundle: &Path) -> Option<InstalledApp> {
        if !is_app_path(bundle) || !bundle.is_dir() {
            return None;
        }
        let info = self.info(bundle);
        let field = |key: &str| {
            info.as_ref()
                .and_then(|info| info.get(key))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    // Plists with dates or data have no JSON form; ask for
                    // the one key instead.
                    let plist = bundle.join("Contents/Info.plist");
                    self.runner.run(
                        PLUTIL,
                        &["-extract", key, "raw", "-o", "-", &plist.to_string_lossy()],
                    )
                })
        };
        let bundle_id = BundleId::parse(&field("CFBundleIdentifier")?).ok()?;
        let stem = bundle
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned());
        let name = ["CFBundleDisplayName", "CFBundleName"]
            .into_iter()
            .filter_map(|key| info.as_ref()?.get(key)?.as_str().map(str::to_owned))
            .chain(stem)
            .find_map(|name| AppName::parse(&name).ok())
            .unwrap_or_else(|| AppName::from_bundle_id(&bundle_id));
        Some(InstalledApp {
            bundle_id: bundle_id.as_str().to_owned(),
            name: name.as_str().to_owned(),
            path: bundle.to_string_lossy().into_owned(),
        })
    }

    fn icon_file(&self, bundle: &Path) -> Option<PathBuf> {
        let info = self.info(bundle)?;
        let file = info.get("CFBundleIconFile")?.as_str()?;
        // A plain file name inside Resources; anything with a path in it is
        // not followed.
        if file.is_empty() || file.contains('/') || file.starts_with('.') {
            return None;
        }
        let file = if Path::new(file).extension().is_some() {
            file.to_owned()
        } else {
            format!("{file}.icns")
        };
        let path = bundle.join("Contents/Resources").join(file);
        path.is_file().then_some(path)
    }
}

/// An absolute path whose last component is `<name>.app`.
fn is_app_path(path: &Path) -> bool {
    path.is_absolute()
        && path.file_name().is_some_and(|name| {
            let name = name.as_encoded_bytes();
            name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(b".app")
        })
}

fn sort_by_name(apps: &mut [InstalledApp]) {
    apps.sort_by_cached_key(|app| (app.name.to_lowercase(), app.bundle_id.clone()));
}

fn png_data_url(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests;
