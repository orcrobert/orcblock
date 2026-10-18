use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use jiff::Timestamp;
use orcblock_core::{BlockDuration, Domain, Month, State};

use super::*;
use crate::DryRunEnforcer;
use crate::enforce::Output;
use crate::enforce::runner::fake::FakeRunner;

fn ok_runner() -> FakeRunner {
    FakeRunner::new(|_| Ok(Output::default()))
}

#[test]
fn plist_runs_the_helper_at_load_and_keeps_it_alive() {
    let plist = plist(&Layout::system());
    assert!(plist.contains("<string>com.orcrobert.orcblockd</string>"));
    assert!(plist.contains("<string>/Library/PrivilegedHelperTools/com.orcrobert.orcblockd</string>\n\t\t<string>serve</string>"));
    assert!(plist.contains("<key>RunAtLoad</key>\n\t<true/>"));
    assert!(plist.contains("<key>KeepAlive</key>\n\t<true/>"));
}

#[test]
fn install_copies_the_helper_and_bootstraps_it() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::under(tmp.path().join("root"));
    let exe = tmp.path().join("orcblockd");
    fs::write(&exe, b"binary").expect("writes");
    let runner = ok_runner();

    install(&layout, &runner, &exe).expect("installs");
    assert_eq!(fs::read(layout.helper()).expect("copied"), b"binary");
    #[cfg(unix)]
    {
        let mode = fs::metadata(layout.helper())
            .expect("exists")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
    assert_eq!(
        fs::read_to_string(layout.plist()).expect("written"),
        plist(&layout)
    );
    assert!(layout.data_dir().is_dir());
    assert_eq!(
        runner.lines(),
        [
            "/bin/launchctl bootout system/com.orcrobert.orcblockd".to_owned(),
            format!(
                "/bin/launchctl bootstrap system {}",
                layout.plist().display()
            ),
            "/bin/launchctl enable system/com.orcrobert.orcblockd".to_owned(),
        ]
    );
}

#[test]
fn uninstall_is_refused_during_a_block_and_keeps_the_state() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::under(tmp.path());
    let exe = tmp.path().join("orcblockd");
    fs::write(&exe, b"binary").expect("writes");
    install(&layout, &ok_runner(), &exe).expect("installs");

    let mut state = State::new(Month::new(2026, 10).expect("valid"));
    state
        .add_site(
            Domain::parse("x.com").expect("valid"),
            Timestamp::UNIX_EPOCH,
        )
        .expect("adds");
    state
        .start_block(
            BlockDuration::from_minutes(90).expect("valid"),
            Timestamp::UNIX_EPOCH,
        )
        .expect("starts");
    let store = Store::new(layout.state_file());
    store.save(&state, Timestamp::UNIX_EPOCH).expect("saves");

    let runner = ok_runner();
    let err = uninstall(&layout, &runner, &mut DryRunEnforcer::default()).expect_err("refused");
    assert_eq!(
        err.to_string(),
        "a block is running for another 1 h 30 min; OrcBlock can be uninstalled once it ends"
    );
    assert!(layout.helper().exists());
    assert_eq!(runner.lines(), Vec::<String>::new(), "nothing was stopped");

    state.advance(Duration::from_mins(90));
    store.save(&state, Timestamp::UNIX_EPOCH).expect("saves");
    let mut enforcer = DryRunEnforcer::default();
    uninstall(&layout, &runner, &mut enforcer).expect("uninstalls");
    assert!(!layout.helper().exists());
    assert!(!layout.plist().exists());
    assert!(layout.state_file().exists(), "the potion ledger survives");
    assert_eq!(enforcer.calls(), [crate::enforce::DryRunCall::Clear]);
}

#[test]
fn remaining_time_reads_well() {
    assert_eq!(format_remaining(Duration::from_secs(1)), "1 min");
    assert_eq!(format_remaining(Duration::from_hours(1)), "1 h");
    assert_eq!(format_remaining(Duration::from_mins(61)), "1 h 1 min");
}
