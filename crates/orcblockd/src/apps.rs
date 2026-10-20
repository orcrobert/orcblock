//! Closes blocked apps while a block runs.
//!
//! The daemon polls the process table (ADR 0006): a process belongs to every
//! `.app` bundle in its executable path, a bundle's ID comes from its
//! `Info.plist`, and a process of a blocked app gets SIGTERM, then SIGKILL if
//! it is still there [`KILL_AFTER`] later.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::time::{Duration, Instant};

use jiff::Timestamp;
use nix::fcntl::OFlag;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use orcblock_core::{AppName, BundleId, bundles};
use orcblock_ipc::BlockedLaunch;
use tracing::{debug, info, warn};

use crate::CommandRunner;
use crate::wake::{APP_BUNDLE_ID, Waker};

/// `ps` runs through `env` for a UTF-8 locale: with the empty environment
/// every program gets, it escapes non-ASCII bytes in paths, and an app
/// renamed `Slack ü.app` would never be matched.
const PS: &str = "/usr/bin/env";
const PS_ARGS: &[&str] = &[
    "LC_CTYPE=UTF-8",
    "/bin/ps",
    "-axww",
    "-o",
    "pid=,uid=,comm=",
];
const PLUTIL: &str = "/usr/bin/plutil";

/// How long an app gets to quit after SIGTERM before it is killed.
pub const KILL_AFTER: Duration = Duration::from_secs(3);

/// How many recent blocked launches [`AppGuard::launches`] keeps.
pub const LAUNCHES_KEPT: usize = 8;

/// One app opened twice within this window is one blocked launch, not two:
/// its helpers often show up a moment after the main process.
const SAME_LAUNCH: Duration = Duration::from_secs(5);

/// How many bundle IDs one sweep may read, so a flood of processes with
/// made-up `.app` paths cannot keep root running `plutil` for seconds while
/// it holds the service. Bundles wait their turn oldest first, so a flood
/// only delays a real app, never hides it.
const LOOKUPS_PER_SWEEP: usize = 128;

/// How long one sweep may spend reading bundle IDs; the rest wait for the
/// next one.
const LOOKUP_TIME: Duration = Duration::from_millis(250);

/// Longer paths are not real executable paths. argv[0] can be any length,
/// and every `.app` in it would cost a lookup.
const MAX_PATH: usize = 1024;

/// How many of the innermost bundles in a path are checked, besides the
/// outermost: real apps nest a helper or two deep, and more is made up.
const INNER_BUNDLES: usize = 3;

/// How long a blocked app's ID stays known after its last process is gone,
/// so a relaunch is closed at once instead of queueing again.
const KEEP_BLOCKED: Duration = Duration::from_secs(600);

/// How many such gone bundles are kept at most, newest first, so short-lived
/// processes from many copies cannot grow root's memory.
const MAX_GONE: usize = 256;

/// Reading longer than this means the first listing is stale: pids may have
/// been reused, so the sweep lists the processes again.
const RELIST_AFTER: Duration = Duration::from_millis(5);

/// How long a bundle is left alone after its ID was read before it is read
/// again: when the read failed, and when its `Info.plist` changed, so
/// touching a file every second cannot make root read it every second.
/// Short enough that a transient `plutil` failure does not free an app for
/// the block.
const REREAD_AFTER: Duration = Duration::from_secs(30);

/// An `Info.plist` larger than this, or one that is not a regular file, is
/// never handed to `plutil`: a device or a huge file would keep root busy.
const MAX_PLIST_BYTES: u64 = 1024 * 1024;

/// macOS gives people uids from 501 up; lower ones are root and the system's
/// own accounts, whose processes a block never touches, whatever ID their
/// bundle claims.
const FIRST_USER_UID: u32 = 501;

/// Bundles of macOS itself (the malware scanner, Software Update, the
/// Finder and the like) are never closed. Apple's apps people do block live
/// elsewhere: Messages and Calculator in `/System/Applications`, Safari in
/// the app cryptex (`/System/Volumes/Preboot/Cryptexes/App`).
const SYSTEM_BUNDLES: [&str; 4] = [
    "/System/Library/",
    "/System/iOSSupport/System/Library/",
    "/System/Volumes/Preboot/Cryptexes/OS/",
    "/Library/Apple/",
];

/// True for processes of macOS itself. A path that climbs out with `..`
/// is not one, however it starts.
fn is_system(path: &str) -> bool {
    SYSTEM_BUNDLES.iter().any(|prefix| path.starts_with(prefix))
        && !path.split('/').any(|part| part == ".." || part == ".")
}

