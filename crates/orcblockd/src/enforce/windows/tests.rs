use std::sync::{Arc, Mutex, PoisonError};

use super::*;
use crate::enforce::runner::Output;
use crate::enforce::runner::fake::FakeRunner;

const GUID_A: &str = "{0E5B1F42-6C3C-4E59-9D43-7A1B2C3D4E5F}";
const GUID_B: &str = "{1E5B1F42-6C3C-4E59-9D43-7A1B2C3D4E5F}";
const GUID_C: &str = "{2E5B1F42-6C3C-4E59-9D43-7A1B2C3D4E5F}";

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid")
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|&s| s.to_owned()).collect()
}

fn nrpt(name: &str, domain: &str) -> NrptRule {
    NrptRule {
        name: name.to_owned(),
        namespaces: names(&[&format!(".{domain}"), domain]),
        servers: SINKHOLE.to_owned(),
    }
}

fn firewall() -> Vec<FirewallRule> {
    FIREWALL_RULES
        .iter()
        .map(|&name| FirewallRule {
            name: name.to_owned(),
            blocks: true,
        })
        .collect()
}

fn decode(encoded: &str) -> String {
    let bytes = BASE64.decode(encoded).expect("base64");
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect();
    String::from_utf16(&units).expect("utf-16")
}

#[test]
fn parses_the_status_script_output() {
    let stdout = format!(
        "nrpt\t{GUID_A}\t.YouTube.com,youtube.com\t127.0.0.1\r\n\
         fw\torcblockd: DNS over TLS (TCP)\tTrue\tBlock\tOutbound\r\n\
         fw\torcblockd: DNS over TLS (UDP)\tFalse\tBlock\tOutbound\r\n\
         WARNING: something chatty\r\n"
    );
    assert_eq!(
        parse_status(&stdout),
        Observed {
            nrpt: vec![nrpt(GUID_A, "youtube.com")],
            firewall: vec![
                FirewallRule {
                    name: "orcblockd: DNS over TLS (TCP)".to_owned(),
                    blocks: true,
                },
                FirewallRule {
                    name: "orcblockd: DNS over TLS (UDP)".to_owned(),
                    blocks: false,
                },
            ],
        }
    );
}

struct Case {
    name: &'static str,
    observed: Observed,
    wanted: &'static [&'static str],
    firewall: bool,
    want: Vec<Op>,
}

fn check(cases: Vec<Case>) {
    for case in cases {
        assert_eq!(
            plan(&case.observed, &names(case.wanted), case.firewall),
            case.want,
            "{}",
            case.name
        );
    }
}

fn firewall_ops() -> Vec<Op> {
    let mut ops = vec![Op::ClearFirewall];
    ops.extend(
        doh::RESOLVERS
            .iter()
            .map(|ip| Op::FirewallIp(ip.parse().expect("ip"))),
    );
    ops.push(Op::AddFirewall);
    ops
}

fn in_place() -> Observed {
    Observed {
        nrpt: vec![nrpt(GUID_A, "youtube.com")],
        firewall: firewall(),
    }
}

#[test]
fn plans_nrpt_rules() {
    check(vec![
        Case {
            name: "in place: nothing to do",
            observed: in_place(),
            wanted: &["youtube.com"],
            firewall: true,
            want: vec![],
        },
        Case {
            name: "from nothing: add rules, firewall, flush",
            observed: Observed::default(),
            wanted: &["youtube.com"],
            firewall: true,
            want: [
                vec![Op::AddNrpt("youtube.com".to_owned())],
                firewall_ops(),
                vec![Op::Flush],
            ]
            .concat(),
        },
        Case {
            name: "an unwanted and a duplicate rule go",
            observed: Observed {
                nrpt: vec![
                    nrpt(GUID_A, "youtube.com"),
                    nrpt(GUID_B, "youtube.com"),
                    nrpt(GUID_C, "reddit.com"),
                ],
                firewall: firewall(),
            },
            wanted: &["youtube.com"],
            firewall: true,
            want: vec![
                Op::RemoveNrpt(GUID_B.to_owned()),
                Op::RemoveNrpt(GUID_C.to_owned()),
                Op::Flush,
            ],
        },
        Case {
            name: "a rule pointing elsewhere is replaced",
            observed: Observed {
                nrpt: vec![NrptRule {
                    servers: "1.1.1.1".to_owned(),
                    ..nrpt(GUID_A, "youtube.com")
                }],
                firewall: firewall(),
            },
            wanted: &["youtube.com"],
            firewall: true,
            want: vec![
                Op::RemoveNrpt(GUID_A.to_owned()),
                Op::AddNrpt("youtube.com".to_owned()),
                Op::Flush,
            ],
        },
        Case {
            name: "a rule missing its subdomain namespace is replaced",
            observed: Observed {
                nrpt: vec![NrptRule {
                    namespaces: names(&["youtube.com"]),
                    ..nrpt(GUID_A, "youtube.com")
                }],
                firewall: firewall(),
            },
            wanted: &["youtube.com"],
            firewall: true,
            want: vec![
                Op::RemoveNrpt(GUID_A.to_owned()),
                Op::AddNrpt("youtube.com".to_owned()),
                Op::Flush,
            ],
        },
    ]);
}

