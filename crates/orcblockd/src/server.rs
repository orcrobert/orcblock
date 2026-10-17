//! The Unix socket server and the tick loop.

use std::fs::{self, Permissions};
use std::future::Future;
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use orcblock_ipc::{Command, ErrorKind, MAX_MESSAGE_LEN, Response, codec};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Semaphore, watch};
use tokio::time::{MissedTickBehavior, interval, sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::{Clock, Enforcer, Service};

/// How often the tick loop advances the clock.
pub const TICK: Duration = Duration::from_secs(1);

/// A client that sends nothing for this long is disconnected.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Connections served at once; more are closed immediately.
const MAX_CONNECTIONS: usize = 32;

/// The service shared by connections and the tick loop. Locked only inside
/// `spawn_blocking`, because commands may run system programs.
type Shared<C, E> = Arc<Mutex<Service<C, E>>>;

/// Binds the socket at `path`, mode 0666 so the unprivileged app can
/// connect. A stale socket file is replaced; a socket another daemon still
/// answers on, or a path that is not a socket, is an error.
pub fn bind(path: &Path) -> io::Result<UnixListener> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_socket() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a socket", path.display()),
            ));
        }
        Ok(_) => {
            if std::os::unix::net::UnixStream::connect(path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("another orcblockd is listening on {}", path.display()),
                ));
            }
            fs::remove_file(path)?;
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(0o666))?;
    Ok(listener)
}

/// Serves requests and ticks until `shutdown` completes, then saves the
/// state and returns. Enforcement is left in place: a block outlives the
/// daemon process.
pub async fn run<C, E>(
    service: Service<C, E>,
    listener: UnixListener,
    shutdown: impl Future<Output = ()>,
) where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let service = Arc::new(Mutex::new(service));
    let (stop, stopped) = watch::channel(false);
    let server = tokio::spawn(serve(listener, Arc::clone(&service), stopped.clone()));
    let ticker = tokio::spawn(tick_loop(Arc::clone(&service), stopped));
    info!("orcblockd ready");

    shutdown.await;
    info!("shutting down");
    let _ = stop.send(true);
    for task in [server, ticker] {
        if let Err(err) = task.await {
            error!(%err, "task failed during shutdown");
        }
    }
    let service = Arc::clone(&service);
    let saved = tokio::task::spawn_blocking(move || lock(&service).save()).await;
    match saved {
        Ok(Ok(())) => {}
        Ok(Err(err)) => error!(%err, "could not save the state on shutdown"),
        Err(err) => error!(%err, "save failed during shutdown"),
    }
}

async fn serve<C, E>(
    listener: UnixListener,
    service: Shared<C, E>,
    mut stopped: watch::Receiver<bool>,
) where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        tokio::select! {
            _ = stopped.changed() => return,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
                        warn!("too many connections; closing one");
                        continue;
                    };
                    let service = Arc::clone(&service);
                    tokio::spawn(async move {
                        if let Err(err) = connection(stream, service).await {
                            debug!(%err, "connection closed");
                        }
                        drop(permit);
                    });
                }
                Err(err) => {
                    warn!(%err, "accept failed");
                    sleep(Duration::from_millis(100)).await;
                }
            },
        }
    }
}

async fn connection<C, E>(stream: UnixStream, service: Shared<C, E>) -> io::Result<()>
where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let limit = u64::try_from(MAX_MESSAGE_LEN).unwrap_or(u64::MAX);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = timeout(
            IDLE_TIMEOUT,
            (&mut reader).take(limit).read_until(b'\n', &mut line),
        )
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        if read == 0 {
            return Ok(());
        }
        let too_long = line.last() != Some(&b'\n') && line.len() >= MAX_MESSAGE_LEN;
        let response = if too_long {
            Response::error(ErrorKind::BadRequest, "Message too long")
        } else {
            match codec::decode_request(&line) {
                Ok(request) => handle(&service, request.command).await?,
                Err(err) => err.into(),
            }
        };
        let bytes = codec::encode(&response).map_err(io::Error::other)?;
        write.write_all(&bytes).await?;
        if too_long {
            return Ok(());
        }
    }
}

async fn handle<C, E>(service: &Shared<C, E>, command: Command) -> io::Result<Response>
where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let service = Arc::clone(service);
    tokio::task::spawn_blocking(move || lock(&service).handle(command))
        .await
        .map_err(io::Error::other)
}

async fn tick_loop<C, E>(service: Shared<C, E>, mut stopped: watch::Receiver<bool>)
where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let mut ticks = interval(TICK);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = stopped.changed() => return,
            _ = ticks.tick() => {
                let service = Arc::clone(&service);
                if let Err(err) = tokio::task::spawn_blocking(move || lock(&service).tick()).await {
                    error!(%err, "tick failed");
                }
            }
        }
    }
}

fn lock<C, E>(service: &Shared<C, E>) -> MutexGuard<'_, Service<C, E>> {
    service.lock().unwrap_or_else(PoisonError::into_inner)
}
