use std::collections::HashMap;

use super::*;
use crate::runner::fake::FakeRunner;
use crate::temp_dir::TempDir;

const CLIPPY: &str = "cargo clippy --quiet --all-targets -p orcblock-core -- -D warnings";
type Override = (&'static str, Result<&'static str, RunError>);

const TEST: &str = "cargo test --quiet -p orcblock-core";

fn repo() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "crates/orcblock-core/Cargo.toml",
        "[package]\nname = \"orcblock-core\"\n\n[dependencies]\nname = \"x\"\n",
    );
    dir.write("crates/orcblock-core/src/lib.rs", "pub fn a() {}\n");
    dir.write("crates/orcblock-core/testdata/bad.rs", "fn broken(\n");
    dir.write(
        "crates/orcblock-core/src/new.rs",
        "/// Docs are fine.\n// narrates the code\npub fn b() {}\n",
    );
    dir
}

fn responses(
    overrides: &[(&str, Result<&str, RunError>)],
) -> HashMap<String, Result<String, RunError>> {
    let mut map: HashMap<String, Result<String, RunError>> = [
        ("git merge-base origin/main HEAD", Ok("abc")),
        (
            "git diff --name-only abc",
            Ok("crates/orcblock-core/src/lib.rs\nREADME.md\n"),
        ),
        ("git diff --name-only HEAD", Ok("")),
        ("git ls-files --others --exclude-standard", Ok("")),
        ("git diff -U0 HEAD -- *.rs", Ok("")),
        (CLIPPY, Ok("")),
        (TEST, Ok("")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.map(str::to_owned)))
    .collect();
    for (key, value) in overrides {
        map.insert((*key).to_owned(), value.clone().map(str::to_owned));
    }
    map
}

fn check(
    dir: &TempDir,
    overrides: &[(&str, Result<&str, RunError>)],
    active: bool,
) -> (Outcome, Vec<String>) {
    let map = responses(overrides);
    let runner = FakeRunner::new(move |line| {
        map.get(line)
            .cloned()
            .unwrap_or_else(|| panic!("unexpected command {line}"))
    });
    let env = Env {
        project_dir: dir.path().to_path_buf(),
        runner: &runner,
    };
    let input = HookInput {
        stop_hook_active: active,
        ..HookInput::default()
    };
    let outcome = run(&input, &env);
    (outcome, runner.commands())
}

#[test]
fn active_stop_hook_runs_nothing() {
    let dir = repo();
    let (outcome, calls) = check(&dir, &[], true);
    assert_eq!(outcome, Outcome::Allow);
    assert_eq!(calls, Vec::<String>::new());
}

#[test]
fn outcomes() {
    let failed = |out: &'static str| Err(RunError::Failed(out.into()));
    let cases: Vec<(&str, Vec<Override>, Option<&str>)> = vec![
        ("all green", vec![], None),
        (
            "no rust changes",
            vec![("git diff --name-only abc", Ok("README.md\n"))],
            None,
        ),
        (
            "fixture only",
            vec![(
                "git diff --name-only abc",
                Ok("crates/orcblock-core/testdata/bad.rs\n"),
            )],
            None,
        ),
        (
            "deleted file only",
            vec![("git diff --name-only abc", Ok("crates/gone/src/lib.rs\n"))],
            None,
        ),
        (
            "clippy fails",
            vec![(CLIPPY, failed("error: unused variable"))],
            Some("unused variable"),
        ),
        (
            "tests fail",
            vec![(TEST, failed("test potion ... FAILED"))],
            Some("cargo test failed"),
        ),
        (
            "timeout is not a failure",
            vec![(CLIPPY, Err(RunError::TimedOut))],
            Some("not a failure"),
        ),
        (
            "added comment",
            vec![(
                "git diff -U0 HEAD -- *.rs",
                Ok(
                    "+++ b/crates/orcblock-core/src/lib.rs\n@@ -1,0 +4,2 @@\n+// increment the counter\n+x += 1;\n",
                ),
            )],
            Some("crates/orcblock-core/src/lib.rs:4: // increment the counter"),
        ),
        (
            "doc comment only",
            vec![(
                "git diff -U0 HEAD -- *.rs",
                Ok("+++ b/a.rs\n@@ -1 +1 @@\n+/// Starts a block.\n"),
            )],
            None,
        ),
        (
            "untracked file comments",
            vec![(
                "git ls-files --others --exclude-standard",
                Ok("crates/orcblock-core/src/new.rs\n"),
            )],
            Some("crates/orcblock-core/src/new.rs:2: // narrates the code"),
        ),
    ];
    let dir = repo();
    for (name, overrides, want) in cases {
        let (outcome, _) = check(&dir, &overrides, false);
        match (outcome, want) {
            (Outcome::Allow, None) => {}
            (Outcome::Block(reason), Some(want)) => {
                assert!(reason.contains(want), "{name}: {reason}");
            }
            (outcome, want) => panic!("{name}: got {outcome:?}, want {want:?}"),
        }
    }
}

#[test]
fn added_comments_skips_code_strings_and_allowed_forms() {
    let diff = [
        "+++ b/x.rs",
        "@@ -10,0 +11,6 @@ fn f() {",
        "+    let n = 1; // trailing comment",
        r#"+    let url = "https://example.com";"#,
        "+    // SAFETY: the pointer is valid for the call",
        "+    //! inner doc",
        "+    // why: launchd restarts us within 10 s",
        r##"+    let s = r#"// not a comment"#;"##,
        "-// removed line",
    ]
    .join("\n");
    assert_eq!(
        added_comments(&diff),
        vec![
            "x.rs:11: // trailing comment".to_owned(),
            "x.rs:15: // why: launchd restarts us within 10 s".to_owned(),
        ]
    );
}

#[test]
fn package_name_reads_only_the_package_table() {
    assert_eq!(
        package_name("[workspace]\nname = \"no\"\n[package]\nname = \"yes\"\n"),
        Some("yes".into())
    );
    assert_eq!(package_name("[workspace]\nmembers = []\n"), None);
}
