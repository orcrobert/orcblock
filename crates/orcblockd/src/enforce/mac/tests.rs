use std::collections::BTreeSet;
use std::fs;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

use super::*;
use crate::enforce::runner::Output;
use crate::enforce::runner::fake::FakeRunner;

const PORT: u16 = 53535;

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid")
}

/// A fake pf that remembers whether it is enabled and what the anchor holds.
fn fake_pf() -> (FakeRunner, Arc<Mutex<(bool, bool)>>) {
    let pf = Arc::new(Mutex::new((false, false)));
    let state = Arc::clone(&pf);
    let runner = FakeRunner::new(move |line| {
        let mut pf = state.lock().unwrap_or_else(PoisonError::into_inner);
        let out = |stdout: &str, stderr: &str| Output {
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
        };
        Ok(match line {
            "/sbin/pfctl -E" => {
                pf.0 = true;
                out("", "pf enabled\nToken : 42\n")
            }
            "/sbin/pfctl -s info" => out(
                if pf.0 {
                    "Status: Enabled for 0 days"
                } else {
                    "Status: Disabled"
                },
                "",
            ),
            "/sbin/pfctl -a com.apple/orcblock -f -" => {
                pf.1 = true;
                Output::default()
            }
            "/sbin/pfctl -a com.apple/orcblock -s rules" => out(
                if pf.1 {
                    "block drop out quick proto tcp from any to any port = 853\n"
                } else {
                    ""
                },
                "",
            ),
            "/sbin/pfctl -a com.apple/orcblock -F all" => {
                pf.1 = false;
                Output::default()
            }
            _ => Output::default(),
        })
    });
    (runner, pf)
}

fn enforcer(root: &std::path::Path, runner: FakeRunner) -> MacEnforcer<FakeRunner> {
    MacEnforcer::new(Layout::under(root), runner, PORT).with_lookup(|name| match name {
        "youtube.com" => vec![
            "142.250.0.1".parse().expect("ip"),
            "::".parse().expect("ip"),
        ],
        "www.youtube.com" => vec!["2a00:1450::1".parse().expect("ip")],
        _ => Vec::<IpAddr>::new(),
    })
}

fn resolver_files(root: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(Layout::under(root).resolver_dir())
        .map(|it| {
            it.map(|e| e.expect("entry").file_name().into_string().expect("utf8"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn apply_installs_every_layer() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (runner, _) = fake_pf();
    let mut enforcer = enforcer(tmp.path(), runner.clone());
    enforcer
        .apply(&BTreeSet::from([d("youtube.com")]))
        .expect("applies");

    assert_eq!(
        resolver_files(tmp.path()),
        [
            "mask-h2.icloud.com",
            "mask.icloud.com",
            "use-application-dns.net",
            "youtube.com"
        ]
    );
    assert_eq!(
        runner.lines(),
        [
            "/sbin/pfctl -a com.apple/orcblock -f -",
            "/sbin/pfctl -E",
            "/usr/bin/dscacheutil -flushcache",
            "/usr/bin/killall -HUP mDNSResponder",
            "/sbin/pfctl -k 0.0.0.0/0 -k 142.250.0.1",
            "/sbin/pfctl -k ::/0 -k 2a00:1450::1",
        ]
    );
    assert_eq!(
        runner.calls()[0].stdin.as_deref(),
        Some(pf::rules().as_str())
    );
    let token_file = Layout::under(tmp.path()).data_dir().join("pf-token");
    assert_eq!(fs::read_to_string(token_file).expect("token saved"), "42");
}

#[test]
fn reapplying_the_same_set_changes_nothing_but_the_anchor() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (runner, _) = fake_pf();
    let mut enforcer = enforcer(tmp.path(), runner.clone());
    let domains = BTreeSet::from([d("youtube.com")]);
    enforcer.apply(&domains).expect("applies");
    runner.forget();
    enforcer.apply(&domains).expect("applies");
    assert_eq!(runner.lines(), ["/sbin/pfctl -a com.apple/orcblock -f -"]);
}

#[test]
fn verify_repairs_deleted_files_and_disabled_pf() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (runner, pf) = fake_pf();
    let mut enforcer = enforcer(tmp.path(), runner.clone());
    let domains = BTreeSet::from([d("youtube.com")]);
    enforcer.apply(&domains).expect("applies");
    assert!(!enforcer.verify(&domains).expect("verifies"));

    fs::remove_file(Layout::under(tmp.path()).resolver_dir().join("youtube.com")).expect("removes");
    assert!(enforcer.verify(&domains).expect("verifies"));
    assert!(resolver_files(tmp.path()).contains(&"youtube.com".to_owned()));

    pf.lock().unwrap_or_else(PoisonError::into_inner).0 = false;
    runner.forget();
    assert!(enforcer.verify(&domains).expect("verifies"));
    assert!(
        runner.lines().contains(&"/sbin/pfctl -E".to_owned()),
        "re-enabled with a new token"
    );
}

#[test]
fn clear_removes_everything_and_releases_pf() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (runner, pf) = fake_pf();
    let mut enforcer = enforcer(tmp.path(), runner.clone());
    enforcer
        .apply(&BTreeSet::from([d("youtube.com")]))
        .expect("applies");
    runner.forget();

    enforcer.clear().expect("clears");
    assert_eq!(resolver_files(tmp.path()), Vec::<String>::new());
    assert_eq!(
        runner.lines(),
        [
            "/sbin/pfctl -a com.apple/orcblock -F all",
            "/sbin/pfctl -X 42",
            "/usr/bin/dscacheutil -flushcache",
            "/usr/bin/killall -HUP mDNSResponder",
        ]
    );
    assert!(!pf.lock().unwrap_or_else(PoisonError::into_inner).1);
    assert!(
        !Layout::under(tmp.path())
            .data_dir()
            .join("pf-token")
            .exists()
    );
}

#[test]
fn a_restarted_daemon_reuses_the_saved_token() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (runner, _) = fake_pf();
    enforcer(tmp.path(), runner.clone())
        .apply(&BTreeSet::from([d("x.com")]))
        .expect("applies");

    let mut again = enforcer(tmp.path(), runner.clone());
    runner.forget();
    again.clear().expect("clears");
    assert!(runner.lines().contains(&"/sbin/pfctl -X 42".to_owned()));
}

#[test]
fn a_failing_program_is_an_error() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let runner = FakeRunner::new(|line| {
        if line.contains("-f -") {
            Err("syntax error".to_owned())
        } else {
            Ok(Output::default())
        }
    });
    let mut enforcer = enforcer(tmp.path(), runner);
    let err = enforcer
        .apply(&BTreeSet::from([d("x.com")]))
        .expect_err("fails");
    assert_eq!(err.to_string(), "/sbin/pfctl: syntax error");
    assert!(
        resolver_files(tmp.path()).contains(&"x.com".to_owned()),
        "the DNS layer still holds"
    );
}
