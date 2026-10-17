---
name: add-ipc-command
description: Checklist for adding or changing a request between the app or CLI and the daemon - core rule, wire types, daemon handler, client, Tauri command, UI, tests and protocol version. Use when asked to add an action the daemon performs, or when the user runs /add-ipc-command.
argument-hint: "<request name>"
---

Request: $ARGUMENTS

1. **Core rule** (`crates/orcblock-core`): a method on `State` taking every input it needs (`now`, the argument) and returning `Result<T, CommandError>`. Validate before mutating. Add `CommandError` variants with messages a user can read. Table-test success and every error.
2. **Wire** (`crates/orcblock-ipc`): a `Request` variant (snake_case `type`) and, if needed, a `Response` variant. Additive changes keep `PROTOCOL_VERSION`; a change old clients would misread bumps it. Round-trip test in `protocol/tests.rs` plus a JSON golden string so the shape is visible in review.
3. **Daemon** (`crates/orcblockd/src/service.rs`): a match arm that locks state, calls the core method, persists, reconciles enforcement if the blocked set changed, then responds. Integration test over a temp socket with the dry-run enforcer.
4. **Bypass check**: if the request can change what is blocked, when a block ends or the potion count, fill the spec's "Bypass and safety" section and ask for a security-reviewer pass.
5. **Clients**: a method on `orcblock_ipc::Client`; a `orcblock` CLI subcommand; a Tauri command in `app/src-tauri/src/commands.rs` (blocking client via `spawn_blocking`, error mapped to a string) registered in `lib.rs` and allowed in `app/src-tauri/capabilities/default.json`.
6. **UI**: a typed wrapper in `app/src/lib/api.ts`, then the component. Errors from `CommandError` are shown verbatim in the snackbar.
7. `make check`.
