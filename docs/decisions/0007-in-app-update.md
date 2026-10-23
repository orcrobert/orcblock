# 0007: Update the app with tauri-plugin-updater

Status: accepted (2026-10-23)

## Context

Spec 0006 lets the app update itself from a published release instead of a new dmg. Updating means downloading a bundle, proving it comes from our release workflow, unpacking it and swapping `OrcBlock.app` in place. The proof has to be a signature checked against a key built into the app: a checksum fetched from the same place as the download proves nothing. ADR 0001 lets no crate in without an ADR, and none on the allowlist verifies signatures.

Options considered:

1. **Hand-written, on `ureq`, `sha2` and a new ed25519 crate.** Smaller tree, but we would own signature verification, tar extraction into the app's place and the atomic bundle swap, all security-relevant code with macOS quirks.
2. **`tauri-plugin-updater`.** Maintained by the Tauri team on Tauri's release train, MIT or Apache-2.0. Verifies minisign signatures (`minisign-verify`), unpacks the `.app.tar.gz`, swaps the bundle atomically on APFS and restores the old one on failure.
3. **Sparkle.** The standard macOS updater, but an Objective-C framework that needs FFI and EdDSA keys of its own; it does not fit a Tauri app without `unsafe` bindings.

## Decision

Option 2, Rust only, with `default-features = false, features = ["rustls-tls"]`:

- No `zip` (Windows and Linux only) and no `system-proxy`.
- The npm `@tauri-apps/plugin-updater` is not added. The UI calls our own `check_update` and `install_update` commands, and the plugin's own commands are not granted to any window. Relaunching uses `AppHandle::restart`, so no `tauri-plugin-process` either.
- The endpoint and public key are compiled in through `tauri.conf.json`; nothing the UI sends picks the URL or the key.
- The private key never reaches the `build` job, which runs npm and build scripts. The `publish` job, which runs no repository code, signs the tarball with Ubuntu's `minisign` package.

It adds 22 crates to `Cargo.lock`, among them `reqwest`'s rustls stack (`hyper-rustls`, `tokio-rustls`, `rustls-platform-verifier`, `security-framework`), `minisign-verify`, `tar`, `filetime`, `xattr` and `osakit`. `cargo deny` passes. They are linked into the app only; the daemon's tree is unchanged.

## Consequences

- Two HTTP stacks in the app: `ureq` for favicons and `reqwest` for updates. Worth revisiting if the plugin ever allows another client.
- Losing the signing key means every installed app refuses new updates; people then reinstall from the dmg once, after a release with a new public key. Leaking it lets someone publish an update the app accepts, so it lives only in the `release` environment.
- An update can only move forward (the plugin compares versions), and the helper inside still needs the administrator password and the pinned SHA-256 before it runs as root.
