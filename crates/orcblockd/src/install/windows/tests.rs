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

/// The SDDL icacls leaves on a locked data directory.
const LOCKED: &str = "O:SYG:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)";
/// A directory a user made in `ProgramData` and kept write access to.
const USER_OWNED: &str = "O:S-1-5-21-1-2-3-1001G:S-1-5-21-1-2-3-513D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)(A;OICI;FA;;;S-1-5-21-1-2-3-1001)";

/// A runner where `whoami` prints `groups` and everything else succeeds.
fn runner(groups: &'static str) -> FakeRunner {
    runner_with_acl(groups, LOCKED)
}

/// Like [`runner`], with PowerShell printing `sddl` for the data directory.
fn runner_with_acl(groups: &'static str, sddl: &'static str) -> FakeRunner {
    FakeRunner::new(move |line| {
        let stdout = if line.starts_with(WHOAMI) {
            groups
        } else if line.starts_with(crate::enforce::windows::POWERSHELL) {
            sddl
        } else {
            ""
        };
        Ok(Output {
            stdout: stdout.to_owned(),
            stderr: String::new(),
        })
    })
}

/// The siblings of the data directory that install moved aside.
fn set_aside(layout: &Layout) -> Vec<std::path::PathBuf> {
    let data_dir = layout.data_dir();
    let parent = data_dir.parent().expect("has a parent");
    fs::read_dir(parent)
        .expect("lists")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.to_string_lossy().contains(".untrusted-"))
        .collect()
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
fn recognises_a_locked_data_directory() {
    let cases = [
        ("as icacls leaves it", LOCKED, true),
        (
            "owned by Administrators",
            "O:BAG:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
            true,
        ),
        ("with an audit part", &format!("{LOCKED}S:AI"), true),
        (
            "a deny entry",
            "O:SYD:P(D;OICI;FA;;;WD)(A;OICI;FA;;;SY)",
            true,
        ),
        ("owned by a user", USER_OWNED, false),
        (
            "a user may write",
            "O:SYG:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;0x1301bf;;;BU)",
            false,
        ),
        (
            "inherits from ProgramData",
            "O:SYG:SYD:AI(A;OICIID;FA;;;SY)(A;CIID;DCLCRPCR;;;BU)",
            false,
        ),
        ("everyone may write", "O:SYD:P(A;OICI;FA;;;WD)", false),
        ("an object entry", "O:SYD:P(OA;;FA;x;;BU)", false),
        ("a malformed entry", "O:SYD:P(A;OICI;FA)", false),
        ("no access list", "O:SYG:SY", false),
        ("empty", "", false),
    ];
    for (name, sddl, want) in cases {
        assert_eq!(is_locked_sddl(sddl), want, "{name}");
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
            format!("{ICACLS} {data} /setowner *S-1-5-18 /C /Q"),
            format!(
                "{ICACLS} {data} /inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F *S-1-5-32-545:(OI)(CI)RX /C /Q"
            ),
            format!(r"{SCHTASKS} /End /TN \OrcBlock\orcblockd"),
            format!(r"{SCHTASKS} /Create /TN \OrcBlock\orcblockd /XML {xml} /F"),
            format!(r"{SCHTASKS} /Run /TN \OrcBlock\orcblockd"),
        ]
    );
}

#[test]
fn reinstalling_keeps_a_locked_data_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = installed(tmp.path());
    fs::write(layout.state_file(), b"{}").expect("writes");
    let exe = tmp.path().join("orcblockd.exe");
    let runner = runner(ELEVATED);

    install(&layout, &runner, &exe).expect("reinstalls");
    assert_eq!(fs::read(layout.state_file()).expect("kept"), b"{}");
    assert_eq!(set_aside(&layout), Vec::<std::path::PathBuf>::new());
    let checked = runner
        .calls()
        .into_iter()
        .find(|call| call.program == crate::enforce::windows::POWERSHELL)
        .expect("the ACL was read");
    assert_eq!(
        checked.stdin,
        Some(format!("{}\n", layout.data_dir().display()))
    );
}

#[test]
fn install_moves_a_data_directory_a_user_made_aside() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    fs::create_dir_all(layout.data_dir()).expect("creates");
    let planted = layout.data_dir().join("state.json.tmp");
    fs::write(&planted, b"planted").expect("writes");
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");

    install(&layout, &runner_with_acl(ELEVATED, USER_OWNED), &exe).expect("installs");
    assert!(!planted.exists(), "nothing of theirs is used");
    let aside = set_aside(&layout);
    assert_eq!(aside.len(), 1);
    assert_eq!(
        fs::read(aside[0].join("state.json.tmp")).expect("moved"),
        b"planted"
    );
}

