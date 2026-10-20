//! The `OrcBlock` desktop app: Tauri commands that forward to the daemon, a
//! background task that streams its status to the window, and favicons.

mod apps;
mod favicon;
mod shield;

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use orcblock_core::{BlockDuration, Domain};
use orcblock_ipc::codec::CodecError;
use orcblock_ipc::{Client, ClientError, DEFAULT_SOCKET, ErrorKind, Status};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::apps::{Apps, InstalledApp, SystemRunner};
use crate::favicon::Favicons;
use crate::shield::ShieldTracker;

/// The label of the orc shield window; the UI picks its route by it.
const SHIELD: &str = "shield";

/// The label of the main window, hidden until the app decides to show it.
const MAIN: &str = "main";

/// The argument the daemon starts the app with to show only the shield.
const SHIELD_ARG: &str = "--shield";

/// Whether the daemon started the app just to show the shield: then the
/// main window stays hidden.
fn woken<S: AsRef<str>>(args: impl IntoIterator<Item = S>) -> bool {
    args.into_iter()
        .skip(1)
        .any(|arg| arg.as_ref() == SHIELD_ARG)
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Why a command failed, as the UI receives it.
#[derive(Debug, Clone, Serialize)]
struct AppError {
    /// `unavailable` when the helper is not running, `helper_outdated` when
    /// it speaks an older protocol, otherwise the daemon's error kind in
    /// snake case.
    kind: String,
    message: String,
}

impl From<ClientError> for AppError {
    fn from(err: ClientError) -> Self {
        let kind = match &err {
            ClientError::Unavailable(_) => "unavailable".to_owned(),
            ClientError::Refused {
                kind: ErrorKind::Version,
                ..
            }
            | ClientError::Codec(CodecError::Json(_)) => "helper_outdated".to_owned(),
            ClientError::Refused { kind, .. } => kind_name(*kind),
            _ => "internal".to_owned(),
        };
        Self {
            kind,
            message: err.to_string(),
        }
    }
}

fn kind_name(kind: ErrorKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internal".to_owned())
}

/// What the status task emits on the `status` event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum StatusEvent {
    Ok { status: Box<Status> },
    Err { error: AppError },
}

struct AppState {
    client: Client,
    favicons: Favicons,
    apps: Apps<SystemRunner>,
    dev: bool,
}

type Shared<'a> = State<'a, Arc<AppState>>;

async fn call<T: Send + 'static>(
    state: &Shared<'_>,
    f: impl FnOnce(&Client) -> Result<T, ClientError> + Send + 'static,
) -> Result<T, AppError> {
    let state = Arc::clone(state);
    tauri::async_runtime::spawn_blocking(move || f(&state.client))
        .await
        .map_err(|err| AppError {
            kind: "internal".into(),
            message: err.to_string(),
        })?
        .map_err(AppError::from)
}

#[tauri::command]
async fn status(state: Shared<'_>) -> Result<Status, AppError> {
    call(&state, Client::status).await
}

#[tauri::command]
async fn add_site(state: Shared<'_>, site: String) -> Result<Status, AppError> {
    call(&state, move |client| client.add_site(&site)).await
}

#[tauri::command]
async fn remove_site(state: Shared<'_>, site: String) -> Result<Status, AppError> {
    call(&state, move |client| client.remove_site(&site)).await
}

#[tauri::command]
async fn start_block(state: Shared<'_>, minutes: u64) -> Result<Status, AppError> {
    let duration = BlockDuration::from_minutes(minutes).map_err(|err| AppError {
        kind: "invalid_duration".into(),
        message: err.to_string(),
    })?;
    call(&state, move |client| client.start_block(duration)).await
}

#[tauri::command]
async fn drink_potion(state: Shared<'_>) -> Result<Status, AppError> {
    call(&state, Client::drink_potion).await
}

/// The site's icon as a data URL, or `null` for a monogram.
#[tauri::command]
async fn favicon(state: Shared<'_>, domain: String) -> Result<Option<String>, AppError> {
    let Ok(domain) = Domain::parse(&domain) else {
        return Ok(None);
    };
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.favicons.get(&domain))
        .await
        .map_err(|err| AppError {
            kind: "internal".into(),
            message: err.to_string(),
        })
}

