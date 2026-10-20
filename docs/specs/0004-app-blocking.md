# 0004: Block macOS apps

Status: approved (maintainer, 2026-10-20)
Issue: roadmap, "Block apps as well as websites"

## Problem
OrcBlock blocks websites only. Someone who blocks youtube.com can still open Slack, Discord, Steam or a game and lose the afternoon there. The commitment device has a hole the size of the Dock.

## Behaviour
1. Apps sit on the same list as sites and in the same block: one timer, and one potion ends both.
2. Adding an app is fast. There are three ways, and all of them end in the same `add_app` request:
   - **"Add app" dialog.** It shows a grid of installed apps with their icons, with "Apps open now" first. Typing filters the grid instantly, and Enter adds the top match.
   - **"Browse in Finder…"** opens the native macOS picker at /Applications, limited to apps.
   - **Drag and drop** of a `.app` from Finder or the Dock onto the list.
3. Finder, the Dock, the login window, the menu bar, Control Centre, Notification Centre and OrcBlock itself cannot be added. The app says so plainly.
4. On Start, any blocked app that is open is listed: "Slack will be closed. Save your work." Confirming starts the block and closes those apps.
5. During a block, opening a blocked app closes it within about a second. An orc shield window appears with the orc, "<App> is blocked" and the time left. It closes on OK or after about 6 s.
6. If OrcBlock is not running when that happens, the daemon starts it in the background, showing only the shield.
7. Apps cannot be removed during a block, exactly like sites.
8. **Helper not installed or too old:** the app already shows "update the helper" on a version mismatch. The app list and picker are hidden until the helper speaks protocol 2.

## Acceptance criteria
- [ ] `BundleId` accepts only reverse-DNS IDs (ASCII alphanumerics, `-` and `.`, at least 2 labels, at most 255 characters) and compares them case-insensitively. It has table tests and proptests.
- [ ] Apps are stored in `State` and frozen into `Session`. State files from before this feature still load.
- [ ] A block can start with apps only, sites only, or both.
- [ ] Removing an app during a block is refused with `Locked`. Never-block IDs are refused with `Unblockable`.
- [ ] `PROTOCOL_VERSION` is 2, and a version-1 client gets `Version`, not a misread status.
- [ ] During a block the daemon closes any process whose outermost `.app` bundle has a blocked ID, including helpers nested inside it and copies of the app moved elsewhere.
- [ ] The daemon never signals pid ≤ 1, its own pid, or a process outside a `.app` bundle.
- [ ] The kill ring of the last 8 launches is in `Status.blocked_launches`. It is never persisted.
- [ ] The app shows the shield once per new kill entry.
- [ ] When OrcBlock is not running and a kill happens, the daemon wakes it with `--shield` for the console user. It does this at most once per 60 s and never for root.
- [ ] The picker filters and sorts correctly (vitest), and Enter adds the top match.
- [ ] Dev mode never signals another user's processes. `--dev-kill-own-apps` is refused as root.

## Design

### orcblock-core
- **`BundleId` newtype** in `app.rs`. It parses and validates like `Domain`, with a `BundleIdError` enum.
- **`AppList`** of `AppEntry {bundle_id, name, added_at}`. The `name` is display-only and limited in length.
- **`State` changes:**
  - `State.apps`, with `#[serde(default)]`.
  - `Session.apps`, also with `#[serde(default)]`.
  - `add_app`, `remove_app` and `blocked_apps()`. `remove_app` is refused during a block.
- **`start_block`** needs at least one site or app. `EmptyBlocklist` now means "nothing to block".
- **`NEVER_BLOCK`** lists `com.orcrobert.*`, `com.apple.finder`, `com.apple.dock`, `com.apple.loginwindow`, `com.apple.systemuiserver`, `com.apple.controlcenter` and `com.apple.notificationcenterui`. Adding any of them fails with `CommandError::Unblockable`.

### orcblock-ipc (`protocol.rs`)
- **New requests:** `AddApp {bundle_id, name}` and `RemoveApp {bundle_id}`.
- **`Status.apps`:** the configured app list.
- **`Block.apps`:** the apps frozen into the running block.
- **`Status.blocked_launches`:** a list of `{bundle_id, name, at}`.
- **New error kinds:** `InvalidApp` and `Unblockable`.
- **`PROTOCOL_VERSION` = 2.** Follows the `add-ipc-command` skill.