#[test]
fn install_keeps_an_unlocked_data_directory_that_holds_state() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    fs::create_dir_all(layout.data_dir()).expect("creates");
    fs::write(layout.state_file(), b"{}").expect("writes");
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");
    let runner = runner_with_acl(ELEVATED, USER_OWNED);

    let err = install(&layout, &runner, &exe).expect_err("refused");
    assert!(matches!(err, InstallError::UnlockedState(_)), "{err:?}");
    assert_eq!(fs::read(layout.state_file()).expect("kept"), b"{}");
    assert_eq!(set_aside(&layout), Vec::<std::path::PathBuf>::new());
    assert!(
        runner
            .lines()
            .iter()
            .all(|line| !line.starts_with(SCHTASKS)),
        "the running helper is left alone"
    );
}

#[test]
fn install_moves_a_data_directory_aside_when_its_acl_cannot_be_read() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    fs::create_dir_all(layout.data_dir()).expect("creates");
    let runner = FakeRunner::new(|line| {
        if line.starts_with(crate::enforce::windows::POWERSHELL) {
            return Err("access denied".to_owned());
        }
        Ok(Output {
            stdout: if line.starts_with(WHOAMI) {
                ELEVATED.to_owned()
            } else {
                String::new()
            },
            stderr: String::new(),
        })
    });
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");

    install(&layout, &runner, &exe).expect("installs");
    assert_eq!(set_aside(&layout).len(), 1);
}

#[cfg(unix)]
#[test]
fn install_moves_a_linked_data_directory_aside_without_following_it() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    let elsewhere = tmp.path().join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("creates");
    fs::create_dir_all(layout.data_dir().parent().expect("has a parent")).expect("creates");
    std::os::unix::fs::symlink(&elsewhere, layout.data_dir()).expect("links");
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");
    let runner = runner(ELEVATED);

    install(&layout, &runner, &exe).expect("installs");
    assert!(
        !fs::symlink_metadata(layout.data_dir())
            .expect("exists")
            .file_type()
            .is_symlink()
    );
    assert_eq!(set_aside(&layout).len(), 1);
    assert_eq!(fs::read_dir(&elsewhere).expect("lists").count(), 0);
    assert!(
        runner
            .lines()
            .iter()
            .all(|line| !line.starts_with(crate::enforce::windows::POWERSHELL)),
        "a link is never trusted"
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
    assert_eq!(
        runner.lines()[1..],
        [
            format!(r"{SCHTASKS} /End /TN \OrcBlock\orcblockd"),
            format!(r"{SCHTASKS} /Run /TN \OrcBlock\orcblockd"),
        ],
        "stopped for the check, then started again"
    );

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

#[test]
fn install_fails_when_files_appear_before_the_lock_down() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = Layout::windows_under(tmp.path().join("C"));
    let planted = layout.data_dir().join("state.json.tmp");
    let runner = FakeRunner::new(move |line| {
        if line.starts_with(ICACLS) && !planted.exists() {
            fs::write(&planted, b"link").expect("plants");
        }
        Ok(Output {
            stdout: if line.starts_with(WHOAMI) {
                ELEVATED.to_owned()
            } else {
                String::new()
            },
            stderr: String::new(),
        })
    });
    let exe = tmp.path().join("orcblockd.exe");
    fs::write(&exe, b"binary").expect("writes");

    let err = install(&layout, &runner, &exe).expect_err("refused");
    assert!(matches!(err, InstallError::Tampered(_)), "{err:?}");
    assert!(!layout.data_dir().exists());
    assert!(!layout.helper().exists(), "nothing was installed");
}

#[test]
fn elevation_needs_the_label_as_a_whole_field() {
    assert!(!is_elevated(
        "\"Group\",\"Alias\",\"S-1-16-12288-1\",\"\"\r\n"
    ));
}

#[test]
fn reinstalling_from_the_installed_helper_copies_nothing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let layout = installed(tmp.path());
    let dir = layout.helper().parent().expect("has a dir").to_path_buf();
    let roundabout = dir
        .join("..")
        .join(dir.file_name().expect("named"))
        .join("orcblockd.exe");
    install(&layout, &runner(ELEVATED), &roundabout).expect("reinstalls");
    assert_eq!(fs::read(layout.helper()).expect("kept"), b"binary");
}
