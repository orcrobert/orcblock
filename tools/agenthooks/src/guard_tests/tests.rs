use std::path::PathBuf;

use super::*;
use crate::hook::ToolArgs;
use crate::runner::fake::FakeRunner;

fn allowed(path: &str) -> bool {
    let runner = FakeRunner::new(|line| panic!("guard-tests must not run {line}"));
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
    run(&input, &env) == Outcome::Allow
}

#[test]
fn only_test_files_are_allowed() {
    let cases = [
        (
            "unit tests module",
            "/repo/crates/orcblock-core/src/domain/tests.rs",
            true,
        ),
        (
            "integration test",
            "/repo/crates/orcblockd/tests/socket.rs",
            true,
        ),
        ("fixture", "/repo/crates/orcblockd/testdata/query.bin", true),
        ("fuzz target", "/repo/fuzz/fuzz_targets/dns.rs", true),
        ("ui test", "/repo/app/src/lib/format.test.ts", true),
        (
            "production code",
            "/repo/crates/orcblock-core/src/domain.rs",
            false,
        ),
        (
            "cargo manifest",
            "/repo/crates/orcblock-core/Cargo.toml",
            false,
        ),
        ("outside repo", "/tmp/tests/x.rs", false),
        ("escaping repo", "/repo/../other/tests.rs", false),
    ];
    for (name, path, want) in cases {
        assert_eq!(allowed(path), want, "{name}");
    }
}
