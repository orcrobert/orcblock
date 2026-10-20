# Architecture

```
 ┌──────────────── OrcBlock.app (user) ────────────────┐        ┌────────── orcblockd (root, launchd) ──────────┐
 │  Svelte UI  ──invoke──▶  Tauri commands (Rust)       │  JSON  │  server ──▶ service ──▶ orcblock-core::State   │
 │  app/src                 app/src-tauri               │ lines  │     ▲          │  persist ──▶ store (state.json)│
 │                          favicons, tray, helper      │◀──────▶│     │          └─ reconcile ──▶ enforce        │
 │                          install                     │ Unix   │  tick loop (1 s): advance time, verify, end    │
 └──────────────────────────────────────────────────────┘ socket │  dns sinkhole 127.0.0.1:53535                  │
          orcblock CLI ──────────────────────────────────────────▶└────────────────────────────────────────────────┘
                                                                     │ /etc/resolver/<domain>  pf anchor  flush
```

## Crates

| Crate | Kind | Responsibility |
| --- | --- | --- |
| `orcblock-core` | lib | The domain. `Domain` (parse and clean user input), `Blocklist`, `BundleId` and `AppList`, `Session`, `PotionLedger`, `State` and its commands. Pure and synchronous: no I/O, no clock, no async. |
| `orcblock-ipc` | lib | The wire protocol: versioned `Request`/`Response` as newline-delimited JSON, `Status` snapshot, and a blocking `Client`. |
| `orcblockd` | lib + bin | The daemon: socket server, state store, tick loop, enforcement layers, DNS sinkhole, app guard, install and uninstall. |
| `orcblock-cli` | bin | `orcblock`: status, add, remove, start, potion from the terminal. |
| `orcblock-app` | bin (Tauri) | The desktop app's Rust side: commands for the UI, the daemon connection, favicons, helper install. |
| `agenthooks` | bin | Claude Code hooks for this repo. |

Dependencies point one way: `core ← ipc ← {daemon, cli, app}`. Core knows nothing about JSON framing, sockets or Tauri.

## State and rules (orcblock-core)

- **Blocklist**: an ordered set of `Domain`s with the time each was added. A `Domain` is lowercase ASCII (punycode for IDNs), no scheme, port, path, trailing dot or leading `www.`; adding `m.youtube.com` when `youtube.com` is listed is a no-op because the resolver file for `youtube.com` already covers subdomains.
- **App list**: an `AppList` of `BundleId`s (lowercase reverse-DNS, validated) with a display name and the time each was added. The Finder, the Dock, the login window, the menu bar, Control Centre, Notification Centre and OrcBlock itself (`com.orcrobert*`) can never be added or blocked.
- **Session**: the domains and apps frozen at start (plus any added during the block), the chosen duration, `remaining`, and when it started. It ends when `remaining` reaches zero or a potion is spent. Sites and apps cannot be removed from a running session; adding is always allowed (it only makes the block stricter).
- **Potion ledger**: `POTIONS_PER_MONTH = 5`, the month the count belongs to, and how many were used. A new calendar month resets the count; a month earlier than the stored one is treated as the stored month.
- **State**: blocklist + optional session + ledger. Commands (`add`, `remove`, `start`, `drink_potion`, `advance`) validate fully before mutating and return typed `CommandError`s that the UI shows verbatim.

## Time

The daemon's `Clock` provides wall time (for display and month boundaries) and a monotonic `elapsed` that keeps counting during sleep (`clock_gettime(CLOCK_MONOTONIC)` through `nix`, which on macOS is `mach_continuous_time`; `std::time::Instant` uses `CLOCK_UPTIME_RAW` there and stops while the Mac sleeps, so it is not used for block time). Every tick it subtracts monotonic elapsed time from `remaining` and persists. After a restart (crash, reboot), it credits `wall_now - last_persisted_wall` clamped to `0..=MAX_RESTART_CREDIT`, so a powered-off Mac still counts down, but setting the clock forward while the daemon is down only goes so far, and setting it back credits nothing.

## Enforcement (orcblockd::enforce)

See ADR 0002 and the enforcement skill. In short: marked `/etc/resolver/<domain>` files route lookups to the sinkhole, the sinkhole answers `0.0.0.0`/`::`, pf drops DoT/DoH to known resolvers and kills existing connections at block start, caches are flushed, and everything is re-verified every 15 s and at boot. `Enforcer` is a trait; `MacEnforcer` does the real thing, `DryRunEnforcer` records calls for dev mode and tests.

