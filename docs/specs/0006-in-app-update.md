# 0006: In-app update

Status: approved
Issue: (none)

## Problem
Today the only way to get a new OrcBlock is to download the dmg again, quit the app, drag the new one over the old one and click through Gatekeeper's "Open Anyway" once more, because the app is signed ad hoc and the browser quarantines the download. Nothing in the app says a new version exists, so people stay on old releases, and old releases keep old helpers. The roadmap lists "Auto-update".

## Behaviour
1. Once, when the app starts, it asks GitHub whether a newer published release exists. It sends nothing but the request for one small JSON file. Dev mode (`make dev`) never checks.
2. No newer version, no network, or GitHub unreachable: the top bar looks as it does today (`REV 0.3.0`). Check failures are logged, never shown.
3. A newer version exists: next to `REV 0.3.0` the top bar shows a button `UPDATE → 0.4.0`. Hovering it, or VoiceOver, gives `Update to 0.4.0, released <date>`.
4. Click: the button turns into `DOWNLOADING 42%`, then `INSTALLING`. The app downloads the new bundle, checks its signature against the public key built into the app, replaces `OrcBlock.app` in place and relaunches itself. No dmg, no drag, no Gatekeeper prompt (the app downloads the file itself, so it is never quarantined).
5. If the app sits where the user cannot write (a root-owned `/Applications/OrcBlock.app`), the plugin asks for the administrator password once to swap it. Any failure (network drops, bad signature, a cancelled password prompt): the button comes back as `UPDATE → 0.4.0` and the snackbar says what went wrong in one sentence. The old app keeps working; nothing is half replaced.
6. The update does not touch the helper. If the new app speaks a newer protocol than the running helper, the existing "update the helper" state takes over (administrator prompt through `install_helper`). A release that keeps the protocol leaves the old helper running, so most updates need no password.
7. During a block the update works the same: the app is not what holds the block, the helper is.
8. Helper not installed: the app still offers its own update and keeps showing the existing install prompt.

## Acceptance criteria
- [x] With a newer published release, the top bar shows an update button naming the new version; with none, it shows only `REV x.y.z`.
- [x] A pre-release (`1.0.0-beta.1`) is never offered, since the update manifest follows `releases/latest`.
- [ ] Clicking it downloads, verifies the minisign signature, replaces the bundle and relaunches into the new version.
- [ ] A download whose signature does not match the built-in public key is refused, and the installed app is unchanged.
- [x] Failures show one sentence in the snackbar and bring the button back.
- [x] Dev mode never contacts the update endpoint.
- [ ] A running block's remaining time and the potion count are unchanged across an app update (manual QA on a real Mac).
- [x] Every release publishes `OrcBlock.app.tar.gz`, its `.sig` and `latest.json`, signed in the `publish` job, never in `build`.
- [x] Button states checked in light and dark.

## Design
**Dependency (needs ADR 0007).** `tauri-plugin-updater` (Rust only, MIT/Apache-2.0, maintained by the Tauri team, same release train as `tauri`). It downloads, verifies minisign signatures with `minisign-verify`, unpacks the `.app.tar.gz` and swaps the bundle. Hand-rolling this would mean our own signature verification, which is worse. The npm `@tauri-apps/plugin-updater` is not needed: the UI goes through our own Tauri commands, and relaunching uses `AppHandle::restart`, so no `tauri-plugin-process` either. The ADR records its transitive crates (`reqwest`, `tar`, `flate2`, ...) and what `cargo deny` says about them.

**App (`app/src-tauri`).** New module `update`:
- `check_update` command: returns `Option<{ version, date }>`; `None` in dev mode. The UI calls it once on start.
- `install_update` command: downloads and installs, emitting `update_progress` events (`{ type: "downloading", percent }` once per whole percent, then `{ type: "installing" }`), unpacking and swapping in `spawn_blocking`, then restarts. Only one install at a time: `Pending` moves to `Installing` and ignores checks until the install fails. Checks time out after 30 s, downloads after 10 min.
- `tauri.conf.json`: `plugins.updater.pubkey` (the public key, committed) and `endpoints: ["https://github.com/orcrobert/orcblock/releases/latest/download/latest.json"]`. `bundle.createUpdaterArtifacts` stays off: see Release.
- Capabilities: `allow-check-update`, `allow-install-update`. The CSP is unchanged; all network traffic is on the Rust side.

