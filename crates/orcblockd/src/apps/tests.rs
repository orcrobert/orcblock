use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use super::fake::{FakePlists, FakeTerminator};
use super::*;
use crate::enforce::runner::Output;
use crate::enforce::runner::fake::FakeRunner;
use crate::wake::fake::FakeConsole;

const PS_FIXTURE: &str = include_str!("../../testdata/ps/during-block.txt");
const PS_LINE: &str = "/usr/bin/env LC_CTYPE=UTF-8 /bin/ps -axww -o pid=,uid=,comm=";

fn id(s: &str) -> BundleId {
    BundleId::parse(s).expect("valid bundle id")
}

fn name(s: &str) -> AppName {
    AppName::parse(s).expect("valid app name")
}

fn bundle_ids() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (
            "/System/Library/CoreServices/Finder.app",
            "com.apple.finder",
        ),
        ("/System/Library/CoreServices/Dock.app", "com.apple.dock"),
        ("/Applications/Slack.app", "com.tinyspeck.slackmacgap"),
        (
            "/Users/sam/Desktop/Copy of Calculator.app",
            "com.apple.calculator",
        ),
        ("/Users/sam/w.app/Slack.app", "com.tinyspeck.slackmacgap"),
        ("/Users/sam/O.app", APP_BUNDLE_ID),
        ("/Users/sam/O.app/Slack.app", "com.tinyspeck.slackmacgap"),
        (
            "/System/Library/../../Users/sam/Slack.app",
            "com.tinyspeck.slackmacgap",
        ),
        (
            "/System/Volumes/Preboot/Cryptexes/App/System/Applications/Safari.app",
            "com.apple.Safari",
        ),
        (
            "/Users/sam/Desktop/Slack ü.app",
            "com.tinyspeck.slackmacgap",
        ),
        (
            "/System/Applications/Calculator.app",
            "com.apple.calculator",
        ),
        (
            "/Applications/Visual Studio Code.app",
            "com.microsoft.VSCode",
        ),
    ])
}

/// Answers `ps` with `ps_output` and `plutil` from [`bundle_ids`], by the
/// bundle path [`FakePlists`] hands it.
fn runner(ps_output: &'static str) -> FakeRunner {
    let ids = bundle_ids();
    FakeRunner::by_call(move |call| {
        let line = call.line();
        if line == PS_LINE {
            return Ok(Output {
                stdout: ps_output.to_owned(),
                stderr: String::new(),
            });
        }
        if call.program != PLUTIL {
            return Err(format!("unexpected call: {line}"));
        }
        ids.get(call.stdin.as_deref().unwrap_or_default())
            .map(|id| Output {
                stdout: format!("{id}\n"),
                stderr: String::new(),
            })
            .ok_or_else(|| "Could not extract value".to_owned())
    })
}

/// How often `plutil` was asked about `bundle`.
fn lookups_of(runner: &FakeRunner, bundle: &str) -> usize {
    runner
        .calls()
        .iter()
        .filter(|call| call.program == PLUTIL && call.stdin.as_deref() == Some(bundle))
        .count()
}

fn guard(runner: FakeRunner) -> (AppGuard, FakeTerminator) {
    let terminator = FakeTerminator::default();
    let mut guard = AppGuard::new(runner, terminator.clone()).reading(FakePlists);
    guard.own_pid = 810;
    (guard, terminator)
}

