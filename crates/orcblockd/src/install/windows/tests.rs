use std::fs;

use jiff::Timestamp;
use orcblock_core::{BlockDuration, Domain, Month, State};

use super::*;
use crate::enforce::Output;
use crate::enforce::runner::fake::FakeRunner;
use crate::{DryRunEnforcer, Store};

const ELEVATED: &str =
    "\"Mandatory Label\\High Mandatory Level\",\"Label\",\"S-1-16-12288\",\"\"\r\n";
const NOT_ELEVATED: &str =
    "\"Mandatory Label\\Medium Mandatory Level\",\"Label\",\"S-1-16-8192\",\"\"\r\n";

/// A runner where `whoami` prints `groups` and everything else succeeds.
fn runner(groups: &'static str) -> FakeRunner {
    FakeRunner::new(move |line| {
        Ok(Output {
            stdout: if line.starts_with(WHOAMI) {
                groups.to_owned()
            } else {
                String::new()
            },
            stderr: String::new(),
        })
    })
}

fn installed(tmp: &Path) -> Layout {
    let layout = Layout::windows_under(tmp.join("C"));
    let exe = tmp.join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");
    install(&layout, &runner(ELEVATED), &exe).expect("installs");
    layout
}

#[test]
fn recognises_elevation() {
    let cases = [
        ("elevated administrator", ELEVATED, true),
        (
            "SYSTEM",
            "\"Mandatory Label\\System Mandatory Level\",\"Label\",\"S-1-16-16384\",\"\"",
            true,
        ),
        ("standard user", NOT_ELEVATED, false),
        ("nothing", "", false),
    ];
    for (name, groups, want) in cases {
        assert_eq!(is_elevated(groups), want, "{name}");
    }
}

#[test]
fn task_runs_the_helper_as_system_and_keeps_it_alive() {
    let layout = Layout::windows_under(r"C:\");
    let xml = task_xml(&layout);
    for needle in [
        "<UserId>S-1-5-18</UserId>",
        "<BootTrigger>",
        "<Interval>PT5M</Interval>",
        "<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>",
        "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
        "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
        "<Arguments>serve</Arguments>",
        r"<URI>\OrcBlock\orcblockd</URI>",
    ] {
        assert!(xml.contains(needle), "{needle}");
    }
    assert!(xml.contains(&format!("<Command>{}</Command>", layout.helper().display())));
}

#[test]
fn task_xml_escapes_the_helper_path() {
    let xml = task_xml(&Layout::windows_under("C:/a&b<c>"));
    assert!(xml.contains("a&amp;b&lt;c&gt;"), "{xml}");
}

#[test]
fn utf16_starts_with_a_byte_order_mark() {
    assert_eq!(utf16_with_bom("<a"), [0xFF, 0xFE, b'<', 0, b'a', 0]);
}

#[test]
fn install_locks_the_data_dir_copies_the_helper_and_starts_the_task() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");
    let runner = runner(ELEVATED);

    install(&layout, &runner, &exe).expect("installs");
    assert_eq!(fs::read(layout.helper()).expect("copied"), b"binary");
    assert_eq!(
        fs::read(layout.plist()).expect("written"),
        utf16_with_bom(&task_xml(&layout))
    );
    let data = layout.data_dir().display().to_string();
    let xml = layout.plist().display().to_string();
    assert_eq!(
        runner.lines(),
        [
            format!("{WHOAMI} /groups /fo csv /nh"),
            format!("{ICACLS} {data} /setowner *S-1-5-18 /T /C /Q"),
            format!(
                "{ICACLS} {data} /inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F *S-1-5-32-545:(OI)(CI)RX /T /C /Q"
            ),
            format!(r"{SCHTASKS} /End /TN \OrcBlock\orcblockd"),
            format!(r"{SCHTASKS} /Create /TN \OrcBlock\orcblockd /XML {xml} /F"),
            format!(r"{SCHTASKS} /Run /TN \OrcBlock\orcblockd"),
        ]
    );
}

#[test]
fn install_needs_elevation() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    let runner = runner(NOT_ELEVATED);
    let err = install(&layout, &runner, &tmp.path().join("orcblockd.exe")).expect_err("refused");
    assert!(matches!(err, InstallError::NotElevated), "{err:?}");
    assert_eq!(runner.lines().len(), 1, "only whoami ran");
    assert!(!layout.data_dir().exists());
}

#[test]
fn uninstall_is_refused_during_a_block_and_keeps_the_state() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = installed(tmp.path());

    let mut state = State::new(Month::new(2026, 10).expect("valid"));
    state
        .add_site(
            Domain::parse("x.com").expect("valid"),
            Timestamp::UNIX_EPOCH,
        )
        .expect("adds");
    state
        .start_block(
            BlockDuration::from_minutes(30).expect("valid"),
            Timestamp::UNIX_EPOCH,
        )
        .expect("starts");
    let store = Store::new(layout.state_file());
    store.save(&state, Timestamp::UNIX_EPOCH).expect("saves");

    let runner = runner(ELEVATED);
    let err = uninstall(&layout, &runner, &mut DryRunEnforcer::default()).expect_err("refused");
    assert!(matches!(err, InstallError::Blocking(_)), "{err:?}");
    assert!(layout.helper().exists());
    assert_eq!(runner.lines().len(), 1, "nothing was stopped");

    state.advance(Duration::from_mins(30));
    store.save(&state, Timestamp::UNIX_EPOCH).expect("saves");
    let mut enforcer = DryRunEnforcer::default();
    uninstall(&layout, &runner, &mut enforcer).expect("uninstalls");
    assert!(!layout.helper().exists());
    assert!(
        !layout.helper().parent().expect("has a dir").exists(),
        "the empty helper directory goes too"
    );
    assert!(!layout.plist().exists());
    assert!(layout.state_file().exists(), "the potion ledger survives");
    assert_eq!(enforcer.calls(), [crate::enforce::DryRunCall::Clear]);
    assert!(
        runner
            .lines()
            .contains(&format!(r"{SCHTASKS} /Delete /TN \OrcBlock\orcblockd /F"))
    );
}