**UI (`app/src`).** `TopBar.svelte` gets an `UpdateButton` (Button tokens, no new colours) with four states: hidden, available, downloading %, installing. `api.ts` gets `checkUpdate`, `installUpdate` and the progress listener.

**Release (`release.yml`, `Makefile`, `docs/releasing.md`).**
- `make bundle-universal` also writes `OrcBlock.app.tar.gz` from the app *after* `codesign`, so the updated app keeps its ad hoc signature and pinned helper. (Tauri's own updater artifacts are made before our `codesign`, so they would be wrong.)
- The `build` job uploads the tarball with the dmgs and adds it to `SHA256SUMS`. It never sees the signing key, because it runs npm install scripts and build scripts.
- The `publish` job (no repository code, `release` environment) installs `minisign` from apt, signs the tarball with the key from the `TAURI_SIGNING_PRIVATE_KEY` environment secret, writes `OrcBlock.app.tar.gz.sig` in Tauri's format (base64 of the minisign signature file, with `version:x.y.z` in the trusted comment, which the app requires through `requireSignedVersion` so an old signed bundle cannot be served as a new version) and `latest.json` (`version`, `pub_date`, and the url and signature for `darwin-aarch64` and `darwin-x86_64`, both pointing at the universal tarball), attests the tarball and uploads all three.
- One-time maintainer step, documented: `npm --prefix app run tauri signer generate`, public key into `tauri.conf.json`, private key and password into the `release` environment's secrets.

**Risks.** A leaked signing key lets someone ship an update that the app installs, and that update's helper then asks for the admin password. Mitigations: key only in the `release` environment, used only by the job that runs no repository code; the helper still needs the user's password and is checked against the SHA-256 pinned in the (signed) app. Losing the key means users must reinstall from the dmg once, after a new key ships.

## Bypass and safety
- **Ending a block early.** The app update replaces only `OrcBlock.app`; the helper keeps running and keeps enforcing during and after it. Nothing here sends a request to the daemon. No new IPC request and no protocol change.
- **Helper reinstall during a block.** Unchanged: only a protocol bump asks for it, as today.
- **Downgrade to escape.** The updater only installs versions newer than the running one, and an older helper still reads the same state file (0.3.0 already keeps it readable by older helpers). A downgrade by hand is possible today and unchanged.
- **Potions.** Untouched: the app never writes the state file.
- **Other local processes.** `install_update` takes no arguments; the URL and public key are compiled in. A local process that can call Tauri commands already controls the user's app.
- **Supply chain.** See Risks: an update is code the user will later run as root via the helper prompt, so the signing key is handled like the release itself.

Needs the security-reviewer on slices 1 and 3.

## Slices
1. ADR 0007 for `tauri-plugin-updater`, the `update` module with `check_update`, `install_update` and tests, config with the public key, capabilities. (~200 lines)
2. UI: `UpdateButton` in the top bar, progress, error snackbar, vitest for the state logic. (~200 lines)
3. Release: tarball in `make bundle-universal`, signing and `latest.json` in the `publish` job, `docs/releasing.md`, roadmap tick, version 0.3.1. (~150 lines)

## Test plan
- Rust unit tests: progress reporting, one install at a time, error sentences. The plugin's own network calls are not faked; dev mode skipping the check is a guard in `check_update`, checked by hand with a build pointed at a local server.
- Vitest: button state machine (hidden → available → downloading → installing; error → available).
- CI: `make check`, `cargo deny` with the new crates.
- Manual on a real Mac: release a `0.3.1` from a test fork or draft, install `0.3.0` from the dmg, update in-app; app relaunches as 0.3.1 without Gatekeeper; tamper with the `.sig` and see the refusal; repeat with a block running and check remaining time and potions before and after.

## Out of scope
- Updating silently without a click, or a setting to turn checks off (could follow).
- Release notes shown inside the app.
- Updating the helper without the administrator prompt.
- Homebrew cask updates.
- Developer ID signing or notarisation.

## Decisions
1. Check once on start, not on a timer.
2. No helper prompt after an app update; only a protocol bump asks for it, as today.
3. The signing key lives as a secret of the `release` environment, read only by the `publish` job.
4. The first release with the updater is 0.3.1. Users on 0.3.0 and older reinstall from the dmg once.