fn blocked(apps: &[(&str, &str)]) -> BTreeMap<BundleId, AppName> {
    apps.iter().map(|(i, n)| (id(i), name(n))).collect()
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

const NOW: Timestamp = Timestamp::UNIX_EPOCH;

#[test]
fn parses_the_process_table() {
    let processes = parse_ps(PS_FIXTURE);
    assert_eq!(
        processes.len(),
        19,
        "the garbage and empty lines are skipped"
    );
    assert_eq!(
        processes[0],
        Process {
            pid: 1,
            uid: 0,
            path: "/sbin/launchd".into()
        }
    );
    assert_eq!(
        processes[5].path,
        "/Applications/Slack.app/Contents/Frameworks/Slack Helper (Renderer).app/Contents/MacOS/Slack Helper (Renderer)"
    );
    assert_eq!(processes[10].uid, 502);
}

#[test]
fn closes_every_process_of_a_blocked_app_and_nothing_else() {
    let (mut guard, terminator) = guard(runner(PS_FIXTURE));
    let apps = blocked(&[
        ("com.tinyspeck.slackmacgap", "Slack"),
        ("com.apple.calculator", "Calculator"),
    ]);
    guard.sweep(&apps, secs(100), NOW, false);
    let mut sent = terminator.take();
    sent.sort_unstable_by_key(|(pid, _)| *pid);
    assert_eq!(
        sent,
        [
            (612, Stop::Term),
            (618, Stop::Term),
            (640, Stop::Term),
            (655, Stop::Term),
            (660, Stop::Term),
            (670, Stop::Term),
            (702, Stop::Term),
        ]
    );
}

#[test]
fn never_touches_system_processes() {
    let ps = "  320     0 /Applications/Slack.app/Contents/MacOS/Slack\n  321   200 /Applications/Slack.app/Contents/MacOS/Slack\n  401   501 /System/Library/CoreServices/Finder.app/Contents/MacOS/Finder\n  402   501 /Library/Apple/System/Library/CoreServices/XProtect.app/Contents/MacOS/XProtect\n";
    let (mut guard, terminator) = guard(runner(ps));
    let apps = blocked(&[
        ("com.tinyspeck.slackmacgap", "Slack"),
        ("com.apple.finder", "Finder"),
        ("com.apple.xprotectframework.xprotect", "XProtect"),
    ]);
    guard.sweep(&apps, secs(1), NOW, true);
    assert_eq!(terminator.take(), []);
    assert_eq!(guard.launches(), []);
}

#[test]
fn never_signals_launchd_or_itself() {
    // A user's uid, so only the pid rules keep these alive.
    let ps = "    1   501 /Applications/Slack.app/Contents/MacOS/Slack\n    0   501 /Applications/Slack.app/Contents/MacOS/Slack\n  810   501 /Applications/Slack.app/Contents/MacOS/Slack\n";
    let (mut guard, terminator) = guard(runner(ps));
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        true,
    );
    assert_eq!(terminator.take(), []);
    assert_eq!(guard.launches(), []);
}

#[test]
fn kills_what_ignores_sigterm_once() {
    let (mut guard, terminator) = guard(runner(PS_FIXTURE));
    let apps = blocked(&[("com.microsoft.vscode", "Code")]);
    guard.sweep(&apps, secs(10), NOW, false);
    assert_eq!(terminator.take(), [(999, Stop::Term)]);
    guard.sweep(&apps, secs(12), NOW, false);
    assert_eq!(terminator.take(), [], "still within the grace period");
    guard.sweep(&apps, secs(13), NOW, false);
    assert_eq!(terminator.take(), [(999, Stop::Kill)]);
    guard.sweep(&apps, secs(20), NOW, false);
    assert_eq!(terminator.take(), [], "killed once");
}

#[test]
fn a_reappearing_process_is_closed_again() {
    let (mut guard, terminator) = guard(runner(PS_FIXTURE));
    let apps = blocked(&[("com.microsoft.vscode", "Code")]);
    guard.sweep(&apps, secs(10), NOW, false);
    terminator.take();

    let (mut empty, _) = self::guard(runner(""));
    std::mem::swap(&mut guard.runner, &mut empty.runner);
    guard.sweep(&apps, secs(11), NOW, false);
    std::mem::swap(&mut guard.runner, &mut empty.runner);

    guard.sweep(&apps, secs(12), NOW, false);
    assert_eq!(terminator.take(), [(999, Stop::Term)], "a fresh launch");
}

#[test]
fn records_one_launch_per_app_and_only_when_asked() {
    let (mut guard, _) = guard(runner(PS_FIXTURE));
    let apps = blocked(&[
        ("com.tinyspeck.slackmacgap", "Slack"),
        ("com.apple.calculator", "Calculator"),
    ]);
    guard.sweep(&apps, secs(1), NOW, false);
    assert_eq!(guard.launches(), [], "already open at block start");

    let (mut guard, _) = self::guard(runner(PS_FIXTURE));
    guard.sweep(&apps, secs(1), NOW, true);
    let launches = guard.launches();
    let names: Vec<&str> = launches.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Slack", "Calculator"]);
    let ids: Vec<u64> = launches.iter().map(|l| l.id).collect();
    assert_eq!(ids, [1, 2]);
}

