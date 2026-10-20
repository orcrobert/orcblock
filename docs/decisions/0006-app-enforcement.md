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
  - `/bin/ps -axww -o pid=,uid=,comm=` runs through the existing `CommandRunner`.
  - For each process, the outermost `*.app` in the executable path decides the bundle.
  - The bundle ID is read with `/usr/bin/plutil -extract CFBundleIdentifier raw`, cached by path.
  - Matching is by bundle ID, so a renamed or moved copy is still caught.
  - Polling runs only while the running block contains apps.
- **Termination:**
  - The daemon sends SIGTERM, then SIGKILL after 3 s.
  - It never signals pid ≤ 1, its own pid, or anything outside a `.app` bundle.
  - A never-block list keeps the Finder, the Dock, the login window, the menu bar, Control Centre, Notification Centre and OrcBlock itself out of reach.
- **Dependency:**
  - The allowlisted `nix` gains the `signal` feature for `kill`, and `fs` for `stat` on `/dev/console`.
  - No new crate is added. The allowlist line for `nix` changes to "euid, file ownership and process signals".
- **Waking the app:** when a kill happens and OrcBlock.app is not running, the daemon runs `/bin/launchctl asuser <console uid> /usr/bin/open -g -b com.orcrobert.orcblock --args --shield`.
  - Every argument is fixed except the uid, which is read from the owner of `/dev/console`.
  - It is skipped when that owner is root.
  - It runs at most once per 60 s.
- **The app picker's "Browse in Finder…"** runs `/usr/bin/osascript` with a constant script (`choose application`). This avoids adding the Tauri dialog plugin, a new dependency. No user data is ever put into the script.
- **Dev mode:**
  - The dev daemon uses a dry-run terminator that logs instead of signalling.
  - `--dev-kill-own-apps` signals only processes with the daemon's own uid, and is refused as root.

## Consequences

- **Delay:** a blocked app can run for up to about 1 s before it is closed, and may flash its window.
- **Cost:** one `ps` and an occasional `plutil` per second during an app block, roughly 10–20 ms of CPU each.
- **Accepted bypasses:**
  - editing `Info.plist` in a copy of the app to change its bundle ID;
  - running a non-bundle binary.
  - Like ADR 0002's administrator caveat, these are deliberate, effortful acts, not slips.
- **Lost work:** unsaved work in a blocked app is lost when it is killed. The Start screen warns about open blocked apps first.
- **Upgrade path:** moving to Endpoint Security later replaces only the detection in `orcblockd/src/apps.rs`. Core, protocol and UI stay.