/// A row of the process table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Process {
    pub(crate) pid: i32,
    pub(crate) uid: u32,
    pub(crate) path: String,
}

/// Parses `ps -o pid=,uid=,comm=`; rows that do not fit are skipped.
pub(crate) fn parse_ps(output: &str) -> Vec<Process> {
    output
        .lines()
        .filter_map(|line| {
            let (pid, rest) = line.trim_start().split_once(char::is_whitespace)?;
            let (uid, path) = rest.trim_start().split_once(char::is_whitespace)?;
            Some(Process {
                pid: pid.parse().ok()?,
                uid: uid.parse().ok()?,
                path: path.trim().to_owned(),
            })
        })
        .collect()
}

/// Which signal to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// SIGTERM: asks the app to quit.
    Term,
    /// SIGKILL: ends it.
    Kill,
}

/// Sends signals to processes.
pub trait Terminator: Send {
    /// Sends `stop` to `pid`, owned by `uid`.
    fn stop(&self, pid: i32, uid: u32, stop: Stop) -> io::Result<()>;
}

/// Signals for real; for the installed daemon.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTerminator;

impl Terminator for SystemTerminator {
    fn stop(&self, pid: i32, _uid: u32, stop: Stop) -> io::Result<()> {
        let signal = match stop {
            Stop::Term => Signal::SIGTERM,
            Stop::Kill => Signal::SIGKILL,
        };
        kill(Pid::from_raw(pid), signal).map_err(io::Error::from)
    }
}

/// Logs instead of signalling; the dev daemon's default.
#[derive(Debug, Clone, Copy, Default)]
pub struct DryRunTerminator;

impl Terminator for DryRunTerminator {
    fn stop(&self, pid: i32, uid: u32, stop: Stop) -> io::Result<()> {
        info!(pid, uid, ?stop, "dry run: would signal");
        Ok(())
    }
}

/// Signals only processes of one user and logs the rest; lets the
/// unprivileged dev daemon close the developer's own apps.
#[derive(Debug, Clone, Copy)]
pub struct OwnUidTerminator {
    uid: u32,
}

impl OwnUidTerminator {
    /// A terminator limited to processes of `uid`.
    #[must_use]
    pub fn new(uid: u32) -> Self {
        Self { uid }
    }
}

impl Terminator for OwnUidTerminator {
    fn stop(&self, pid: i32, uid: u32, stop: Stop) -> io::Result<()> {
        if uid == self.uid {
            SystemTerminator.stop(pid, uid, stop)
        } else {
            DryRunTerminator.stop(pid, uid, stop)
        }
    }
}

/// Identifies one `Info.plist` on disk, so a bundle replaced at the same
/// path is read again. The ctime is in it because a file's owner can set
/// its mtime back, but not its ctime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Stamp {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
}

/// Reads bundles' `Info.plist` files.
pub(crate) trait Plists: Send {
    /// The identity of the file at `path`, or `None` when it cannot be read.
    fn stamp(&self, path: &str) -> Option<Stamp>;

    /// The file's bytes and the identity of the file they came from,
    /// refusing anything but a regular file of at most [`MAX_PLIST_BYTES`].
    fn read(&self, path: &str) -> io::Result<(Vec<u8>, Option<Stamp>)>;
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemPlists;

impl Plists for SystemPlists {
    fn stamp(&self, path: &str) -> Option<Stamp> {
        fs::metadata(path).ok().as_ref().map(Stamp::of)
    }

    /// Opens the file once and checks what was opened, so it cannot be
    /// swapped between the check and the read. `O_NONBLOCK` keeps a FIFO
    /// from waiting for a writer. Symlinks are followed, as `LaunchServices`
    /// does: refusing them would free a copy whose `Info.plist` links to the
    /// original. Root learns no more than whether the target names a
    /// blocked app.
    fn read(&self, path: &str) -> io::Result<(Vec<u8>, Option<Stamp>)> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(OFlag::O_NONBLOCK.bits())
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.len() > MAX_PLIST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a small regular file",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_PLIST_BYTES).read_to_end(&mut bytes)?;
        Ok((bytes, Some(Stamp::of(&meta))))
    }
}

impl Stamp {
    fn of(meta: &fs::Metadata) -> Self {
        Self {
            dev: meta.dev(),
            ino: meta.ino(),
            size: meta.size(),
            mtime: meta.mtime(),
            mtime_nsec: meta.mtime_nsec(),
            ctime: meta.ctime(),
            ctime_nsec: meta.ctime_nsec(),
        }
    }
}