#[test]
fn a_quick_relaunch_is_the_same_launch() {
    let (mut guard, _) = guard(runner(
        "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n",
    ));
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    guard.sweep(&apps, secs(10), NOW, true);
    guard.pending.clear();
    guard.sweep(&apps, secs(12), NOW, true);
    assert_eq!(guard.launches().len(), 1, "within {SAME_LAUNCH:?}");
    guard.pending.clear();
    guard.sweep(&apps, secs(20), NOW, true);
    assert_eq!(guard.launches().len(), 2);
}

#[test]
fn keeps_the_last_launches_only() {
    let (mut guard, _) = guard(runner(
        "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n",
    ));
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    for n in 0..20 {
        guard.pending.clear();
        guard.sweep(&apps, secs(n * 10), NOW, true);
    }
    let launches = guard.launches();
    assert_eq!(launches.len(), LAUNCHES_KEPT);
    assert_eq!(launches.first().map(|l| l.id), Some(13));
    assert_eq!(launches.last().map(|l| l.id), Some(20));
}

#[test]
fn caches_bundle_ids_and_retries_failures_later() {
    let ps = "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n  700   501 /Applications/Broken.app/Contents/MacOS/Broken\n";
    let runner = runner(ps);
    let (mut guard, _) = guard(runner.clone());
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    guard.sweep(&apps, secs(1), NOW, false);
    guard.sweep(&apps, secs(2), NOW, false);
    let slack = "/Applications/Slack.app";
    let broken = "/Applications/Broken.app";
    assert_eq!(lookups_of(&runner, slack), 1);
    assert_eq!(lookups_of(&runner, broken), 1);
    guard.sweep(&apps, secs(31), NOW, false);
    assert_eq!(lookups_of(&runner, broken), 2, "a failure is retried");

    guard.idle();
    guard.sweep(&apps, secs(32), NOW, false);
    assert_eq!(lookups_of(&runner, slack), 2, "idle forgets the cache");
}

#[test]
fn a_failed_listing_signals_nothing() {
    let failing = FakeRunner::new(|_| Err("ps: out of memory".to_owned()));
    let (mut guard, terminator) = guard(failing);
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        true,
    );
    assert_eq!(terminator.take(), []);
}

#[test]
fn own_uid_terminator_leaves_other_users_alone() {
    // Only the foreign-uid branch is exercised: the other one sends a real
    // signal, which tests never do.
    let foreign = OwnUidTerminator::new(501);
    assert!(foreign.stop(i32::MAX, 502, Stop::Kill).is_ok());
}

/// A guard whose waker records its `launchctl` calls in the returned runner.
fn waking_guard(ps_output: &'static str) -> (AppGuard, FakeRunner) {
    let launches = FakeRunner::new(|_| {
        Ok(Output {
            stdout: String::new(),
            stderr: String::new(),
        })
    });
    let console = FakeConsole::default();
    console.set(Some(501));
    let (guard, _) = guard(runner(ps_output));
    (
        guard.waking(Waker::new(launches.clone(), console)),
        launches,
    )
}

#[test]
fn wakes_the_app_for_a_new_launch_only() {
    let slack = "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n";
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);

    let (mut guard, launches) = waking_guard(slack);
    guard.sweep(&apps, secs(1), NOW, false);
    assert_eq!(launches.lines().len(), 0, "open at block start: no shield");
    guard.pending.clear();
    guard.sweep(&apps, secs(10), NOW, true);
    assert_eq!(launches.lines().len(), 1);
}

#[test]
fn leaves_an_open_app_to_show_the_shield_itself() {
    let ps = "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n  400   501 /Applications/OrcBlock.app/Contents/MacOS/orcblock-app\n";
    let ids = bundle_ids();
    let answers = FakeRunner::by_call(move |call| {
        if call.line() == PS_LINE {
            return Ok(Output {
                stdout: ps.to_owned(),
                stderr: String::new(),
            });
        }
        let id = if call.stdin.as_deref() == Some("/Applications/OrcBlock.app") {
            APP_BUNDLE_ID
        } else {
            ids["/Applications/Slack.app"]
        };
        Ok(Output {
            stdout: format!("{id}\n"),
            stderr: String::new(),
        })
    });
    let launches = FakeRunner::new(|_| Err("must not run".to_owned()));
    let console = FakeConsole::default();
    console.set(Some(501));
    let (guard, terminator) = guard(answers);
    let mut guard = guard.waking(Waker::new(launches.clone(), console));
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        true,
    );
    assert_eq!(launches.lines(), Vec::<String>::new());
    assert_eq!(
        terminator.take(),
        [(612, Stop::Term)],
        "the app itself is left alone"
    );
    assert_eq!(guard.launches().len(), 1);
}