#[tauri::command]
async fn add_app(state: Shared<'_>, bundle_id: String, name: String) -> Result<Status, AppError> {
    call(&state, move |client| client.add_app(&bundle_id, &name)).await
}

#[tauri::command]
async fn remove_app(state: Shared<'_>, bundle_id: String) -> Result<Status, AppError> {
    call(&state, move |client| client.remove_app(&bundle_id)).await
}

/// Runs `f` on the app finder off the async runtime: it starts programs
/// and reads the disk.
async fn with_apps<T: Send + 'static>(
    state: &Shared<'_>,
    f: impl FnOnce(&Apps<SystemRunner>) -> T + Send + 'static,
) -> Result<T, AppError> {
    let state = Arc::clone(state);
    tauri::async_runtime::spawn_blocking(move || f(&state.apps))
        .await
        .map_err(|err| AppError {
            kind: "internal".into(),
            message: err.to_string(),
        })
}

/// The apps in the usual folders, sorted by name.
#[tauri::command]
async fn installed_apps(state: Shared<'_>) -> Result<Vec<InstalledApp>, AppError> {
    with_apps(&state, Apps::installed).await
}

/// The bundle IDs of the apps open now.
#[tauri::command]
async fn running_apps(state: Shared<'_>) -> Result<Vec<String>, AppError> {
    with_apps(&state, Apps::running).await
}

/// The app's icon as a data URL, or `null` for a monogram.
#[tauri::command]
async fn app_icon(state: Shared<'_>, bundle_id: String) -> Result<Option<String>, AppError> {
    with_apps(&state, move |apps| apps.icon(&bundle_id)).await
}

/// The app bundle at `path`, as dropped from Finder or the Dock.
#[tauri::command]
async fn app_from_path(state: Shared<'_>, path: String) -> Result<InstalledApp, AppError> {
    with_apps(&state, move |apps| {
        apps.at_path(std::path::Path::new(&path))
    })
    .await?
    .ok_or_else(|| AppError {
        kind: "invalid_app".into(),
        message: "That is not an app; drop an app from Applications or the Dock".into(),
    })
}

/// Lets the user choose an app in the macOS file dialog; `null` when they
/// cancel.
#[tauri::command]
async fn pick_app_in_finder(state: Shared<'_>) -> Result<Option<InstalledApp>, AppError> {
    with_apps(&state, Apps::pick).await
}

/// Installs the bundled helper through the macOS administrator prompt.
#[tauri::command]
async fn install_helper(state: Shared<'_>) -> Result<(), AppError> {
    let fail = |message: String| AppError {
        kind: "install".into(),
        message,
    };
    if state.dev {
        return Err(fail(
            "Dev mode talks to the dev daemon: run `make daemon-dev` in the repository".into(),
        ));
    }
    if HELPER_SHA256.is_empty() {
        return Err(fail(
            "This build has no bundled helper; build the app with `make bundle`".into(),
        ));
    }
    let helper = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("orcblockd")))
        .filter(|path| path.is_file())
        .ok_or_else(|| {
            fail("The helper is missing from the app bundle; reinstall OrcBlock".into())
        })?;
    let script = install_script(&helper, HELPER_SHA256);
    tauri::async_runtime::spawn_blocking(move || {
        Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()
    })
    .await
    .map_err(|err| fail(err.to_string()))?
    .map_err(|err| {
        fail(format!(
            "Could not ask for the administrator password: {err}"
        ))
    })
    .and_then(|output| {
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(fail(if stderr.contains("-128") {
                "Installation cancelled".into()
            } else {
                format!("Installation failed: {}", stderr.trim())
            }))
        }
    })
}

/// SHA-256 of the daemon bundled with this build, or empty without one.
const HELPER_SHA256: &str = env!("ORCBLOCK_HELPER_SHA256");

