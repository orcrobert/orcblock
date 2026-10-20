//! The `OrcBlock` desktop app: Tauri commands that forward to the daemon, a
//! background task that streams its status to the window, and favicons.

mod favicon;
mod helper;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use orcblock_core::{BlockDuration, Domain};
use orcblock_ipc::codec::CodecError;
use orcblock_ipc::{Client, ClientError, Endpoint, ErrorKind, Status, default_endpoint};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::favicon::Favicons;

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

/// Installs the bundled helper through the system's administrator prompt.
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
    if helper::HELPER_SHA256.is_empty() {
        return Err(fail(
            "This build has no bundled helper; build the app with `make bundle`".into(),
        ));
    }
    let helper = helper::bundled().ok_or_else(|| {
        fail("The helper is missing from the app bundle; reinstall OrcBlock".into())
    })?;
    tauri::async_runtime::spawn_blocking(move || helper::install(&helper))
        .await
        .map_err(|err| fail(err.to_string()))?
        .map_err(|err| fail(err.to_string()))
}

/// Streams the daemon status to the window: every second during a block,
/// every two seconds otherwise.
fn spawn_status_task(app: AppHandle, client: Client) {
    thread::spawn(move || {
        loop {
            let (event, interval) = match client.status() {
                Ok(status) => {
                    let interval = if status.block.is_some() { 1 } else { 2 };
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

/// Starts the app.
pub fn run() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let socket = std::env::var_os("ORCBLOCK_SOCKET");
    // Set at all means a development build, even if the value is wrong:
    // install stays disabled rather than falling back to the live helper.
    let dev = socket.is_some();
    let endpoint = socket.map_or_else(default_endpoint, |socket| {
        socket
            .to_string_lossy()
            .parse::<Endpoint>()
            .unwrap_or_else(|err| {
                tracing::error!(?socket, %err, "ORCBLOCK_SOCKET is not a daemon address");
                std::process::exit(2)
            })
    });
    let client = Client::new(endpoint);

    let result = tauri::Builder::default()
        .setup(move |app| {
            let cache_dir = app.path().app_cache_dir()?.join("favicons");
            app.manage(Arc::new(AppState {
                client: client.clone(),
                favicons: Favicons::new(cache_dir),
                dev,
            }));
            spawn_status_task(app.handle().clone(), client);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            add_site,
            remove_site,
            start_block,
            drink_potion,
            favicon,
            install_helper
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        tracing::error!(%err, "OrcBlock stopped");
    }
}

#[cfg(test)]
mod tests;
