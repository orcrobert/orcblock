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
/// user-writable bundle is never started as root. Empty without a sidecar
/// (`make sidecar`), which leaves install disabled.
fn pin_helper() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let sidecar = PathBuf::from(format!("binaries/orcblockd-{target}"));
    println!("cargo:rerun-if-changed={}", sidecar.display());
    let hash = std::fs::read(&sidecar)
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