## Apps (orcblockd::apps, orcblockd::wake)

See ADR 0006. While a block holds apps, `AppGuard` lists processes every second (`ps -axww -o pid=,uid=,comm=` in a UTF-8 locale), checks each `.app` bundle in a process's argv[0], outermost first, by its `CFBundleIdentifier` (`plutil`, cached by path and `Info.plist` identity; failures retried after 30 s; at most 128 new lookups a sweep), and sends a blocked one SIGTERM, then SIGKILL 3 s later through a `Terminator`. It never signals pid 0 or 1, itself, root's or system accounts' processes (uid < 501), or bundles under `/System/Library` and `/Library/Apple`. Apps open when the block starts are closed without being recorded; later launches go into an in-memory ring of the last 8 (`Status.blocked_launches`), from which the app shows the orc shield. When a launch is recorded and OrcBlock is not running, the installed daemon's `Waker` runs `launchctl asuser <owner of /dev/console> sudo -u #<uid> open -g -b com.orcrobert.orcblock --args --shield`, at most once a minute and never for root. Dev mode uses `DryRunTerminator` (or `OwnUidTerminator` with `--dev-kill-own-apps`) and no waker.

## Wire protocol (orcblock-ipc)

One JSON object per line in each direction (`{"v":1,"cmd":"add_site","site":"youtube.com"}`, answered by `{"result":"ok","status":{...}}` or `{"result":"error","kind":...,"message":...}`), one request per connection or many in sequence. Every request carries `"v": PROTOCOL_VERSION`; the daemon answers a newer or older major version with `Response::Error { kind: "version", .. }` so the app can ask the user to update the helper. Requests: `status`, `add_site`, `remove_site`, `add_app`, `remove_app`, `start_block`, `drink_potion`. `PROTOCOL_VERSION` is 2 since apps were added. Every successful response carries a full `Status` (blocklist, session with `remaining_secs` and `ends_at`, potions left, daemon version), so clients never need a second round trip.

## App (app/)

- `app/src-tauri`: Tauri commands mirror the requests and call the blocking IPC client inside `spawn_blocking`. A background task polls status every 2 s (1 s while a block runs) and emits `status` events. Favicons are fetched once per domain (`https://<domain>/apple-touch-icon.png`, then `/favicon.ico`, then the first few `<link rel="icon">` tags of the home page; `www.<domain>` when the bare name is unreachable), checked by magic bytes, capped at 256 KB, cached in the app cache dir and returned as data URLs. Redirects and icon links may go to any https host name on the default port whose top-level label is letters (no IP literal in any spelling, no `localhost`); this filters names, not addresses. Unreachable sites are not cached as misses; the row asks again after a minute, backing off to an hour. The app picker scans the usual Applications folders with `plutil`, converts icons with `sips` (cached as PNG), lists open apps with `ps`, and offers Browse in Finder through `osascript` with a constant script; all of it runs behind a `Runner` trait. A second, borderless `shield` window opens once per new blocked launch; started with `--shield` (by the daemon) the app shows only that window. If the daemon socket is missing, the UI shows the Setup screen, whose button runs the bundled `orcblockd install` through an admin prompt.
- `app/src`: Svelte 5 UI. Swiss brutalist tokens in `theme.css` (see the ui-components skill), theme stored in `localStorage` (`system` by default). One window, 900 x 520, resizable only within narrow bounds. The orc is a 1-bit pixel sprite: frames in `lib/mascot/frames.ts`, played by `lib/mascot/Orc.svelte` in the theme's colours.

## Files on a user's Mac

| Path | Owner | What |
| --- | --- | --- |
| `/Applications/OrcBlock.app` | user | The app, with `orcblockd` inside `Contents/MacOS`. |
| `/Library/PrivilegedHelperTools/com.orcrobert.orcblockd` | root 0755 | Installed daemon binary. |
| `/Library/LaunchDaemons/com.orcrobert.orcblockd.plist` | root 0644 | launchd job. |
| `/Library/Application Support/OrcBlock/state.json` | root 0644 | Blocklist, session, potions. |
| `/var/run/orcblockd.sock` | root 0666 | The socket. |
| `/etc/resolver/<domain>` | root 0644 | Only during a block, each marked `# managed by orcblockd`. |
| `~/Library/Caches/com.orcrobert.orcblock/favicons/` | user | Favicon cache. |
