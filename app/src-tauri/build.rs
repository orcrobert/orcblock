use std::fmt::Write as _;
use std::path::PathBuf;

use sha2::{Digest as _, Sha256};

// Commands are listed so each gets an allow permission and none is callable
// from the webview unless capabilities/default.json grants it.
const COMMANDS: &[&str] = &[
    "status",
    "add_site",
    "remove_site",
    "start_block",
    "drink_potion",
    "favicon",
    "install_helper",
];

fn main() {
    pin_helper();
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(err) = tauri_build::try_build(attributes) {
        panic!("tauri build script failed: {err:#}");
    }
}

/// Bakes the SHA-256 of the bundled daemon into the app. The install runs
/// only a copy whose hash matches, so a helper swapped inside the
/// user-writable bundle is never started as root. The sidecar is
/// `binaries/orcblockd-<target>` (`.exe` on Windows), where `ORCBLOCK_HELPER_TARGET` overrides
/// the build target. Empty without a sidecar (plain dev builds), which
/// leaves install disabled; with the override set the sidecar must exist.
fn pin_helper() {
    // A universal build compiles the app once per architecture, but the
    // bundle ships the fat helper, so `make bundle-universal` names it here.
    println!("cargo:rerun-if-env-changed=ORCBLOCK_HELPER_TARGET");
    let pinned_target = std::env::var("ORCBLOCK_HELPER_TARGET").ok();
    let target = pinned_target
        .clone()
        .or_else(|| std::env::var("TARGET").ok())
        .unwrap_or_default();
    let suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let sidecar = PathBuf::from(format!("binaries/orcblockd-{target}{suffix}"));
    println!("cargo:rerun-if-changed={}", sidecar.display());
    let bytes = match std::fs::read(&sidecar) {
        Ok(bytes) => Some(bytes),
        Err(err) if pinned_target.is_some() => {
            panic!("ORCBLOCK_HELPER_TARGET names {}: {err}", sidecar.display())
        }
        Err(_) => None,
    };
    let hash = bytes
        .map(|bytes| {
            Sha256::digest(bytes)
                .iter()
                .fold(String::with_capacity(64), |mut hex, byte| {
                    // Writing to a String cannot fail.
                    let _ = write!(hex, "{byte:02x}");
                    hex
                })
        })
        .unwrap_or_default();
    println!("cargo:rustc-env=ORCBLOCK_HELPER_SHA256={hash}");
}
