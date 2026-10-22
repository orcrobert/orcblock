use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::thread;

use jiff::Timestamp;
use orcblock_core::{ListId, Month, State};

use super::*;
use crate::Mode;

/// Serves one connection: records the request line and answers with `reply`.
fn serve_once(reply: Option<Response>) -> (tempfile::TempDir, thread::JoinHandle<String>) {
    let dir = tempfile::tempdir().expect("temp dir");
    let listener = UnixListener::bind(dir.path().join("d.sock")).expect("binds");
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
    (dir, handle)
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
    let (dir, server) = serve_once(Some(Response::Ok { status: status() }));
    let client = Client::new(dir.path().join("d.sock"));
    assert_eq!(
        client.add_site(ListId::new(1), "youtube.com").expect("ok"),
        status()
    );
    assert_eq!(
        server.join().expect("server"),
        "{\"v\":3,\"cmd\":\"add_site\",\"list\":1,\"site\":\"youtube.com\"}\n"
    );
}

#[test]
fn surfaces_refusals() {
    let refusal = Response::error(ErrorKind::NotBlocking, "No block is running");
    let (dir, server) = serve_once(Some(refusal));
    let err = Client::new(dir.path().join("d.sock"))
        .drink_potion()
        .expect_err("refused");
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
    let (dir, server) = serve_once(None);
    let err = Client::new(dir.path().join("d.sock"))
        .status()
        .expect_err("closed");
    assert!(matches!(err, ClientError::Closed), "{err:?}");
    server.join().expect("server");
}

#[test]
fn missing_socket_is_unavailable() {
    let dir = tempfile::tempdir().expect("temp dir");
    let err = Client::new(dir.path().join("none.sock"))
        .status()
        .expect_err("no daemon");
    assert!(matches!(err, ClientError::Unavailable(_)), "{err:?}");
}