### orcblockd
- **New module `apps.rs` with `AppGuard`:**
  - **`parse_ps` and `outermost_bundle`** are pure. They are tested against `crates/orcblockd/testdata/ps/` fixtures covering nested helpers, spaces in names and non-bundle processes.
  - **`BundleResolver`** runs `/usr/bin/plutil -extract CFBundleIdentifier raw -o - <bundle>/Contents/Info.plist` through `CommandRunner`. It caches by bundle path.
  - **A `Terminator` trait:**
    - It sends SIGTERM, then SIGKILL after 3 s, using `nix::sys::signal::kill`.
    - The real implementation is used for the installed daemon.
    - `DryRunTerminator` is the dev default. It logs "would kill" but still records the launch, so the shield can be exercised.
    - `OwnUidTerminator` sits behind `--dev-kill-own-apps`. It only signals processes with the daemon's uid and is refused as root.
  - **A `Launcher` trait:**
    - It runs `/bin/launchctl asuser <uid> /usr/bin/open -g -b com.orcrobert.orcblock --args --shield`.
    - `<uid>` is the owner of `/dev/console`.
  - **The kill ring:** the last 8 kills, kept in memory.
- **Snapshot:** `/bin/ps -axww -o pid=,uid=,comm=`, run through `CommandRunner`. It runs on every 1 s tick while `blocked_apps()` is non-empty, and once in `reconcile` at block start. The same snapshot tells whether OrcBlock is running.

### orcblock-cli
`add-app <bundle-id>` and `remove-app <bundle-id>`. `render.rs` shows the apps and recent blocked launches.

### app/src-tauri
- **Commands:**
  - `installed_apps`: scans `/Applications`, `/Applications/Utilities`, `/System/Applications`, `/System/Applications/Utilities` and `~/Applications`.
    - It gets the name and ID with `plutil -convert json`.
    - It converts the `.icns` icon to a 64 px PNG with `sips`.
    - Icons are cached in the app cache dir, reusing the data-URL and cache code from `favicon.rs`. Results are cached for the session.
    - It runs in `spawn_blocking`.
  - `app_from_path`: used by Browse and by drag and drop.
  - `pick_app_in_finder`: runs `/usr/bin/osascript` with a **constant** script.
  - `add_app` and `remove_app`.
  - `running_blocked_apps`: used by the Start warning.
- **All OS calls** go through a runner trait with fakes in tests.
- **The `shield` window:** borderless, always on top, on all Spaces, centred, about 420×300.
- **The `--shield` argument** opens only that window.
- **The status task** (`spawn_status_task`) opens the shield once per new `blocked_launches` entry.

### app/src
- **New components:**
  - `AppPicker.svelte`, inside `Dialog.svelte`.
  - `AppRow.svelte`, next to `SiteRow.svelte`.
  - `routes/Shield.svelte`, with the orc from `lib/mascot`, the app name, the time left and OK.
- **The Start confirmation** lists the blocked apps that are open.
- **Styling:** theme.css tokens only, in light and dark.

### Files the daemon writes
None new. Apps live in the existing `state.json`. The kill ring is in memory only.

### Risks
- **Cost:** one `ps` a second during an app block, roughly 10–20 ms of CPU.
- **Delay:** the app may show its window for up to a second before it is killed.
- **Lost work:** apps with unsaved work lose it. The Start warning says so.

## Bypass and safety
- **No new way out.**
  - Apps are frozen into the session, and removal is refused during a block.
  - A crash or restart re-applies the block from `state.json`.
  - Nothing in this feature ends a block.
- **Untrusted socket input** is only ever parsed into a `BundleId`. No path from a request reaches the filesystem or a command. Bundle paths come only from `ps` output and are passed to `plutil` as a single `Command::arg`.
- **The osascript script is a constant.** No user data is interpolated into it.
- **`launchctl asuser`** only ever opens OrcBlock's own bundle ID with a fixed argument, at most once a minute. It is skipped when the console owner is root.
- **Signal safety:**
  - The daemon never signals pid ≤ 1, its own pid, or a process outside a `.app`.
  - The never-block list keeps the session usable: the Finder, the Dock and the login window cannot be blocked.
- **Accepted and documented bypasses:**
  - editing `Info.plist` in a copied app to change its ID;
  - running a non-bundle binary;
  - the up to 1 s before a kill;
  - the general ADR 0002 caveat that an administrator can unload the daemon.

## Slices
0. **Spec and ADR 0006.** Stop for approval.
1. **Core:**
   - [ ] `BundleId` and its parse errors, with table tests and proptests
   - [ ] `AppList` and `AppEntry`
   - [ ] `Session.apps`, `State::add_app`, `remove_app` and `blocked_apps`, with lock rules
   - [ ] `start_block` accepts apps only; `NEVER_BLOCK` and `Unblockable`
   - [ ] An old `state.json` fixture still loads
2. **ipc and CLI:**
   - [ ] `AddApp`, `RemoveApp`, `Status.apps`, `Block.apps`, `blocked_launches` and the new error kinds
   - [ ] `PROTOCOL_VERSION = 2`, with version mismatch tests
   - [ ] CLI `add-app` and `remove-app`, and rendering
