use std::path::PathBuf;

use super::*;
use crate::hook::ToolArgs;
use crate::runner::RunError;
use crate::runner::fake::FakeRunner;

fn format(path: &str, fail: bool) -> Vec<(PathBuf, String)> {
    let runner = FakeRunner::new(move |_| {
        if fail {
            Err(RunError::Failed("syntax error".into()))
        } else {
            Ok(String::new())
        }
    });
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
    assert_eq!(run(&input, &env), Outcome::Allow, "format never blocks");
    runner.calls.into_inner()
}

#[test]
fn rust_files_go_through_rustfmt() {
    let calls = format("/repo/crates/x/src/lib.rs", false);
    assert_eq!(
        calls,
        vec![(
            PathBuf::from("/repo"),
            "rustfmt --edition 2024 /repo/crates/x/src/lib.rs".into()
        )]
    );
}

#[test]
fn ui_files_go_through_prettier_in_app() {
    let calls = format("/repo/app/src/lib/Site.svelte", false);
    assert_eq!(
        calls,
        vec![(
            PathBuf::from("/repo/app"),
            "npx --no-install prettier --write /repo/app/src/lib/Site.svelte".into()
        )]
    );
}

#[test]
fn other_files_are_left_alone() {
    assert_eq!(format("/repo/README.md", false), vec![]);
    assert_eq!(format("/repo/docs/x.json", false), vec![]);
}

#[test]
fn formatter_failure_still_allows() {
    assert_eq!(format("/repo/src/main.rs", true).len(), 1);
}