/// What reading a bundle's ID found.
#[derive(Debug, Clone)]
enum Lookup {
    Found(BundleId),
    /// `plutil` failed or was not run; tried again after [`REREAD_AFTER`].
    Failed,
    /// The bundle's ID is not one a list can hold, so it is never blocked.
    Invalid,
}

/// A bundle's ID as last read.
#[derive(Debug, Clone)]
struct Cached {
    stamp: Option<Stamp>,
    lookup: Lookup,
    at: Duration,
    /// The last sweep that saw a process in this bundle.
    seen: Duration,
}

impl Cached {
    fn bundle_id(&self) -> Option<&BundleId> {
        match &self.lookup {
            Lookup::Found(bundle_id) => Some(bundle_id),
            Lookup::Failed | Lookup::Invalid => None,
        }
    }

    /// Whether the ID should be read again for a file now at `stamp`.
    fn stale(&self, stamp: Option<Stamp>, monotonic: Duration) -> bool {
        let due = monotonic.saturating_sub(self.at) >= REREAD_AFTER;
        match self.lookup {
            Lookup::Failed => due,
            Lookup::Found(_) | Lookup::Invalid => due && stamp != self.stamp,
        }
    }
}

#[derive(Debug)]
struct Pending {
    since: Duration,
    killed: bool,
}

/// Finds and closes processes of blocked apps, and remembers the recent
/// launches it closed for the shield.
pub struct AppGuard {
    runner: Box<dyn CommandRunner>,
    terminator: Box<dyn Terminator>,
    plists: Box<dyn Plists>,
    own_pid: i32,
    ids: HashMap<String, Cached>,
    /// Bundles whose ID is due to be read, with when they started waiting.
    waiting: HashMap<String, Duration>,
    pending: HashMap<(i32, BundleId), Pending>,
    recorded_at: HashMap<BundleId, Duration>,
    launches: VecDeque<BlockedLaunch>,
    next_id: u64,
    waker: Option<Waker>,
}

impl std::fmt::Debug for AppGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppGuard")
            .field("pending", &self.pending.len())
            .field("launches", &self.launches)
            .finish_non_exhaustive()
    }
}

impl AppGuard {
    /// A guard that lists processes through `runner` and signals them
    /// through `terminator`. It never signals itself.
    #[must_use]
    pub fn new(
        runner: impl CommandRunner + 'static,
        terminator: impl Terminator + 'static,
    ) -> Self {
        Self {
            runner: Box::new(runner),
            terminator: Box::new(terminator),
            plists: Box::new(SystemPlists),
            own_pid: i32::try_from(std::process::id()).unwrap_or(i32::MAX),
            ids: HashMap::new(),
            waiting: HashMap::new(),
            pending: HashMap::new(),
            recorded_at: HashMap::new(),
            launches: VecDeque::new(),
            next_id: 1,
            waker: None,
        }
    }

    /// Wakes the app through `waker` when a new launch is closed and the
    /// app is not open to show the shield.
    #[must_use]
    pub fn waking(mut self, waker: Waker) -> Self {
        self.waker = Some(waker);
        self
    }

