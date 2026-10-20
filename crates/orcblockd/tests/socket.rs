//! The daemon end to end over its socket, in dry-run mode under a temp dir.

use std::time::Duration;

use orcblock_core::{BlockDuration, BundleId};
use orcblock_ipc::{Client, ClientError, ErrorKind, Mode};
use orcblockd::{
    AppGuard, Blocked, CommandRunner, DryRunEnforcer, DryRunTerminator, EnforceError, Output,
    Service, Store, SystemClock, server,
};
use tokio::sync::oneshot;

/// A process table with nothing in it; tests never list real processes.
struct NoProcesses;

impl CommandRunner for NoProcesses {
    fn run(&self, _: &str, _: &[&str], _: Option<&[u8]>) -> Result<Output, EnforceError> {
        Ok(Output::default())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn serves_clients_over_the_socket() {
    let dir = tempfile::tempdir().expect("temp dir");
    let socket = dir.path().join("d.sock");
    let blocked = Blocked::default();
    let service = Service::start(
        Store::new(dir.path().join("state.json")),
        SystemClock,
        DryRunEnforcer::default(),
        blocked.clone(),
        AppGuard::new(NoProcesses, DryRunTerminator),
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

    let client = Client::new(&socket).with_timeout(Duration::from_secs(5));
    let calls = tokio::task::spawn_blocking(move || {
        let status = client.status().expect("status");
        assert_eq!(status.mode, Mode::DryRun);
        client.add_site("https://www.YouTube.com/").expect("adds");
        client
            .add_app("com.hnc.Discord", "Discord")
            .expect("adds app");
        let status = client
            .start_block(BlockDuration::from_minutes(10).expect("valid"))
            .expect("starts");
        let block = status.block.expect("blocking");
        assert_eq!(block.remaining_secs, 600);
        assert_eq!(
            block.apps.iter().map(BundleId::as_str).collect::<Vec<_>>(),
            ["com.hnc.discord"]
        );
        let err = client.remove_app("com.hnc.discord").expect_err("locked");
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
