---
name: rust-testing
description: Test patterns for this workspace - table tests, proptest properties, fakes behind traits, daemon integration tests over a temp socket, and time without sleeps. Use when writing or changing any Rust test.
paths: "**/tests.rs,**/tests/**"
---

Run one crate: `cargo test -p orcblock-core`. The gate is `make check`.

## Layout
- Unit tests live in a sibling `tests.rs`: `foo.rs` declares `#[cfg(test)] mod tests;` and the tests sit in `foo/tests.rs` with `use super::*;`. For `lib.rs` or `mod.rs` the file is `tests.rs` in the same directory.
- Integration tests that use only the public API go in `crates/<name>/tests/<topic>.rs`.
- Fixtures (recorded DNS packets, sample state files) in `crates/<name>/testdata/`. Never hand-edit a recorded fixture; build edge cases inline.

## Table tests (default shape)
A slice or array of `(name, input, want)` tuples, iterate and assert with the name in the message: `assert_eq!(got, want, "{name}")`. See `tools/agenthooks/src/guard_git/tests.rs`. Large tables go in a `const CASES` at module level.

## Property tests
Parsers and codecs (`Domain::parse`, the DNS codec, IPC framing) get `proptest`:
- never panics on arbitrary input (`any::<Vec<u8>>()`, `".*"`),
- round-trips (`decode(encode(x)) == x`),
- output keeps its invariant (a parsed `Domain` is lowercase ASCII, has no scheme, path or `www.` prefix).
A shrunk failure gets added as a named table case so it stays covered.

## Time
- Core takes time as arguments (`now: DateTime`, `elapsed: Duration`); tests pass fixed values.
- The daemon uses a `Clock` trait; tests use `FakeClock` and advance it. Never `std::thread::sleep` or `tokio::time::sleep` to wait for a condition; with tokio use `#[tokio::test(start_paused = true)]` and `tokio::time::advance`.

## Daemon tests
- Build `Service` with the `DryRunEnforcer` (records what it would apply) and a store in `tempfile::tempdir()`.
- Socket tests start the server on `<tempdir>/orcblockd.sock`, talk to it with `orcblock_ipc::Client`, and shut it down through the shutdown channel.
- Never touch /etc, /Library, pf or launchd from a test. `make check` runs unprivileged; a test that needs root is a design bug.

## Rules
- No network: DNS tests feed byte slices to the codec or talk to the sinkhole on 127.0.0.1 with an ephemeral port.
- Test behaviour through the public API; `pub(crate)` seams are fine for fakes.
- `unwrap`/`expect` are fine in tests (clippy.toml allows them); give `expect` a message that says what should have happened.
- A failing test that exposes a real bug stays failing; fix the code.