#[test]
fn plans_firewall_rules_and_clearing() {
    check(vec![
        Case {
            name: "a disabled firewall rule rebuilds the firewall only",
            observed: Observed {
                firewall: {
                    let mut rules = firewall();
                    rules[2].blocks = false;
                    rules
                },
                ..in_place()
            },
            wanted: &["youtube.com"],
            firewall: true,
            want: firewall_ops(),
        },
        Case {
            name: "a missing firewall rule rebuilds the firewall",
            observed: Observed {
                firewall: firewall()[..3].to_vec(),
                ..in_place()
            },
            wanted: &["youtube.com"],
            firewall: true,
            want: firewall_ops(),
        },
        Case {
            name: "clear removes everything",
            observed: in_place(),
            wanted: &[],
            firewall: false,
            want: vec![
                Op::RemoveNrpt(GUID_A.to_owned()),
                Op::ClearFirewall,
                Op::Flush,
            ],
        },
        Case {
            name: "clear with nothing in place does nothing",
            observed: Observed::default(),
            wanted: &[],
            firewall: false,
            want: vec![],
        },
    ]);
}

#[test]
fn ops_render_as_script_lines() {
    let cases = [
        (
            Op::RemoveNrpt(GUID_A.to_owned()),
            format!("nrpt-remove {GUID_A}"),
        ),
        (
            Op::AddNrpt("youtube.com".to_owned()),
            "nrpt-add youtube.com".to_owned(),
        ),
        (Op::ClearFirewall, "fw-clear".to_owned()),
        (
            Op::FirewallIp("2620:fe::fe".parse().expect("ip")),
            "fw-ip 2620:fe::fe".to_owned(),
        ),
        (Op::AddFirewall, "fw-add".to_owned()),
        (Op::Flush, "flush".to_owned()),
    ];
    for (op, line) in cases {
        assert_eq!(op.to_string(), line);
    }
}

#[test]
fn scripts_only_touch_marked_rules() {
    assert!(STATUS_SCRIPT.contains("-eq 'managed by orcblockd'"));
    assert!(APPLY_SCRIPT.contains("-Comment 'managed by orcblockd'"));
    assert!(APPLY_SCRIPT.contains(&format!("-NameServers '{SINKHOLE}'")));
    assert!(APPLY_SCRIPT.contains("foreach ($protocol in 'TCP', 'UDP')"));
    for rule in FIREWALL_RULES {
        let template = rule
            .replace("(TCP)", "($protocol)")
            .replace("(UDP)", "($protocol)");
        assert!(
            APPLY_SCRIPT.contains(&format!("-DisplayName \"{template}\"")),
            "{rule}"
        );
    }
}

#[test]
fn encodes_commands_as_utf16_base64() {
    assert_eq!(encode_command("dir"), "ZABpAHIA");
    assert_eq!(decode(&encode_command(APPLY_SCRIPT)), APPLY_SCRIPT);
}

