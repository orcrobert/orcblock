# 0001: Dependency policy

Status: accepted (2026-10-17)

## Context

OrcBlock installs a daemon that runs as root on people's Macs. Every dependency is code we did not review running with full privileges (in the daemon) or with the user's files (in the app). Coding agents tend to add packages to solve small problems. At the same time a GUI app cannot reasonably avoid a framework, and Rust's ecosystem is built around a handful of foundational crates.

## Decision

- Direct dependencies, Rust and npm, are allowlisted below. `scripts/check-deps.sh` (run by `make check`) fails on anything else, and on crates that declare dependencies without `{ workspace = true }`.
- Adding one needs a new ADR (or an amendment here) that says what it replaces, its size, maintenance status and licence, approved by the maintainer. Agents never add one on their own.
- Transitive dependencies are governed by `cargo deny` (deny.toml): licences, RustSec advisories, yanked crates and registries. The daemon's dependency tree is kept small on purpose: core, ipc, tokio, tracing, jiff, nix.
- Versions live once in `[workspace.dependencies]`; `Cargo.lock` and `app/package-lock.json` are committed.

### Allowlist

<!-- allowlist:start -->
- `serde`: serialization derive, everywhere.
- `serde_json`: state file, wire protocol, hook input.
- `thiserror`: library error enums.
- `anyhow`: errors in binaries and Tauri commands only.
- `tracing`: structured logging.
- `tracing-subscriber`: log output in binaries.
- `tokio`: async runtime for the daemon.
- `jiff`: time zones and calendar months for the potion ledger.
- `idna`: Unicode domain names to and from punycode.
- `nix`: safe wrappers for euid and file ownership in the daemon.
- `clap`: command-line parsing for the daemon and the CLI.
- `tauri`: the desktop app framework (ADR 0004).
- `tauri-build`: Tauri's build script.
- `ureq`: small blocking HTTPS client for favicons.
- `base64`: favicon data URLs.
- `sha2`: stable favicon cache file names.
- `proptest`: property tests (dev only).
- `tempfile`: temp dirs in tests (dev only).
- `npm:svelte`: UI components (ADR 0004).
- `npm:@sveltejs/vite-plugin-svelte`: Svelte in Vite.
- `npm:vite`: frontend build.
- `npm:typescript`: type checking.
- `npm:svelte-check`: Svelte type checking.
- `npm:@tsconfig/svelte`: base tsconfig.
- `npm:@tauri-apps/api`: calls into the Rust side.
- `npm:@tauri-apps/cli`: build and dev tooling.
- `npm:vitest`: UI unit tests.
- `npm:prettier`: formatting.
- `npm:prettier-plugin-svelte`: formatting Svelte files.
<!-- allowlist:end -->

## Consequences

- Some small things are written by hand rather than pulled in: the DNS message codec, the launchd plist, atomic file writes, URL cleanup on top of `idna`. Each is small and fully tested.
- Upgrades are reviewable. Dependabot proposes them weekly, grouped for Tauri, tokio and Svelte.