    /// Reads plists through `plists` instead of the file system.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn reading(mut self, plists: impl Plists + 'static) -> Self {
        self.plists = Box::new(plists);
        self
    }

    /// The recent blocked launches, oldest first.
    #[must_use]
    pub fn launches(&self) -> Vec<BlockedLaunch> {
        self.launches.iter().cloned().collect()
    }

    /// Closes every running process of the `blocked` apps. `monotonic` is
    /// the daemon's clock reading; `record` says whether new closures count
    /// as blocked launches (not for apps that were open when the block
    /// started, which the Start warning already covered).
    pub(crate) fn sweep(
        &mut self,
        blocked: &BTreeMap<BundleId, AppName>,
        monotonic: Duration,
        now: Timestamp,
        record: bool,
    ) {
        let Some(mut processes) = self.list() else {
            return;
        };
        if self.read_bundle_ids(&processes, blocked, monotonic) {
            // Reading took time, and pids get reused: act only on a fresh
            // listing, with the IDs known by now.
            let Some(fresh) = self.list() else {
                return;
            };
            processes = fresh;
        }

        let mut seen = HashSet::new();
        let mut recorded = false;
        let mut app_open = false;
        for process in &processes {
            let mut found = None;
            let mut innermost = None;
            for bundle in checked_bundles(&process.path) {
                innermost = self.ids.get(bundle).and_then(Cached::bundle_id);
                if found.is_none()
                    && let Some(bundle_id) = innermost
                    && let Some(name) = blocked.get(bundle_id)
                {
                    found = Some((bundle_id.clone(), name));
                }
            }
            // The process runs from its innermost bundle; an outer one with
            // OrcBlock's ID is just a folder and must not hide a blocked app.
            if found.is_none() && innermost.is_some_and(|id| id.as_str() == APP_BUNDLE_ID) {
                app_open = true;
            }
            let Some((bundle_id, name)) = found else {
                continue;
            };
            let key = (process.pid, bundle_id);
            seen.insert(key.clone());
            match self.pending.get_mut(&key) {
                None => {
                    self.signal(process, Stop::Term);
                    if record {
                        recorded |= self.record(&key.1, name, monotonic, now);
                    }
                    self.pending.insert(
                        key,
                        Pending {
                            since: monotonic,
                            killed: false,
                        },
                    );
                }
                Some(pending)
                    if !pending.killed && monotonic.saturating_sub(pending.since) >= KILL_AFTER =>
                {
                    pending.killed = true;
                    self.signal(process, Stop::Kill);
                }
                Some(_) => {}
            }
        }
        self.pending.retain(|key, _| seen.contains(key));
        if recorded
            && !app_open
            && let Some(waker) = &mut self.waker
        {
            waker.wake(monotonic);
        }
    }

    /// Forgets per-block bookkeeping once no apps are blocked. The launches
    /// stay so the shield can still show the last one.
    pub(crate) fn idle(&mut self) {
        self.ids.clear();
        self.waiting.clear();
        self.pending.clear();
        self.recorded_at.clear();
    }

    /// The processes a block may close: those of people, outside macOS
    /// itself, with a believable path.
    fn list(&self) -> Option<Vec<Process>> {
        let output = match self.runner.run(PS, PS_ARGS, None) {
            Ok(output) => output,
            Err(err) => {
                warn!(%err, "could not list processes; retrying next tick");
                return None;
            }
        };
        let processes = parse_ps(&output.stdout)
            .into_iter()
            .filter(|process| {
                process.pid > 1
                    && process.pid != self.own_pid
                    && process.uid >= FIRST_USER_UID
                    && process.path.len() <= MAX_PATH
                    && !is_system(&process.path)
            })
            .collect();
        Some(processes)
    }

    /// Reads the IDs of the bundles in `processes` that are new or due, at
    /// most [`LOOKUPS_PER_SWEEP`] of them within [`LOOKUP_TIME`], longest
    /// waiting first. Bundles of running processes stay cached, and blocked
    /// apps for [`KEEP_BLOCKED`] after that. Says whether reading made the
    /// listing stale: `plutil` ran, or it took longer than [`RELIST_AFTER`].
    fn read_bundle_ids(
        &mut self,
        processes: &[Process],
        blocked: &BTreeMap<BundleId, AppName>,
        monotonic: Duration,
    ) -> bool {
        let started = Instant::now();
        let mut stamps = HashMap::new();
        for process in processes {
            for bundle in checked_bundles(&process.path) {
                stamps
                    .entry(bundle)
                    .or_insert_with(|| self.plists.stamp(&plist(bundle)));
            }
        }
        self.ids.retain(|bundle, cached| {
            if stamps.contains_key(bundle.as_str()) {
                cached.seen = monotonic;
                return true;
            }
            cached
                .bundle_id()
                .is_some_and(|id| blocked.contains_key(id))
                && monotonic.saturating_sub(cached.seen) < KEEP_BLOCKED
        });
        let mut gone: Vec<(Duration, String)> = self
            .ids
            .iter()
            .filter(|(bundle, _)| !stamps.contains_key(bundle.as_str()))
            .map(|(bundle, cached)| (cached.seen, bundle.clone()))
            .collect();
        if gone.len() > MAX_GONE {
            gone.sort_unstable();
            for (_, bundle) in &gone[..gone.len() - MAX_GONE] {
                self.ids.remove(bundle);
            }
        }
        let due: HashMap<&str, Option<Stamp>> = stamps
            .into_iter()
            .filter(|(bundle, stamp)| {
                self.ids
                    .get(*bundle)
                    .is_none_or(|cached| cached.stale(*stamp, monotonic))
            })
            .collect();
        self.waiting
            .retain(|bundle, _| due.contains_key(bundle.as_str()));
        for bundle in due.keys() {
            if !self.waiting.contains_key(*bundle) {
                self.waiting.insert((*bundle).to_owned(), monotonic);
            }
        }
        let mut queue: Vec<(Duration, &str)> = due
            .keys()
            .map(|bundle| (self.waiting[*bundle], *bundle))
            .collect();
        queue.sort_unstable();
        let mut ran = false;
        for (_, bundle) in queue.into_iter().take(LOOKUPS_PER_SWEEP) {
            if started.elapsed() >= LOOKUP_TIME {
                break;
            }
            let (lookup, read) = self.read_bundle_id(bundle, &mut ran);
            let stamp = read.or(due[bundle]);
            self.waiting.remove(bundle);
            self.ids.insert(
                bundle.to_owned(),
                Cached {
                    stamp,
                    lookup,
                    at: monotonic,
                    seen: monotonic,
                },
            );
        }
        ran || started.elapsed() > RELIST_AFTER
    }

    /// Reads the `Info.plist` itself and hands the bytes to `plutil`, which
    /// then never opens a path a user controls. Also returns the identity
    /// of the file it read.
    fn read_bundle_id(&self, bundle: &str, ran: &mut bool) -> (Lookup, Option<Stamp>) {
        let (bytes, stamp) = match self.plists.read(&plist(bundle)) {
            Ok(read) => read,
            Err(err) => {
                debug!(%err, bundle, "no readable Info.plist");
                return (Lookup::Failed, None);
            }
        };
        *ran = true;
        let lookup = match self.runner.run(
            PLUTIL,
            &["-extract", "CFBundleIdentifier", "raw", "-o", "-", "-"],
            Some(&bytes),
        ) {
            Ok(output) => BundleId::parse(&output.stdout).map_or(Lookup::Invalid, Lookup::Found),
            Err(err) => {
                debug!(%err, bundle, "no bundle ID");
                Lookup::Failed
            }
        };
        (lookup, stamp)
    }

    fn signal(&self, process: &Process, stop: Stop) {
        match self.terminator.stop(process.pid, process.uid, stop) {
            Ok(()) => info!(pid = process.pid, path = %process.path, ?stop, "closed a blocked app"),
            Err(err) => {
                debug!(%err, pid = process.pid, ?stop, "signal failed; the process may be gone");
            }
        }
    }

    /// Adds a blocked launch unless it is the same one as a moment ago;
    /// says whether it added one.
    fn record(
        &mut self,
        bundle_id: &BundleId,
        name: &AppName,
        monotonic: Duration,
        now: Timestamp,
    ) -> bool {
        let recent = self
            .recorded_at
            .get(bundle_id)
            .is_some_and(|at| monotonic.saturating_sub(*at) < SAME_LAUNCH);
        self.recorded_at.insert(bundle_id.clone(), monotonic);
        if recent {
            return false;
        }
        if self.launches.len() == LAUNCHES_KEPT {
            self.launches.pop_front();
        }
        self.launches.push_back(BlockedLaunch {
            id: self.next_id,
            bundle_id: bundle_id.clone(),
            name: name.clone(),
            at: now,
        });
        self.next_id += 1;
        true
    }
}

