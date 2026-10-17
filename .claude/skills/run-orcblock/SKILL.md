---
name: run-orcblock
description: Exact recipe to run the daemon in dev mode, drive it with the CLI, and launch the Tauri app against it. Use when asked to run, start or try the app, and from the verify skill after daemon or app changes.
---

## Dev daemon (no root, no system changes)
1. Start it in the background: `make daemon-dev`. It runs `orcblockd serve --dev --data-dir ./data`: state in `./data/state.json`, socket at `./data/orcblockd.sock`, the DNS sinkhole on 127.0.0.1:53536 (the installed helper owns 53535), and the `DryRunEnforcer`, which logs `would apply: [...]` instead of touching /etc/resolver or pf.
2. Wait until ready: `cargo run -q -p orcblock-cli -- --socket ./data/orcblockd.sock status` prints the status.
3. Drive it:
   - `... add https://www.YouTube.com/watch?v=x` (stored as `youtube.com`)
   - `... start 1m`, then `... remove youtube.com` must fail with the orc's message
   - `... potion` ends the block and shows potions left
   - `dig @127.0.0.1 -p 53536 youtube.com` answers `0.0.0.0` while blocked
4. Stop it with Ctrl-C (or stop the background task); it saves state on shutdown. `./data/` is gitignored; move it aside to start fresh.

## App
1. With the dev daemon running: `make dev`. It sets `ORCBLOCK_SOCKET=$PWD/data/orcblockd.sock` and runs `npm --prefix app run tauri dev`. The first build compiles Tauri (~2 min).
2. The window shows the sites list, theme switch in the top bar, potions as small flasks. Check light, dark and system.
3. Without a daemon the app shows the "Install helper" screen; in dev that button refuses and points at `make daemon-dev`.

## Real install (changes the system; only when the user asks)
`make install-helper` builds release binaries and runs `sudo target/release/orcblockd install`, which copies itself to `/Library/PrivilegedHelperTools/com.orcrobert.orcblockd`, writes `/Library/LaunchDaemons/com.orcrobert.orcblockd.plist` and bootstraps it. Undo with `sudo /Library/PrivilegedHelperTools/com.orcrobert.orcblockd uninstall` (refused while a block is active).
