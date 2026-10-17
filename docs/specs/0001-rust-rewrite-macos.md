# 0001: Rust rewrite for macOS with a root daemon, potions and a Tauri app

Status: approved (maintainer's goal, 2026-10-17)

## Problem
OrcBlock is a Python script that edits `/etc/hosts` while it runs in a terminal. Closing the terminal or editing the file ends the block, there is no app to install, and nothing stops you from quitting a block the moment it gets hard. The maintainer wants a real macOS application, written in Rust, whose blocks hold for the time you set, with a small, humane escape hatch: five sleep potions per month.

## Behaviour
1. First launch: the app shows the sleeping orc and "Helper required" with one paragraph on why a helper keeps blocks running when the app is closed. **Install helper** asks for the admin password once and installs the LaunchDaemon. Cancelling leaves the app on this screen.
2. Home, idle: the status strip reads "Idle" and the orc sleeps. A text field accepts anything that looks like a site (`https://www.YouTube.com/watch?v=1`, `reddit.com/r/x`, `bücher.de`) and adds the cleaned domain (`youtube.com`, `reddit.com`, `bücher.de`) with its favicon, or an empty square if there is none. Duplicates and subdomains of listed sites are rejected with a message. Each row has a remove button.
3. Duration: a large readout (`1:30`, "Ends 17:42") set with presets (25 min to 1 day) and minus/plus steps from 15 minutes to 7 days, finer at the short end. Double-clicking the readout lets you type any duration from 1 minute to 7 days (`1:30`, `90m`, `2h`, `2d 4h`). **Start blocking** is disabled while the list is empty. The CLI accepts any duration from 1 minute to 7 days.
4. Blocking: the status strip reads "Armed" and the orc stands guard with red eyes; a countdown (`1:59:42`) with a progress bar and "Ends 17:42 // 3 targets"; the list stays visible; new sites can be added and are blocked at once; rows show "Locked" instead of the remove button. Closing the app, quitting it, logging out or rebooting does not end the block.
5. Potions: five pixel vials above the orc show how many are left this month (purple liquid for each one left). **Drink** opens "End block now?" with the potions left. Confirm ends the block immediately and the orc drinks, then falls asleep. With zero potions the button is disabled and the shelf shows the refill date. Potions refill on the first day of each month.
6. When the timer reaches zero the block ends by itself, whether or not the app is open.
7. Theme menu in the top bar: System (default), Light, Dark. The choice persists.
8. The `orcblock` CLI offers `status`, `add`, `remove`, `start <duration>` and `potion` against the same daemon.

## Acceptance criteria
- [x] `Domain::parse` cleans scheme, credentials, port, path, query, fragment, trailing dot, case and a leading `www.`, converts IDNs to punycode, rejects IPs, single labels and invalid names; `Display` shows the Unicode form. Property-tested.
- [x] Adding a duplicate or a subdomain of a listed domain fails with `CommandError::AlreadyListed`; adding a parent domain replaces its listed subdomains.
- [x] `start` fails on an empty list, an active session, or a duration outside 1 min..=7 days.
- [x] During a session, `remove` of a domain in the session fails with `CommandError::Locked`; `add` succeeds and the domain joins the session.
- [x] `advance(elapsed)` ends the session exactly when remaining reaches zero; time never goes negative.
- [x] Five potions per calendar month; drinking ends the session and decrements atomically; drinking without a session or without potions fails; a new month resets; an earlier month does not.
- [x] The wire protocol round-trips every request and response; version mismatches yield a `version` error.
- [x] The daemon persists state atomically, restores an active session at start and enforces it before serving requests, credits restart gaps within the clamp and never credits a negative gap.
- [x] The daemon's tick loop ends the session at zero and clears enforcement; enforcement is re-applied when verification finds drift.
- [x] The DNS sinkhole answers A/AAAA for blocked names and their subdomains with 0.0.0.0/::, NXDOMAIN for `use-application-dns.net` and Private Relay hosts during a block, REFUSED for names it does not own, and never panics on malformed packets (property-tested).
- [x] `MacEnforcer` writes marked resolver files atomically, removes only marked files, loads the pf anchor, flushes caches; verified with a fake command runner and a temp root.
- [x] `orcblockd install` and `uninstall` produce the right plist and file modes (temp root in tests); uninstall is refused during a block.
- [x] The CLI drives a dev daemon end to end.
- [ ] The app shows Setup without a daemon, Home idle and Home blocking with live status, favicons with an empty-square fallback, potions, the confirm dialog, and all three themes.
- [x] `make bundle` produces `OrcBlock.app` with `orcblockd` inside.

## Design
See docs/architecture.md and ADRs 0002 and 0004. Crates: orcblock-core, orcblock-ipc, orcblockd, orcblock-cli, orcblock-app.

## Bypass and safety
- The daemon is the only writer of state and enforcement; the app has no privileges.
- Clock tampering: block time uses a monotonic clock that counts sleep; restart credit is clamped and never negative; potion months never go backwards.
- Tampering with enforcement: resolver files and pf rules are verified every 15 s and at boot.
- Uninstall during a block is refused.
- Socket: any local user may read status and start or extend blocking; only requests that can end a block (potion) exist, and they cost a potion.

## Slices
1. orcblock-core with full tests.
2. orcblock-ipc.
3. orcblockd service, store, socket server, dry-run enforcer, dev mode.
4. DNS sinkhole.
5. macOS enforcer, install and uninstall.
6. orcblock CLI.
7. Tauri app shell, theme, API layer, Setup screen.
8. Home screens, sites list with favicons, durations, countdown, potions.
9. Mascot placeholders.
10. Bundling with the sidecar, README.

## Test plan
Unit and property tests in core and ipc; service and socket integration tests with the dry-run enforcer and temp dirs; codec property tests; enforcer tests with a fake runner under a temp root; UI unit tests for formatting helpers; manual run on a Mac with the dev daemon, then with the installed helper in Safari and Chrome.

## Out of scope
Blocking apps (roadmap), schedules, signing and notarization, Homebrew, Windows and Linux.

## Open questions
- Final orc art (docs/mascot.md); placeholders ship first.
