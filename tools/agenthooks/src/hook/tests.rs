use std::path::{Path, PathBuf};

use super::*;

#[test]
fn clean_resolves_dots() {
    let cases = [
        ("/a/./b/../c", "/a/c"),
        ("a/b/../../..", ".."),
        ("./x", "x"),
        ("/repo/crates/../app", "/repo/app"),
    ];
    for (input, want) in cases {
        assert_eq!(
            clean(Path::new(input)),
            PathBuf::from(want),
            "clean({input})"
        );
    }
}

#[test]
fn rel_path_strips_project_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        rel_path(root, Path::new("/repo/a/b.rs")),
        PathBuf::from("a/b.rs")
    );
    assert_eq!(
        rel_path(root, Path::new("/other/x")),
        PathBuf::from("/other/x")
    );
    assert_eq!(rel_path(root, Path::new("rel/x")), PathBuf::from("rel/x"));
    assert_eq!(
        rel_path(Path::new(""), Path::new("/a")),
        PathBuf::from("/a")
    );
}

#[test]
fn ask_outcome_is_json_for_claude_code() {
    let (code, stdout, stderr) = Outcome::Ask("why".into()).into_parts();
    assert_eq!(code, 0);
    assert_eq!(stderr, "");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "ask");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecisionReason"],
        "why"
    );
}

#[test]
fn block_outcome_exits_two_with_reason_on_stderr() {
    assert_eq!(
        Outcome::Block("no".into()).into_parts(),
        (2, String::new(), "no".into())
    );
}

#[test]
fn input_tolerates_missing_fields() {
    let input: HookInput = serde_json::from_str(r#"{"hook_event_name":"Stop"}"#).expect("parses");
    assert_eq!(input.tool_input.command, "");
    assert!(!input.stop_hook_active);
}
