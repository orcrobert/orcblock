# AGENTS.md

OrcBlock is a macOS website blocker: a Tauri desktop app talking to a small root daemon that keeps the block in place even when the app is closed or the Mac reboots. A block runs for the time you set; the only way out early is one of five sleep potions per month.
Read this file before any task. Details live in docs/; open them only when relevant.

## Commands
- `make check`: the gate (rustfmt, clippy with -D warnings, tests, cargo-deny, UI typecheck and tests). Must pass before every commit.
- `make test`: all Rust tests. One crate: `cargo test -p orcblock-core`. One test: `cargo test -p orcblock-core potion`.
- `make daemon-dev`: run orcblockd unprivileged with state and socket in ./data and a dry-run enforcer that logs instead of touching the system.
- `make dev`: the Tauri app with hot reload, pointed at the dev daemon (`ORCBLOCK_SOCKET=./data/orcblockd.sock`). Start `make daemon-dev` first.
- `cargo run -p orcblock-cli -- status`: talk to a daemon from the terminal. Add `--socket ./data/orcblockd.sock` for the dev daemon.
- `make bundle`: build `OrcBlock.app` and the dmg into target/release/bundle.
- `make bundle-universal`: the release build: one ad hoc signed app for Apple Silicon and Intel and `OrcBlock.dmg` in target/universal-apple-darwin/release/bundle. Merging a version bump into main releases it; see docs/releasing.md.
- `make site-dev`, `make site-check`, `make site-build`: the website in website/ (needs `npm ci --prefix app` and `npm ci --prefix website`). `make site-screenshots` re-renders its app screenshots. Not part of `make check`: run `make site-check` after changing app/src/lib, which the site imports. Deployed to Cloudflare Pages from main; see website/README.md.
- `make install-helper`: install the freshly built daemon as the real LaunchDaemon (asks for sudo; changes the system). Only when the user asks.

## Map
- crates/orcblock-core: the domain, pure and synchronous. Domain parsing and cleanup, blocklist, block sessions, the potion ledger, the `State` machine and its commands. No I/O, no clocks, no async. Time comes in as arguments.
- crates/orcblock-ipc: the wire protocol (versioned `Request`/`Response`, newline-delimited JSON) and a small blocking client. Shared by the app, the CLI and the daemon.
- crates/orcblockd: the root LaunchDaemon. `lib.rs` holds everything; `main.rs` only parses args and calls it. Modules: `server` (Unix socket), `store` (atomic state file), `service` (applies commands to `State`, owns the tick loop), `enforce` (the `Enforcer` trait and the macOS layers: resolver files, DNS sinkhole, pf), `dns` (the DNS message codec and sinkhole server), `install` (launchd plist, install and uninstall).
- crates/orcblock-cli: `orcblock`, a thin terminal client over orcblock-ipc.
- app/src-tauri (crate orcblock-app): Tauri commands, the daemon connection, favicon fetching and cache, the tray.
- app/src: the Svelte 5 + TypeScript UI. Design tokens in app/src/lib/theme.css; the orc mascot in app/src/lib/mascot.
- website: the download site (Vite + Svelte 5), reusing app/src/lib's tokens, orc and vials through the `$app` alias. Non-technical copy; technical detail lives in the README.
- tools/agenthooks: Claude Code hooks. Tested like any other crate.
- docs/architecture.md: how the pieces fit. docs/specs: one spec per feature. docs/decisions: ADRs. docs/roadmap.md: what is next. docs/mascot.md: how the orc art is made.

## Hard rules
1. No new dependencies without an ADR. The allowlist is in docs/decisions/0001-dependency-policy.md and enforced by `cargo deny`. Same for npm packages.
2. orcblock-core stays pure: no `std::fs`, `std::net`, `std::process`, `SystemTime::now`, threads or async. If logic needs the outside world, pass the value in or put a trait at the edge.
3. The daemon runs as root. Never build a shell command from a string; call binaries by absolute path with `Command::arg`. Never follow paths or domains from a request into the filesystem without validating them as a `Domain` first. Treat everything on the socket as untrusted.
4. A block can never end early except by its timer or a potion. Any change that adds a way out (a new request, a recovery path, a crash that clears state) is a bug, and the security-reviewer must see it.
5. No network in tests. Use fakes behind the traits (`Enforcer`, `CommandRunner`, `Clock`) and recorded fixtures in testdata/.
6. Never touch the real system in tests or dev mode: no writes to /etc, /Library, pf or launchd outside `make install-helper` and the installed daemon.
7. `unsafe` is forbidden in every crate (`#![forbid(unsafe_code)]`) except where a crate's lib.rs documents why; each `unsafe` block carries a `// SAFETY:` comment.
8. Never weaken tests, lints, deny rules, hooks or CI to get green. Fix the code or stop and ask.
9. Stay in scope. Unrelated problems go under "Follow-ups" in the PR description.
10. Comments are the exception. Doc comments (`///`, `//!`) on public items say what a caller needs to know, never restate the name. Plain `//` comments only for a why that the code cannot show: an invariant, an OS quirk, a security decision, `// SAFETY:`. No commented-out code, no banners.

