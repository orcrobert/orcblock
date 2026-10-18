//! The socket server and the tick loop: a Unix socket, or loopback TCP on
//! Windows.

use std::future::Future;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use orcblock_ipc::{Command, Endpoint, ErrorKind, MAX_MESSAGE_LEN, Response, codec};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
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

/// What the server accepts connections on.
#[cfg(unix)]
pub type Listener = tokio::net::UnixListener;

/// What the server accepts connections on.
#[cfg(windows)]
pub type Listener = tokio::net::TcpListener;

/// Binds the socket at `path`, mode 0666 so the unprivileged app can
/// connect. A stale socket file is replaced; a socket another daemon still
/// answers on, or a path that is not a socket, is an error.
#[cfg(unix)]
pub fn bind(path: &Endpoint) -> io::Result<Listener> {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

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
    let listener = Listener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(0o666))?;
    Ok(listener)
}

/// Binds the loopback address `addr`. Any local process may connect, as
/// with the 0666 socket on Unix; another daemon on the port is an error.
#[cfg(windows)]
pub fn bind(addr: &Endpoint) -> io::Result<Listener> {
    if !addr.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{addr} is not a loopback address"),
        ));
    }
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    Listener::from_std(listener)
}

/// Serves requests and ticks until `shutdown` completes, then saves the
/// state and returns. Enforcement is left in place: a block outlives the
/// daemon process.
pub async fn run<C, E>(
    service: Service<C, E>,
    listener: Listener,
    shutdown: impl Future<Output = ()>,
) where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    run_with(service, |_| async move { Some(listener) }, shutdown).await;
}

/// Like [`run`], but binds `endpoint` itself, retrying every `retry` while
/// that fails. Any local process may take the port while the daemon
/// restarts; the block keeps counting down and its enforcement stays
/// verified meanwhile.
pub async fn run_on<C, E>(
    service: Service<C, E>,
    endpoint: Endpoint,
    retry: Duration,
    shutdown: impl Future<Output = ()>,
) where
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    run_with(
        service,
        move |stopped| bind_retrying(endpoint, retry, stopped),
        shutdown,
    )
    .await;
}

async fn bind_retrying(
    endpoint: Endpoint,
    retry: Duration,
    mut stopped: watch::Receiver<bool>,
) -> Option<Listener> {
    let mut failing = false;
    loop {
        match bind(&endpoint) {
            Ok(listener) => {
                if failing {
                    info!(?endpoint, "listening");
                }
                return Some(listener);
            }
            Err(err) if !failing => {
                error!(%err, ?endpoint, "cannot listen; retrying");
                failing = true;
            }
            Err(_) => {}
        }
        tokio::select! {
            () = sleep(retry) => {}
            _ = stopped.changed() => return None,
        }
    }
}

async fn run_with<C, E, L, F>(service: Service<C, E>, listen: L, shutdown: impl Future<Output = ()>)
where
    C: Clock + 'static,
    E: Enforcer + 'static,
    L: FnOnce(watch::Receiver<bool>) -> F,
    F: Future<Output = Option<Listener>> + Send + 'static,
{
    let service = Arc::new(Mutex::new(service));
    let (stop, stopped) = watch::channel(false);
    let listening = listen(stopped.clone());
    let server = tokio::spawn({
        let service = Arc::clone(&service);
        let stopped = stopped.clone();
        async move {
            if let Some(listener) = listening.await {
                serve(listener, service, stopped).await;
            }
        }
    });
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

async fn serve<C, E>(listener: Listener, service: Shared<C, E>, mut stopped: watch::Receiver<bool>)
where
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

/// Answers requests on one connection until it closes, idles or sends
/// something that is not a request. Hanging up on the first undecodable line
/// matters for TCP: a web page can make a browser connect to the port, but
/// its first line is an HTTP request line, so nothing after it is read.
async fn connection<S, C, E>(stream: S, service: Shared<C, E>) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite,
    C: Clock + 'static,
    E: Enforcer + 'static,
{
    let (read, mut write) = tokio::io::split(stream);
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
        let (response, hang_up) = if too_long {
            (
                Response::error(ErrorKind::BadRequest, "Message too long"),
                true,
            )
        } else {
            match codec::decode_request(&line) {
                Ok(request) => (handle(&service, request.command).await?, false),
                Err(err) => (err.into(), true),
            }
        };
        let bytes = codec::encode(&response).map_err(io::Error::other)?;
        write.write_all(&bytes).await?;
        if hang_up {
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