3. **Daemon guard:**
   - [ ] `parse_ps` and `outermost_bundle` with `testdata/ps/` fixtures
   - [ ] `BundleResolver` with a cache (fake runner)
   - [ ] `Terminator` (TERM, then KILL after 3 s) and its safety filters
   - [ ] Wired into `tick` and `reconcile`, with `Service` tests using fakes
   - [ ] The kill ring is exposed in `Status`
   - [ ] Dry-run terminator by default in dev; `--dev-kill-own-apps` (own uid only, refused as root)
4. **App picker and list:**
   - [ ] Tauri `installed_apps` (plutil and sips, icon cache, blocking thread) behind a runner trait, with tests
   - [ ] `app_from_path` and drag and drop of a `.app`
   - [ ] `pick_app_in_finder` (constant osascript)
   - [ ] `AppPicker.svelte`: grid, type-to-filter, Enter adds, "Apps open now" first
   - [ ] `AppRow.svelte` on Home, with removal disabled during a block
   - [ ] The Start warning lists open blocked apps
   - [ ] vitest for the filter and sort logic; `make site-check` green
5. **Shield:**
   - [ ] The `shield` window config and `routes/Shield.svelte`
   - [ ] The status task opens the shield once per new `blocked_launches` entry, with tests for detecting new entries
6. **Waking the app:**
   - [ ] The `Launcher` trait and the console uid from `/dev/console`; skipped for root, at most once per 60 s, skipped when OrcBlock is running
   - [ ] The app handles `--shield`, with tests using fakes
7. **Wrap-up:**
   - [ ] `architecture.md`, the roadmap, the README and one website line updated
   - [ ] Spec checkboxes ticked and status set

## Test plan
- **Unit tests and proptests** for `BundleId`, plus table tests for the `State` app rules.
- **Fixture tests** for `ps` parsing and bundle resolution.
- **`Service` tests** with fake `CommandRunner`, `Terminator`, `Launcher` and `Clock`.
- **Protocol round-trip and version tests.**
- **Tauri tests** for the app scanner with a fake runner.
- **vitest** for the picker's filter and sort and for detecting new shield entries.
- **No network, and no real processes signalled in tests.**
- **Manual checks** are done by the QA agent with the checklist below. The developing agent does not run the app.

### QA checklist for the Copilot test agent (manual, real app)
**Rules:**
- Use the **dev version only**: `make daemon-dev` (state and socket in `./data`) and `make dev`.
- Every CLI call uses `--socket ./data/orcblockd.sock`.
- Never run `make install-helper` or `orcblockd install`, and never talk to `/var/run/orcblockd.sock`.
- Use 1-minute blocks.

**Setup:**
- [ ] Start `make daemon-dev -- --dev-kill-own-apps` in one terminal, then `make dev` in another.
- [ ] Check that the status shows `mode: dry_run`.

**Picker:**
- [ ] Add app → the grid shows installed apps with icons. Typing "calc" then Enter adds Calculator.
- [ ] "Browse in Finder…" opens at /Applications. Choosing TextEdit adds it.
- [ ] Dragging Chess.app from Finder onto the list adds it.
- [ ] Adding Finder or OrcBlock is refused with a clear message.

**Block:**
- [ ] Open Calculator, then Start a 1-minute block. The warning lists Calculator; confirming closes it.
- [ ] Opening Calculator during the block closes it within about 1 s and shows the orc shield with the time left. The shield closes after OK or about 6 s.
- [ ] A copy of Calculator.app on the Desktop is also closed.
- [ ] Removing Calculator during the block is refused.
- [ ] Quit OrcBlock.app and open Calculator. OrcBlock relaunches and shows only the shield.
- [ ] Light and dark theme both look right (shield and picker).

**Ending:**
- [ ] Drink a potion (dev ledger only). Calculator opens normally.
- [ ] Start another 1-minute block, let it run out, and check that apps open again.

**Teardown (mandatory, even if a step failed):**
- [ ] `cargo run -p orcblock-cli -- --socket ./data/orcblockd.sock status` shows **no block running**. If one is running, drink a potion.
- [ ] Stop `make dev` and the dev daemon (Ctrl+C), and confirm with `pgrep -fl orcblockd` that no dev daemon is left.
- [ ] `rm -rf ./data` so no dev block or state survives.
- [ ] Confirm that Calculator, TextEdit and Chess open normally.

## Out of scope
- **Endpoint Security or a signed system extension.** That is the upgrade path in ADR 0006.
- **Blocking command-line binaries or non-bundle processes.**
- **Per-app schedules or per-app timers.**
- **Windows and Linux app blocking.**

## Open questions
None. Resolved: apps that are already open at block start are closed without a shield and are not added to the kill ring. The Start warning covers them.