/// The `AppleScript` that installs the helper as root. The bundle is
/// writable by the user, so root first copies the helper into its own
/// temp dir, checks the copy against `sha256` and runs only that copy:
/// nothing swapped in before or during the password prompt gets root.
fn install_script(helper: &std::path::Path, sha256: &str) -> String {
    let quote = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    let (before, after) = install_shell(sha256);
    format!(
        "do shell script \"{}\" & quoted form of \"{}\" & \"{}\" with prompt \"OrcBlock needs to install its helper to keep blocks in place.\" with administrator privileges",
        quote(before),
        quote(&helper.to_string_lossy()),
        quote(&after),
    )
}

/// The shell that runs as root, split around the single-quoted helper path.
fn install_shell(sha256: &str) -> (&'static str, String) {
    let before = "t=$(/usr/bin/mktemp -d) || exit 1; /bin/cp -- ";
    let after = format!(
        " \"$t/orcblockd\" || {{ /bin/rm -rf \"$t\"; exit 1; }}; \
         if [ \"$(/usr/bin/shasum -a 256 \"$t/orcblockd\" | /usr/bin/cut -c1-64)\" != \"{sha256}\" ]; then \
         /bin/rm -rf \"$t\"; echo \"the helper does not match this copy of OrcBlock\" >&2; exit 1; fi; \
         \"$t/orcblockd\" install; s=$?; /bin/rm -rf \"$t\"; exit $s"
    );
    (before, after)
}

/// Streams the daemon status to the window: every second during a block,
/// every two seconds otherwise.
fn spawn_status_task(app: AppHandle, client: Client, woken: bool) {
    thread::spawn(move || {
        let mut shield = ShieldTracker::new(woken);
        // Until the first status: a woken app with nothing to show (the
        // block ended meanwhile) opens its window rather than sit hidden.
        let mut waking = woken;
        loop {
            let (event, interval) = match client.status() {
                Ok(status) => {
                    let interval = if status.block.is_some() { 1 } else { 2 };
                    let new = shield.next(&status.blocked_launches).is_some();
                    if new && status.block.is_some() {
                        show_shield(&app);
                    } else if waking {
                        show_main(&app);
                    }
                    waking = false;
                    (
                        StatusEvent::Ok {
                            status: Box::new(status),
                        },
                        interval,
                    )
                }
                Err(err) => (StatusEvent::Err { error: err.into() }, 2),
            };
            if app.emit("status", event).is_err() {
                return;
            }
            thread::sleep(Duration::from_secs(interval));
        }
    });
}

/// Opens the orc shield over everything, or brings it back with a fresh
/// timer when it is already open.
fn show_shield(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SHIELD) {
        let _ = window.emit_to(SHIELD, "shield", ());
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SHIELD, WebviewUrl::App("index.html".into()))
        .title("OrcBlock")
        .inner_size(420.0, 300.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .center()
        .focused(true)
        .build();
    if let Err(err) = built {
        tracing::warn!(%err, "could not open the shield");
    }
}

/// Starts the app.
pub fn run() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let socket = std::env::var_os("ORCBLOCK_SOCKET").map(PathBuf::from);
    let dev = socket.is_some();
    let client = Client::new(socket.unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET)));

    let woken = woken(std::env::args());

    let built = tauri::Builder::default()
        .setup(move |app| {
            let cache_dir = app.path().app_cache_dir()?;
            let home = app.path().home_dir().ok();
            app.manage(Arc::new(AppState {
                client: client.clone(),
                favicons: Favicons::new(cache_dir.join("favicons")),
                apps: Apps::new(
                    SystemRunner,
                    apps::app_folders(home.as_deref()),
                    cache_dir.join("app-icons"),
                ),
                dev,
            }));
            if !woken {
                show_main(app.handle());
            }
            spawn_status_task(app.handle().clone(), client, woken);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            add_site,
            remove_site,
            start_block,
            drink_potion,
            favicon,
            install_helper,
            add_app,
            remove_app,
            installed_apps,
            running_apps,
            app_icon,
            app_from_path,
            pick_app_in_finder
        ])
        .build(tauri::generate_context!());
    match built {
        // A click on the Dock icon brings back the main window, which a
        // woken app never showed.
        Ok(app) => app.run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        }),
        Err(err) => tracing::error!(%err, "OrcBlock stopped"),
    }
}

#[cfg(test)]
mod tests;