fn plist(bundle: &str) -> String {
    format!("{bundle}/Contents/Info.plist")
}

/// The bundles of a path worth checking: the outermost and the innermost
/// [`INNER_BUNDLES`], outermost first.
fn checked_bundles(path: &str) -> Vec<&str> {
    let mut all: Vec<&str> = bundles(path).collect();
    if all.len() > INNER_BUNDLES + 1 {
        all.drain(1..all.len() - INNER_BUNDLES);
    }
    all
}

#[cfg(test)]
pub(crate) mod fake {
    use std::io;
    use std::sync::{Arc, Mutex, PoisonError};

    use super::{Plists, Stamp, Stop, Terminator};

    /// Reads every `Info.plist` as its bundle's path, so a fake `plutil`
    /// can answer by bundle. No file has a stamp.
    #[derive(Debug, Clone, Copy, Default)]
    pub(crate) struct FakePlists;

    impl Plists for FakePlists {
        fn stamp(&self, _path: &str) -> Option<Stamp> {
            None
        }

        fn read(&self, path: &str) -> io::Result<(Vec<u8>, Option<Stamp>)> {
            let bundle = path.trim_end_matches("/Contents/Info.plist");
            Ok((bundle.as_bytes().to_vec(), None))
        }
    }

    /// Records signals instead of sending them.
    #[derive(Debug, Clone, Default)]
    pub(crate) struct FakeTerminator {
        sent: Arc<Mutex<Vec<(i32, Stop)>>>,
    }

    impl FakeTerminator {
        /// The signals sent since the last call.
        pub(crate) fn take(&self) -> Vec<(i32, Stop)> {
            std::mem::take(&mut *self.sent.lock().unwrap_or_else(PoisonError::into_inner))
        }
    }

    impl Terminator for FakeTerminator {
        fn stop(&self, pid: i32, _uid: u32, stop: Stop) -> io::Result<()> {
            self.sent
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((pid, stop));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
