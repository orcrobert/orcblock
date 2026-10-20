# 0006: Enforce app blocks by polling processes from the daemon

Status: accepted (2026-10-20)

## Context

Spec 0004 adds macOS apps to the block. The build is unsigned and ad hoc signed, has no paid Developer ID, and adds no new crates (ADR 0001). The block must hold with the app closed, like sites (ADR 0002).

Options considered for noticing that a blocked app has been launched:

1. **Endpoint Security (`ES_EVENT_TYPE_AUTH_EXEC`).** This is precise and can deny an exec before the app runs. It needs the `com.apple.developer.endpoint-security.client` entitlement, which Apple grants on request to Developer ID accounts, plus Full Disk Access and an FFI binding. It is not possible today.
2. **`NSWorkspace` launch notifications in the user's app.** These only fire while OrcBlock.app runs in that user's session, and quitting the app would end enforcement. That breaks ADR 0002's rule that the daemon owns the block. It also needs Objective-C bindings.
3. **The root daemon polls the process table** on its existing 1 s tick while an app block runs, and signals matching processes.

## Decision

Option 3, with option 1 as the upgrade path once the build is signed (alongside the Network Extension in ADR 0002).

- **Detection:**
  - `/usr/bin/env LC_CTYPE=UTF-8 /bin/ps -axww -o pid=,uid=,comm=` runs through the existing `CommandRunner`. The locale keeps non-ASCII paths intact; in the empty environment `ps` escapes them.
  - `comm` is the process's argv[0], which for apps started normally is the executable path. Paths over 1024 bytes are skipped, since argv[0] can be any length. The outermost `*.app` in the path and the innermost three are checked, so a helper counts as its app and a copy inside another `.app` folder is still found, even one carrying OrcBlock's own ID. OrcBlock counts as open only when the innermost bundle is OrcBlock.
  - The daemon opens the `Info.plist` itself, once, with `O_NONBLOCK`, checks that what it opened is a regular file of at most 1 MiB, and hands the bytes to `/usr/bin/plutil -extract CFBundleIdentifier raw -o - -` on stdin. A FIFO or a device, even one swapped in between check and read, is refused, and `plutil` never opens a path a user controls. Symlinks are followed, as LaunchServices follows them; refusing them would free a copy whose `Info.plist` links to the original.
  - Bundle IDs are cached for bundles of running processes, and for 10 minutes after the last process of a blocked app is gone (at most 256 such bundles), so a relaunch is closed at once:
    - The cache also stores the `Info.plist`'s identity: device, inode, size, mtime and ctime. The file's owner can set the mtime back but not the ctime. A changed file, and a failed read, are read again after 30 s, so touching a file every second cannot make root read it every second.
    - One sweep reads at most 128 bundles within 250 ms, longest waiting first. A deliberate flood of made-up bundles (the per-user process limit times four bundles) can delay a newly seen app by about two minutes, but cannot hide it.
    - When a sweep ran `plutil` or spent more than 5 ms reading, it lists the processes again before signalling, and signals only what that fresh listing shows, so a pid reused while bundles were read is not hit.
    - Every program the daemon runs is killed after 10 s, with everything it started, and reaped.
  - Matching is by bundle ID, so a renamed or moved copy is still caught.
  - Polling runs only while the running block contains apps.
- **Termination:**
  - The daemon sends SIGTERM, then SIGKILL after 3 s.
  - It never signals pid ≤ 1, its own pid, anything outside a `.app` bundle, processes of root or the system's accounts (uid < 501), or bundles under `/System/Library` and `/Library/Apple`. The socket is open to every local account, so the bundle ID alone must never decide that root kills a system process.
  - A never-block list keeps the Finder, the Dock, the login window, the menu bar, Control Centre, Notification Centre and OrcBlock itself out of reach.
- **Dependency:**
  - The allowlisted `nix` gains the `signal` feature for `kill` and `killpg`, and the `fs` feature for the `O_NONBLOCK` open flag (and `mkfifo` in a test). The owner of `/dev/console` comes from the standard library (`MetadataExt::uid`).
  - No new crate is added. The allowlist line for `nix` changes to "euid, file ownership, opening files safely and process signals".
- **Waking the app:** when a kill happens and OrcBlock.app is not running, the daemon runs `/bin/launchctl asuser <console uid> /usr/bin/sudo -u #<console uid> /usr/bin/open -g -b com.orcrobert.orcblock --args --shield`. `asuser` alone keeps root's uid; `sudo -u` drops it, so whatever LaunchServices opens for that bundle ID runs as the user.
  - Every argument is fixed except the uid, which is read from the owner of `/dev/console`.
  - It is skipped when that owner is root, and when OrcBlock is already open (it then shows the shield itself).
  - Only the installed daemon wakes the app; the unprivileged dev daemon cannot run `launchctl asuser` and never touches the system.
  - It runs at most once per 60 s.
- **The app picker's "Browse in Finder…"** runs `/usr/bin/osascript` with a constant script (`choose file of type {"com.apple.application-bundle"}`). This avoids adding the Tauri dialog plugin, a new dependency. No user data is ever put into the script.
- **Dev mode:**
  - The dev daemon uses a dry-run terminator that logs instead of signalling.
  - `--dev-kill-own-apps` signals only processes with the daemon's own uid, and is refused as root.

## Consequences

- **Delay:** a blocked app can run for up to about 1 s before it is closed, and may flash its window.
- **Cost:** one `ps` and an occasional `plutil` per second during an app block, roughly 10–20 ms of CPU each.
- **Accepted bypasses:**
  - editing `Info.plist` in a copy of the app to change its bundle ID;
  - running a non-bundle binary;
  - starting the app's binary with a different argv[0] (`exec -a`) or through a symlink outside the bundle, since `ps` reports argv[0]. Reading the real executable path needs `proc_pidpath`, an `unsafe` libproc call this ADR does not take on; it belongs with the Endpoint Security upgrade.
  - an administrator running the app as root or a system account (`sudo`), which the uid filter skips; this is ADR 0002's administrator caveat again.
  - Like ADR 0002's administrator caveat, these are deliberate, effortful acts, not slips.
- **Lost work:** unsaved work in a blocked app is lost when it is killed. The Start screen warns about open blocked apps first.
- **The lock:** a sweep runs while the daemon holds its state, so socket requests wait for it. The bounds above keep that to one or two `ps` runs, a `stat` per checked bundle, and at most 250 ms of `plutil` reads (plus a single `plutil` call that hangs, up to 10 s).
- **Shield only:** any local account can keep the app from being woken by running a process from a fake bundle that claims OrcBlock's ID. Apps are still closed; only the shield is lost.
- **Privacy:** the recent blocked launches are in every status, which every local account can read, like the list itself.
- **Known limit:** the daemon reads each bundle's `Info.plist` metadata in its own process. A bundle on a network share that stops answering can stall that read, and with it the daemon, until the share times out. Skipping non-local file systems needs `statfs`, a follow-up.
- **Upgrade path:** moving to Endpoint Security later replaces only the detection in `orcblockd/src/apps.rs`. Core, protocol and UI stay.
