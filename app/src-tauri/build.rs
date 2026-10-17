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
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(err) = tauri_build::try_build(attributes) {
        panic!("tauri build script failed: {err:#}");
    }
}
