use std::path::PathBuf;

use super::*;
use crate::hook::ToolArgs;
use crate::runner::fake::FakeRunner;

fn decide(path: &str) -> Outcome {
    let runner = FakeRunner::new(|line| panic!("guard-files must not run {line}"));
    let env = Env {
        project_dir: PathBuf::from("/repo"),
        runner: &runner,
    };
    let input = HookInput {
        tool_input: ToolArgs {
            file_path: path.to_owned(),
            ..ToolArgs::default()
        },
        ..HookInput::default()
    };
    run(&input, &env)
}

#[test]
fn decisions() {
    let cases = [
        ("rust file", "/repo/crates/orcblock-core/src/lib.rs", None),
        ("dotenv", "/repo/.env", Some(".env.example")),
        (
            "dotenv variant",
            "/repo/.env.production",
            Some(".env.example"),
        ),
        ("env example", "/repo/.env.example", None),
        ("cargo lock", "/repo/Cargo.lock", Some("cargo build")),
        (
            "npm lock",
            "/repo/app/package-lock.json",
            Some("npm install"),
        ),
        ("cargo toml", "/repo/Cargo.toml", None),
        (
            "resolver file",
            "/etc/resolver/youtube.com",
            Some("system path"),
        ),
        (
            "launch daemon",
            "/Library/LaunchDaemons/com.orcrobert.orcblockd.plist",
            Some("system path"),
        ),
        (
            "escaping project dir",
            "/repo/../etc/hosts",
            Some("system path"),
        ),
        ("user library is fine", "/Users/me/Library/x", None),
        ("empty path", "", None),
    ];
    for (name, path, hint) in cases {
        match (decide(path), hint) {
            (Outcome::Allow, None) => {}
            (Outcome::Block(reason), Some(hint)) => {
                assert!(
                    reason.contains(hint),
                    "{name}: {reason:?} should mention {hint:?}"
                );
            }
            (outcome, hint) => panic!("{name}: got {outcome:?}, want hint {hint:?}"),
        }
    }
}
