---
name: rust-architecture
description: How Rust code in this workspace is structured - crate boundaries, the functional core, error types, traits at the edges, async rules and public API conventions. Use when adding a module, type, crate, trait or error, or when unsure where code belongs.
paths: "crates/**,app/src-tauri/**"
---

## Where code goes
| It decides... | It lives in | It may use |
| --- | --- | --- |
| what a domain is, whether a block may start, when it ends, whether a potion may be spent | `orcblock-core` | std, serde, thiserror, idna |
| how messages look on the wire | `orcblock-ipc` | core, serde_json |
| when to read and write files, run pfctl, answer DNS, accept sockets | `orcblockd` | core, ipc, tokio, tracing |
| what the user sees | `app/src-tauri` (commands) + `app/src` (UI) | ipc, tauri |

If a function needs both a decision and I/O, split it: the decision is a pure function in core that returns a value (often an enum of what to do), and the shell performs it.

## The state machine
- `orcblock_core::State` owns the blocklist, the active `Session` and the `PotionLedger`. Every change is a method that takes the inputs it needs (including `now`) and returns `Result<_, CommandError>`; it never half-applies: validate first, then mutate.
- The daemon's `Service` holds `State` behind one `tokio::sync::Mutex`, calls a method, persists, then reconciles enforcement. Persist before acknowledging the request.
- New rules get a `CommandError` variant whose message the UI can show as is.

## Types
- Newtypes for anything with rules: `Domain`, `Minutes`. Constructors validate (`Domain::parse`), fields stay private, `Display` gives the user-facing form.
- Enums over booleans and strings. Wire enums are `#[non_exhaustive]` and `#[serde(tag = "type", rename_all = "snake_case")]`.
- Derive `Debug, Clone, PartialEq, Eq` where it makes sense; `Copy` for small value types; `Serialize, Deserialize` only on types that cross a boundary.

## Errors
- One `thiserror` enum per module boundary (`DomainError`, `CommandError`, `StoreError`, `EnforceError`). Variants carry the context (`#[error("write resolver file {path}: {source}")]`), wrap sources with `#[source]`/`#[from]`.
- `anyhow::Result` with `.context(...)` only in `main.rs` files and Tauri commands, converted to a string for the UI at the boundary.
- No `unwrap`/`expect` in non-test code unless infallible, with the reason in `expect("...")`.

## Traits at the edges
- `Enforcer` (apply, clear, verify), `CommandRunner` (run a program), `Clock` (wall `now` + monotonic `elapsed`) are small, object-safe traits defined next to their only consumer. Production types implement them for macOS; tests use fakes in the same crate under `#[cfg(test)]` or in `tests/`.
- Prefer generics (`Service<E: Enforcer>`) for the hot path and `&dyn Trait` where it keeps signatures simple.
- No global state: pass dependencies through constructors.

## Async
- The daemon runs one tokio runtime: the socket accept loop, the DNS sinkhole (UDP and TCP) and the tick task, coordinated with a `CancellationToken`-style shutdown via `tokio::sync::watch`.
- Never call blocking code (`std::fs`, `Command::output`) from async without `spawn_blocking`, except during startup before the runtime serves requests.
- Core is synchronous and never knows about async.

## Public API
- Crate root re-exports the main types; modules stay private unless callers need the path.
- `#[must_use]` on constructors and pure queries; doc comments on every public item saying what a caller needs (invariants, errors, units).
- `pub(crate)` by default inside binaries; `unreachable_pub` is a warning.

## Adding a crate
1. `crates/<name>/Cargo.toml` with `version.workspace = true` etc. and `[lints] workspace = true`.
2. Add it to `[workspace] members`, dependencies via `{ workspace = true }`.
3. `#![forbid(unsafe_code)]` is inherited from workspace lints; document any exception in lib.rs and in an ADR.
4. Add its row to the AGENTS.md map and docs/architecture.md.
