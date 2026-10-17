# 0001: Rust rewrite for macOS with a root daemon, potions and a Tauri app

Status: approved (maintainer's goal, 2026-10-17)

## Problem
OrcBlock is a Python script that edits `/etc/hosts` while it runs in a terminal. Closing the terminal or editing the file ends the block, there is no app to install, and nothing stops you from quitting a block the moment it gets hard. The maintainer wants a real macOS application, written in Rust, whose blocks hold for the time you set, with a small, humane escape hatch: five sleep potions per month.

## Behaviour
1. First launch: the app shows a sleeping orc and "OrcBlock needs a small helper to keep blocks running when the app is closed." **Install helper** asks for the admin password once and installs the LaunchDaemon. Cancelling leaves the app on this screen.
2. Home, idle: the orc sleeps; "The path is open". A text field accepts anything that looks like a site (`https://www.YouTube.com/watch?v=1`, `reddit.com/r/x`, `bücher.de`) and adds the cleaned domain (`youtube.com`, `reddit.com`, `bücher.de`) with its favicon, or a coloured letter if there is none. Duplicates and subdomains of listed sites are rejected with a message. Each row has a remove button.
3. Duration chips: 25 min, 1 h, 2 h, 4 h, and Custom (hours and minutes, 1 minute to 7 days). **Start blocking** is disabled while the list is empty.
4. Blocking: the orc stands guard; "None shall pass"; a countdown (`1:59:42`) with a progress bar; the list stays visible; new sites can be added and are blocked at once; remove buttons are disabled ("The orc won't move until the block ends"). Closing the app, quitting it, logging out or rebooting does not end the block.
5. Potions: a shelf of five flasks shows how many are left this month. **Drink a sleep potion** opens "Use 1 of N potions to end this block now?" Confirm ends the block immediately and the orc falls asleep. With zero potions the button is disabled with "Next potions arrive on <date>". Potions refill on the first day of each month.
6. When the timer reaches zero the block ends by itself, whether or not the app is open.
7. Theme menu in the top bar: System (default), Light, Dark. The choice persists.
8. The `orcblock` CLI offers `status`, `add`, `remove`, `start <duration>` and `potion` against the same daemon.

## Acceptance criteria
- [ ] `Domain::parse` cleans scheme, credentials, port, path, query, fragment, trailing dot, case and a leading `www.`, converts IDNs to punycode, rejects IPs, single labels and invalid names; `Display` shows the Unicode form. Property-tested.
- [ ] Adding a duplicate or a subdomain of a listed domain fails with `CommandError::AlreadyBlocked`; adding a parent domain replaces its listed subdomains.
- [ ] `start` fails on an empty list, an active session, or a duration outside 1 min..=7 days.
- [ ] During a session, `remove` of a domain in the session fails with `CommandError::Locked`; `add` succeeds and the domain joins the session.
- [ ] `advance(elapsed)` ends the session exactly when remaining reaches zero; time never goes negative.
- [ ] Five potions per calendar month; drinking ends the session and decrements atomically; drinking without a session or without potions fails; a new month resets; an earlier month does not.
- [ ] The wire protocol round-trips every request and response; version mismatches yield a `version` error.
- [ ] The daemon persists state atomically, restores an active session at start and enforces it before serving requests, credits restart gaps within the clamp and never credits a negative gap.
- [ ] The daemon's tick loop ends the session at zero and clears enforcement; enforcement is re-applied when verification finds drift.
- [ ] The DNS sinkhole answers A/AAAA for blocked names and their subdomains with 0.0.0.0/::, NXDOMAIN for `use-application-dns.net` and Private Relay hosts during a block, REFUSED for names it does not own, and never panics on malformed packets (property-tested).
- [ ] `MacEnforcer` writes marked resolver files atomically, removes only marked files, loads the pf anchor, flushes caches; verified with a fake command runner and a temp root.
- [ ] `orcblockd install` and `uninstall` produce the right plist and file modes (temp root in tests); uninstall is refused during a block.
- [ ] The CLI drives a dev daemon end to end.
- [ ] The app shows Setup without a daemon, Home idle and Home blocking with live status, favicons with monogram fallback, potions, the confirm dialog, and all three themes.
- [ ] `make bundle` produces `OrcBlock.app` with `orcblockd` inside.

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