## Git
- Never commit on main. Start every task: git switch main && git pull --ff-only && git switch -c <type>/<slug>
- In a worktree (main is checked out elsewhere): git fetch origin && git switch -c <type>/<slug> origin/main
- Never push to main unless the user explicitly asks in this conversation; the hook makes them confirm. Never force-push. Never merge, approve or mark a PR ready unless asked.
- Commit in logical steps as you go: one change per commit, each commit builds and passes make test, tests in the same commit as the code they cover, renames and refactors in their own commits before the behaviour change.
- Stage explicit paths and read git diff --staged before every commit. Never git add -A or git add .
- Conventional Commits: type(scope): summary. Imperative, lowercase, no trailing period, header max 72 characters. Body says why.
- Types: feat fix perf refactor test docs build ci chore revert.
- Scopes: core ipc daemon enforce dns cli app ui mascot agents ci deps docs.
- Finish with /ship: it pushes the branch and opens a draft PR. /ship is user-invoked; when the work is done, say it is ready for /ship. Full procedure: the git-workflow skill.

## Workflow
1. A feature starts with /feature (writes docs/specs/NNNN-slug.md, then stops for approval). A bug starts with /bug (failing test first).
2. Implement an approved spec with /build, one slice at a time.
3. Small, clearly scoped changes (copy tweak, dependency bump, one-file fix) skip the spec: branch, commit conventionally, ready for /ship.
4. Any other change over ~50 lines: write a short plan (files, approach, tests) and wait for approval.
5. Tests first for domain parsing, the session and potion rules, the DNS codec and anything the daemon decides.
6. The verify skill runs make check before every commit. Fix failures; never bypass them.
7. Tick the spec checkboxes as you go; record anything future agents need in docs or here.

## Rust conventions
- Workspace: shared versions in `[workspace.dependencies]`, shared lints in `[workspace.lints]`; every crate sets `lints.workspace = true`. Edition 2024, toolchain pinned in rust-toolchain.toml.
- Libraries return typed errors (`thiserror`, one error enum per module boundary, variants say what failed with context). `anyhow` only in binaries and the Tauri command layer. No `unwrap`/`expect` outside tests and provably infallible spots, which say why in the `expect` message.
- Make invalid states unrepresentable: newtypes (`Domain`, `Minutes`), enums over flags, constructors that validate. Parse, don't validate.
- Functional core, imperative shell: decisions live in orcblock-core as pure functions over `State`; the daemon only does I/O and calls them.
- I/O at the edge behind small traits passed in by the caller (generics or `&dyn Trait`), never global state. No `lazy_static`/mutable statics.
- Async only where the work is I/O (tokio in the daemon, Tauri's runtime in the app). Never block the runtime: blocking work goes in `spawn_blocking`.
- Logging with `tracing`, structured fields. Never log the full blocklist at info level.
- Public API: `#[must_use]` on pure constructors and queries, `Debug` on all public types, `#[non_exhaustive]` on wire enums that will grow.
- Tests: unit tests in a `tests.rs` file next to the module (`#[cfg(test)] mod tests;`), integration tests in `crates/<name>/tests/`. Table-driven with named cases. Parsers get proptest property tests.
- Modules named for what they do; no `utils`, `helpers` or `common`.

## Definition of done
- make check green, new behaviour covered by tests, spec checkboxes ticked.
- UI change: before/after description or screenshot in the PR, checked in light and dark.
- Wire protocol change: `PROTOCOL_VERSION` bumped when old clients would misread it; app shows "update the helper" instead of breaking.
- No TODO without an issue link.
- Draft PR opened by /ship with every template section filled.

## Gotchas
- macOS reads /etc/resolver/<domain> through mDNSResponder; flush with `dscacheutil -flushcache` and `killall -HUP mDNSResponder` after changes, or old answers linger.
- pf rules live under the `com.apple/orcblock` anchor because /etc/pf.conf already loads `com.apple/*`; never edit /etc/pf.conf.
- The daemon measures block time with a monotonic clock while running; wall-clock jumps must not shorten a block. See docs/architecture.md, "Time".
- Tauri commands run on the async runtime; the IPC client is blocking, so call it through `tauri::async_runtime::spawn_blocking`.