#[test]
fn looks_up_a_bounded_number_of_new_bundles_per_sweep() {
    let flood = (0..LOOKUPS_PER_SWEEP + 40).fold(String::new(), |mut table, i| {
        let _ = writeln!(table, "  {}   501 /tmp/fake{i}.app/x", 2000 + i);
        table
    });
    let flood: &'static str = Box::leak(flood.into_boxed_str());
    let runner = runner(flood);
    let (mut guard, _) = guard(runner.clone());
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    let lookups = || {
        runner
            .lines()
            .iter()
            .filter(|line| line.starts_with(PLUTIL))
            .count()
    };
    guard.sweep(&apps, secs(1), NOW, false);
    assert_eq!(lookups(), LOOKUPS_PER_SWEEP);
    guard.sweep(&apps, secs(2), NOW, false);
    assert_eq!(lookups(), LOOKUPS_PER_SWEEP + 40, "the rest next sweep");
    guard.sweep(&apps, secs(3), NOW, false);
    assert_eq!(lookups(), LOOKUPS_PER_SWEEP + 40, "failures wait");
}

#[test]
fn an_invalid_id_is_never_looked_up_again() {
    let ps = "  700   501 /Applications/Unity Game.app/Contents/MacOS/Game\n";
    let runner = FakeRunner::new(move |line| {
        let stdout = if line == PS_LINE {
            ps.to_owned()
        } else {
            "com.unity.my_game\n".to_owned()
        };
        Ok(Output {
            stdout,
            stderr: String::new(),
        })
    });
    let (mut guard, _) = guard(runner.clone());
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    for n in [1, 40, 80] {
        guard.sweep(&apps, secs(n), NOW, false);
    }
    let lookups = runner
        .lines()
        .iter()
        .filter(|line| line.starts_with(PLUTIL))
        .count();
    assert_eq!(lookups, 1);
}

#[test]
fn a_bundle_replaced_at_the_same_path_is_looked_up_again() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("Foo.app");
    let plist = bundle.join("Contents/Info.plist");
    fs::create_dir_all(bundle.join("Contents")).expect("creates bundle");
    fs::write(&plist, "foo").expect("writes plist");
    let ps = format!("  700   501 {}/Contents/MacOS/Foo\n", bundle.display());
    let answer = Arc::new(Mutex::new("com.example.foo\n"));
    let runner = {
        let answer = Arc::clone(&answer);
        FakeRunner::new(move |line| {
            let stdout = if line == PS_LINE {
                ps.clone()
            } else {
                (*answer.lock().unwrap_or_else(PoisonError::into_inner)).to_owned()
            };
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        })
    };
    let (guard, terminator) = guard(runner);
    let mut guard = guard.reading(SystemPlists);
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    guard.sweep(&apps, secs(1), NOW, false);
    assert_eq!(terminator.take(), []);

    // Slack copied over Foo.app: a new Info.plist file at the same path.
    fs::remove_file(&plist).expect("removes plist");
    fs::write(&plist, "slack, a little longer").expect("writes plist");
    *answer.lock().unwrap_or_else(PoisonError::into_inner) = "com.tinyspeck.slackmacgap\n";
    guard.sweep(&apps, secs(2), NOW, false);
    assert_eq!(terminator.take(), [], "a changed file waits its turn");
    guard.sweep(&apps, secs(31), NOW, false);
    assert_eq!(terminator.take(), [(700, Stop::Term)]);
}

#[test]
fn a_wrapper_with_orcblocks_id_hides_nothing() {
    let ps = "  680   501 /Users/sam/O.app/Slack.app/Contents/MacOS/Slack\n";
    let (guard, launches) = waking_guard(ps);
    let mut guard = guard;
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        true,
    );
    assert_eq!(guard.launches().len(), 1);
    assert_eq!(
        launches.lines().len(),
        1,
        "the app is not open, so it is woken"
    );
}

#[test]
fn a_path_climbing_out_of_the_system_is_not_the_system() {
    let ps = "  690   501 /System/Library/../../Users/sam/Slack.app/Contents/MacOS/Slack\n";
    let (mut guard, terminator) = guard(runner(ps));
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        false,
    );
    assert_eq!(terminator.take(), [(690, Stop::Term)]);
}