/// A fake PowerShell holding NRPT and firewall rules, answering the two
/// scripts the way Windows would.
fn fake_windows() -> (FakeRunner, Arc<Mutex<Observed>>) {
    let system = Arc::new(Mutex::new(Observed::default()));
    let state = Arc::clone(&system);
    let status = encode_command(STATUS_SCRIPT);
    let apply = encode_command(APPLY_SCRIPT);
    let runner = FakeRunner::new(move |line| {
        assert!(line.starts_with(POWERSHELL), "{line}");
        let system = state.lock().unwrap_or_else(PoisonError::into_inner);
        if line.ends_with(&status) {
            let mut stdout = String::new();
            for rule in &system.nrpt {
                let namespaces: Vec<&str> = rule.namespaces.iter().map(String::as_str).collect();
                let _ = write!(
                    stdout,
                    "nrpt\t{}\t{}\t{}\r\n",
                    rule.name,
                    namespaces.join(","),
                    rule.servers
                );
            }
            for rule in &system.firewall {
                let _ = write!(stdout, "fw\t{}\tTrue\tBlock\tOutbound\r\n", rule.name);
            }
            Ok(Output {
                stdout,
                stderr: String::new(),
            })
        } else if line.ends_with(&apply) {
            Ok(Output::default())
        } else {
            Err(format!("unexpected: {line}"))
        }
    });
    (runner, system)
}

/// Applies the ops a recorded apply call carried to the fake system.
fn replay(system: &Arc<Mutex<Observed>>, stdin: &str) {
    let mut system = system.lock().unwrap_or_else(PoisonError::into_inner);
    let mut next = 0;
    for line in stdin.lines() {
        let (op, arg) = line.split_once(' ').unwrap_or((line, ""));
        match op {
            "nrpt-remove" => system.nrpt.retain(|rule| rule.name != arg),
            "nrpt-add" => {
                next += 1;
                system.nrpt.push(nrpt(
                    &format!("{{{next:08}-0000-0000-0000-000000000000}}"),
                    arg,
                ));
            }
            "fw-clear" => system.firewall.clear(),
            "fw-add" => system.firewall = firewall(),
            "fw-ip" | "flush" => {}
            other => panic!("unknown op {other}"),
        }
    }
}

#[test]
fn apply_verify_and_clear_round_trip() {
    let (runner, system) = fake_windows();
    let mut enforcer = WinEnforcer::new(runner.clone());
    let domains: BTreeSet<Domain> = [d("youtube.com")].into();

    enforcer.apply(&domains).expect("applies");
    let calls = runner.calls();
    assert_eq!(calls.len(), 2, "status, then apply");
    let stdin = calls[1].stdin.clone().expect("ops on stdin");
    assert!(stdin.contains("nrpt-add youtube.com\n"), "{stdin}");
    for canary in CANARIES {
        assert!(stdin.contains(&format!("nrpt-add {canary}\n")), "{stdin}");
    }
    assert!(stdin.ends_with("flush\n"), "{stdin}");
    assert!(calls[1].args.contains(&"-NonInteractive".to_owned()));
    replay(&system, &stdin);

    runner.forget();
    assert!(!enforcer.verify(&domains).expect("verifies"), "in place");
    assert_eq!(runner.calls().len(), 1, "only the status script");

    system
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .nrpt
        .remove(0);
    runner.forget();
    assert!(
        enforcer.verify(&domains).expect("verifies"),
        "drift repaired"
    );
    replay(&system, &runner.calls()[1].stdin.clone().expect("ops"));

    runner.forget();
    enforcer.clear().expect("clears");
    replay(&system, &runner.calls()[1].stdin.clone().expect("ops"));
    assert_eq!(
        *system.lock().unwrap_or_else(PoisonError::into_inner),
        Observed::default()
    );
}

#[test]
fn a_failing_script_is_an_error() {
    let runner = FakeRunner::new(|_| Err("Access is denied".to_owned()));
    let err = WinEnforcer::new(runner)
        .apply(&[d("youtube.com")].into())
        .expect_err("fails");
    assert!(err.to_string().contains("Access is denied"), "{err}");
}

#[test]
fn apply_script_carries_on_after_a_failed_operation() {
    let body = APPLY_SCRIPT
        .split_once("foreach ($line in @($input))")
        .expect("loops over stdin")
        .1;
    let (inside, after) = body.split_once("} catch {").expect("catches per operation");
    assert!(inside.contains("switch -CaseSensitive ($op)"));
    assert!(after.contains("$failed +="));
    assert!(after.contains("exit 1"), "failures still fail the run");
    for script in [STATUS_SCRIPT, APPLY_SCRIPT] {
        assert!(script.contains("$ProgressPreference = 'SilentlyContinue'"));
    }
}
