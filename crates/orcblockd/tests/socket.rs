//! The daemon end to end over its socket, in dry-run mode under a temp dir.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;

use orcblock_core::BlockDuration;
use orcblock_ipc::{Client, ClientError, Endpoint, ErrorKind, Mode};
use orcblockd::{Blocked, DryRunEnforcer, Service, Store, SystemClock, server};
use tokio::sync::oneshot;

#[cfg(unix)]
fn endpoint(dir: &Path) -> Endpoint {
    dir.join("d.sock")
}

/// A loopback port that was free a moment ago.
#[cfg(windows)]
fn endpoint(_dir: &Path) -> Endpoint {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .unwrap_or_else(|err| panic!("no free port: {err}"))
}

#[cfg(unix)]
fn connect(endpoint: &Endpoint) -> std::io::Result<std::os::unix::net::UnixStream> {
    std::os::unix::net::UnixStream::connect(endpoint)
}

#[cfg(windows)]
fn connect(endpoint: &Endpoint) -> std::io::Result<std::net::TcpStream> {
    std::net::TcpStream::connect(endpoint)
}

#[tokio::test(flavor = "multi_thread")]
async fn serves_clients_over_the_socket() {
    let dir = tempfile::tempdir().expect("temp dir");
    let socket = endpoint(dir.path());
    let blocked = Blocked::default();
    let service = Service::start(
        Store::new(dir.path().join("state.json")),
        SystemClock::new(),
        DryRunEnforcer::default(),
        blocked.clone(),
        Mode::DryRun,
    )
    .expect("starts");
    let listener = server::bind(&socket).expect("binds");
    assert!(
        server::bind(&socket).is_err(),
        "a live socket is not stolen"
    );

    let (stop, stopped) = oneshot::channel::<()>();
    let daemon = tokio::spawn(server::run(service, listener, async {
        let _ = stopped.await;
    }));

    let client = Client::new(socket).with_timeout(Duration::from_secs(5));
    let calls = tokio::task::spawn_blocking(move || {
        let status = client.status().expect("status");
        assert_eq!(status.mode, Mode::DryRun);
        client.add_site("https://www.YouTube.com/").expect("adds");
        let status = client
            .start_block(BlockDuration::from_minutes(10).expect("valid"))
            .expect("starts");
        assert_eq!(status.block.map(|b| b.remaining_secs), Some(600));
        let err = client.remove_site("youtube.com").expect_err("locked");
        assert!(
            matches!(
                err,
                ClientError::Refused {
                    kind: ErrorKind::Locked,
                    ..
                }
            ),
            "{err:?}"
        );
        client.status().expect("still answering")
    });
    let status = calls.await.expect("client calls");
    assert!(status.block.is_some());
    assert!(blocked.blocks_name("youtube.com"));

    stop.send(()).expect("daemon running");
    daemon.await.expect("daemon stops");
    let saved = Store::new(dir.path().join("state.json"))
        .load()
        .expect("loads")
        .expect("saved");
    assert!(saved.state.is_blocking(), "the block outlives the daemon");
}

#[tokio::test(flavor = "multi_thread")]
async fn hangs_up_on_a_line_that_is_not_a_request() {
    let dir = tempfile::tempdir().expect("temp dir");
    let socket = endpoint(dir.path());
    let service = Service::start(
        Store::new(dir.path().join("state.json")),
        SystemClock::new(),
        DryRunEnforcer::default(),
        Blocked::default(),
        Mode::DryRun,
    )
    .expect("starts");
    let listener = server::bind(&socket).expect("binds");
    let (stop, stopped) = oneshot::channel::<()>();
    let daemon = tokio::spawn(server::run(service, listener, async {
        let _ = stopped.await;
    }));

    // What a browser sends when a web page posts to the daemon's port.
    let client = Client::new(socket);
    let smuggled = client.clone();
    let answers = tokio::task::spawn_blocking(move || {
        let mut stream = connect(smuggled.socket()).expect("connects");
        stream
            .write_all(b"POST / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n{\"v\":1,\"cmd\":\"add_site\",\"site\":\"example.com\"}\n")
            .expect("writes");
        BufReader::new(stream)
            .lines()
            .map(|line| line.expect("reads"))
            .collect::<Vec<_>>()
    })
    .await
    .expect("client");
    assert_eq!(answers.len(), 1, "{answers:?}");
    assert!(answers[0].contains("bad_request"), "{answers:?}");

    let status = tokio::task::spawn_blocking(move || client.status().expect("status"))
        .await
        .expect("client");
    assert!(status.sites.is_empty(), "nothing smuggled in: {status:?}");

    stop.send(()).expect("daemon running");
    daemon.await.expect("daemon stops");
}