#[test]
fn a_flood_of_bundles_only_delays_a_blocked_app() {
    let decoys = 2 * LOOKUPS_PER_SWEEP + 10;
    let mut table = (0..decoys).fold(String::new(), |mut table, i| {
        let _ = writeln!(table, "  {}   501 /Applications/A{i:03}.app/x", 2000 + i);
        table
    });
    table.push_str("  9000   501 /Applications/Slack.app/Contents/MacOS/Slack\n");
    let table: &'static str = Box::leak(table.into_boxed_str());
    let (mut guard, terminator) = guard(runner(table));
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    guard.sweep(&apps, secs(1), NOW, false);
    guard.sweep(&apps, secs(2), NOW, false);
    assert_eq!(terminator.take(), [], "Slack comes after every decoy");
    guard.sweep(&apps, secs(3), NOW, false);
    assert_eq!(terminator.take(), [(9000, Stop::Term)]);

    // The decoys keep failing and come due again; Slack's ID stays known.
    guard.pending.clear();
    guard.sweep(&apps, secs(31), NOW, false);
    assert_eq!(terminator.take(), [(9000, Stop::Term)]);
    guard.pending.clear();
    guard.sweep(&apps, secs(32), NOW, false);
    assert_eq!(terminator.take(), [(9000, Stop::Term)]);
}

#[test]
fn an_info_plist_that_is_not_a_small_file_is_never_read() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("Odd.app");
    fs::create_dir_all(bundle.join("Contents/Info.plist")).expect("creates a folder");
    let ps = format!("  700   501 {}/Contents/MacOS/Odd\n", bundle.display());
    let runner = FakeRunner::new(move |line| {
        if line == PS_LINE {
            Ok(Output {
                stdout: ps.clone(),
                stderr: String::new(),
            })
        } else {
            Err(format!("unexpected call: {line}"))
        }
    });
    let (guard, _) = guard(runner.clone());
    let mut guard = guard.reading(SystemPlists);
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        false,
    );
    assert!(runner.lines().iter().all(|line| line == PS_LINE));
}

#[test]
fn reads_only_small_regular_plists_following_links() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = |name: &str| dir.path().join(name).display().to_string();
    fs::write(path("plain"), "<plist/>").expect("writes plist");
    nix::unistd::mkfifo(
        dir.path().join("fifo").as_path(),
        nix::sys::stat::Mode::S_IRWXU,
    )
    .expect("makes a fifo");
    std::os::unix::fs::symlink(path("plain"), path("link")).expect("makes a link");
    fs::write(path("huge"), vec![b'x'; 2 * 1024 * 1024]).expect("writes a big file");

    for readable in ["plain", "link"] {
        let (bytes, stamp) = SystemPlists.read(&path(readable)).expect(readable);
        assert_eq!(bytes, b"<plist/>", "{readable}");
        assert_eq!(stamp, SystemPlists.stamp(&path("plain")), "{readable}");
    }
    for refused in ["fifo", "huge", "missing"] {
        assert!(SystemPlists.read(&path(refused)).is_err(), "{refused}");
    }
}

#[test]
fn closes_safari_in_the_app_cryptex() {
    let ps = "  720   501 /System/Volumes/Preboot/Cryptexes/App/System/Applications/Safari.app/Contents/MacOS/Safari\n";
    let (mut guard, terminator) = guard(runner(ps));
    guard.sweep(
        &blocked(&[("com.apple.Safari", "Safari")]),
        secs(1),
        NOW,
        false,
    );
    assert_eq!(terminator.take(), [(720, Stop::Term)]);
}

