use std::io::{BufRead, BufReader, Write};
use std::thread;

use jiff::Timestamp;
use orcblock_core::{Month, State};

use super::*;
use crate::Mode;

/// A listener on a fresh endpoint, kept alive by the returned guard.
#[cfg(unix)]
fn listen() -> (
    tempfile::TempDir,
    Endpoint,
    std::os::unix::net::UnixListener,
) {
    let dir = tempfile::tempdir().expect("temp dir");
    let endpoint = dir.path().join("d.sock");
    let listener = std::os::unix::net::UnixListener::bind(&endpoint).expect("binds");
    (dir, endpoint, listener)
}

/// A listener on a fresh endpoint.
#[cfg(windows)]
fn listen() -> ((), Endpoint, std::net::TcpListener) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("binds");
    let endpoint = listener.local_addr().expect("bound");
    ((), endpoint, listener)
}

/// An endpoint nothing listens on.
#[cfg(unix)]
fn nobody() -> (tempfile::TempDir, Endpoint) {
    let dir = tempfile::tempdir().expect("temp dir");
    let endpoint = dir.path().join("none.sock");
    (dir, endpoint)
}

/// An endpoint nothing listens on.
#[cfg(windows)]
fn nobody() -> ((), Endpoint) {
    let ((), endpoint, listener) = listen();
    drop(listener);
    ((), endpoint)
}

/// Serves one connection: records the request line and answers with `reply`.
fn serve_once(reply: Option<Response>) -> (impl Sized, Endpoint, thread::JoinHandle<String>) {
    let (guard, endpoint, listener) = listen();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accepts");
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        reader.read_line(&mut request).expect("reads");
        if let Some(reply) = reply {
            reader
                .get_mut()
                .write_all(&codec::encode(&reply).expect("encodes"))
                .expect("writes");
        }
        request
    });
    (guard, endpoint, handle)
}

fn status() -> Status {
    let month = Month::new(2026, 10).expect("valid month");
    Status::new(
        &State::new(month),
        Timestamp::UNIX_EPOCH,
        month,
        Mode::DryRun,
        "0.1.0",
    )
}

#[test]
fn returns_the_status() {
    let (_guard, endpoint, server) = serve_once(Some(Response::Ok { status: status() }));
    let client = Client::new(endpoint);
    assert_eq!(client.add_site("youtube.com").expect("ok"), status());
    assert_eq!(
        server.join().expect("server"),
        "{\"v\":1,\"cmd\":\"add_site\",\"site\":\"youtube.com\"}\n"
    );
}

#[test]
fn surfaces_refusals() {
    let refusal = Response::error(ErrorKind::NotBlocking, "No block is running");
    let (_guard, endpoint, server) = serve_once(Some(refusal));
    let err = Client::new(endpoint).drink_potion().expect_err("refused");
    assert!(
        matches!(
            err,
            ClientError::Refused {
                kind: ErrorKind::NotBlocking,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.to_string(), "No block is running");
    server.join().expect("server");
}

#[test]
fn reports_a_hang_up() {
    let (_guard, endpoint, server) = serve_once(None);
    let err = Client::new(endpoint).status().expect_err("closed");
    assert!(matches!(err, ClientError::Closed), "{err:?}");
    server.join().expect("server");
}

#[test]
fn missing_socket_is_unavailable() {
    let (_guard, endpoint) = nobody();
    let err = Client::new(endpoint).status().expect_err("no daemon");
    assert!(matches!(err, ClientError::Unavailable(_)), "{err:?}");
}

#[cfg(unix)]
#[test]
fn default_endpoint_is_default_socket() {
    assert_eq!(default_endpoint(), Endpoint::from(DEFAULT_SOCKET));
}

#[cfg(windows)]
#[test]
fn default_endpoint_is_default_socket() {
    assert_eq!(default_endpoint().to_string(), DEFAULT_SOCKET);
}

#[test]
fn default_socket_names_the_default_endpoint() {
    assert_eq!(
        DEFAULT_SOCKET.parse::<Endpoint>().expect("parses"),
        default_endpoint()
    );
    assert_ne!(PORT, DEV_PORT);
}