#[test]
fn ignores_made_up_paths_and_deep_nesting() {
    let long = format!("  730   501 /{}Slack.app/x\n", "a/".repeat(600));
    let deep = format!(
        "  740   501 /Users/sam/{}x\n",
        (0..20).fold(String::new(), |path, i| format!("{path}n{i}.app/"))
    );
    let table: &'static str = Box::leak(format!("{long}{deep}").into_boxed_str());
    let runner = runner(table);
    let (mut guard, _) = guard(runner.clone());
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        false,
    );
    let mut looked_up: Vec<String> = runner
        .calls()
        .into_iter()
        .filter(|call| call.program == PLUTIL)
        .filter_map(|call| call.stdin)
        .collect();
    looked_up.sort();
    assert_eq!(
        looked_up,
        [
            "/Users/sam/n0.app",
            "/Users/sam/n0.app/n1.app/n2.app/n3.app/n4.app/n5.app/n6.app/n7.app/n8.app/n9.app/n10.app/n11.app/n12.app/n13.app/n14.app/n15.app/n16.app/n17.app",
            "/Users/sam/n0.app/n1.app/n2.app/n3.app/n4.app/n5.app/n6.app/n7.app/n8.app/n9.app/n10.app/n11.app/n12.app/n13.app/n14.app/n15.app/n16.app/n17.app/n18.app",
            "/Users/sam/n0.app/n1.app/n2.app/n3.app/n4.app/n5.app/n6.app/n7.app/n8.app/n9.app/n10.app/n11.app/n12.app/n13.app/n14.app/n15.app/n16.app/n17.app/n18.app/n19.app",
        ],
        "the outermost and innermost three only; the long path not at all"
    );
}

#[test]
fn a_relaunched_blocked_app_is_closed_at_once() {
    let slack = "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n";
    let state = Arc::new(Mutex::new(slack));
    let runner = {
        let state = Arc::clone(&state);
        let ids = bundle_ids();
        FakeRunner::by_call(move |call| {
            let stdout = if call.line() == PS_LINE {
                (*state.lock().unwrap_or_else(PoisonError::into_inner)).to_owned()
            } else {
                format!("{}\n", ids[call.stdin.as_deref().unwrap_or_default()])
            };
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        })
    };
    let (mut guard, terminator) = guard(runner.clone());
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    guard.sweep(&apps, secs(1), NOW, false);
    *state.lock().unwrap_or_else(PoisonError::into_inner) = "";
    guard.sweep(&apps, secs(2), NOW, false);
    *state.lock().unwrap_or_else(PoisonError::into_inner) =
        "  613   501 /Applications/Slack.app/Contents/MacOS/Slack\n";
    guard.sweep(&apps, secs(300), NOW, false);
    assert_eq!(terminator.take(), [(612, Stop::Term), (613, Stop::Term)]);
    assert_eq!(lookups_of(&runner, "/Applications/Slack.app"), 1);
}

#[test]
fn signals_only_what_a_fresh_listing_still_shows() {
    let listings = Arc::new(Mutex::new(0));
    let runner = {
        let listings = Arc::clone(&listings);
        FakeRunner::by_call(move |call| {
            let stdout = if call.line() == PS_LINE {
                let mut n = listings.lock().unwrap_or_else(PoisonError::into_inner);
                *n += 1;
                if *n == 1 {
                    "  612   501 /Applications/Slack.app/Contents/MacOS/Slack\n".to_owned()
                } else {
                    String::new()
                }
            } else {
                "com.tinyspeck.slackmacgap\n".to_owned()
            };
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        })
    };
    let (mut guard, terminator) = guard(runner);
    guard.sweep(
        &blocked(&[("com.tinyspeck.slackmacgap", "Slack")]),
        secs(1),
        NOW,
        false,
    );
    assert_eq!(terminator.take(), [], "612 quit while its bundle was read");
}

#[test]
fn keeps_a_bounded_number_of_gone_blocked_apps() {
    let state = Arc::new(Mutex::new(String::new()));
    let runner = {
        let state = Arc::clone(&state);
        FakeRunner::by_call(move |call| {
            let stdout = if call.line() == PS_LINE {
                state.lock().unwrap_or_else(PoisonError::into_inner).clone()
            } else {
                "com.tinyspeck.slackmacgap\n".to_owned()
            };
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        })
    };
    let (mut guard, _) = guard(runner);
    let apps = blocked(&[("com.tinyspeck.slackmacgap", "Slack")]);
    for round in 0..3_u64 {
        let copies = (0..LOOKUPS_PER_SWEEP).fold(String::new(), |mut table, i| {
            let _ = writeln!(table, "  {}   501 /Users/sam/r{round}c{i}.app/x", 3000 + i);
            table
        });
        *state.lock().unwrap_or_else(PoisonError::into_inner) = copies;
        guard.sweep(&apps, secs(1 + round), NOW, false);
    }
    state.lock().unwrap_or_else(PoisonError::into_inner).clear();
    guard.sweep(&apps, secs(10), NOW, false);
    assert_eq!(guard.ids.len(), MAX_GONE);
    assert!(
        guard.ids.keys().all(|bundle| !bundle.contains("/r0c")),
        "the oldest go first"
    );
}
