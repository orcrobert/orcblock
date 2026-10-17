use std::path::PathBuf;

use super::*;
use crate::hook::{HookInput, ToolArgs};
use crate::runner::fake::FakeRunner;

fn decide(branch: &'static str, command: &str) -> Outcome {
    let runner = FakeRunner::new(move |line| {
        assert_eq!(line, "git branch --show-current", "unexpected command");
        Ok(format!("{branch}\n"))
    });
    let env = Env {
        project_dir: PathBuf::from("/repo"),
        runner: &runner,
    };
    let input = HookInput {
        cwd: PathBuf::from("/repo"),
        tool_input: ToolArgs {
            command: command.to_owned(),
            ..ToolArgs::default()
        },
        ..HookInput::default()
    };
    run(&input, &env)
}

fn label(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Allow => "allow",
        Outcome::Block(_) => "block",
        Outcome::Ask(_) => "ask",
        Outcome::Context(_) => "context",
    }
}

/// (name, current branch, command, expected decision)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("status is fine", "main", "git status", "allow"),
    (
        "commit on main",
        "main",
        r#"git commit -m "feat: x""#,
        "block",
    ),
    (
        "commit on branch",
        "feat/x",
        r#"git commit -m "feat: x""#,
        "allow",
    ),
    (
        "commit on main after cd",
        "main",
        r#"cd /repo && git commit -m "x""#,
        "block",
    ),
    (
        "commit after creating a branch",
        "main",
        r#"git switch -c feat/x && git commit -m "feat: x""#,
        "allow",
    ),
    (
        "commit after checkout -b",
        "main",
        r#"git checkout -b fix/y; git commit -m "fix: y""#,
        "allow",
    ),
    (
        "commit after switching to main",
        "feat/x",
        r#"git switch main && git commit -m "x""#,
        "block",
    ),
    (
        "commit with -C on main",
        "main",
        "git -C /repo commit -m x",
        "block",
    ),
    ("commit detached head", "", "git commit -m x", "allow"),
    (
        "commit --no-verify",
        "feat/x",
        "git commit --no-verify -m x",
        "block",
    ),
    ("commit -n", "feat/x", "git commit -nm x", "block"),
    (
        "hooksPath override",
        "feat/x",
        "git -c core.hooksPath=/dev/null commit -m x",
        "block",
    ),
    (
        "message mentioning no-verify",
        "feat/x",
        r#"git commit -m "docs: explain --no-verify""#,
        "allow",
    ),
    ("git add -A", "feat/x", "git add -A", "block"),
    ("git add --all", "feat/x", "git add --all", "block"),
    ("git add dot", "feat/x", "git add .", "block"),
    (
        "git add paths",
        "feat/x",
        "git add crates/orcblock-core/src/lib.rs",
        "allow",
    ),
    (
        "add dot in chain",
        "feat/x",
        "git add . && git commit -m x",
        "block",
    ),
    (
        "push feature branch",
        "feat/x",
        "git push -u origin HEAD",
        "allow",
    ),
    (
        "push feature by name",
        "feat/x",
        "git push -u origin feat/x",
        "allow",
    ),
    (
        "push to main explicitly",
        "feat/x",
        "git push origin main",
        "ask",
    ),
    (
        "push HEAD while on main",
        "main",
        "git push -u origin HEAD",
        "ask",
    ),
    ("bare push on main", "main", "git push", "ask"),
    (
        "refspec to main",
        "feat/x",
        "git push origin feat/x:main",
        "ask",
    ),
    (
        "full ref to main",
        "feat/x",
        "git push origin HEAD:refs/heads/main",
        "ask",
    ),
    ("push --all", "feat/x", "git push --all origin", "ask"),
    (
        "force push long",
        "feat/x",
        "git push --force origin feat/x",
        "block",
    ),
    ("force push short", "feat/x", "git push -f", "block"),
    (
        "force push combined flags",
        "feat/x",
        "git push -uf origin feat/x",
        "block",
    ),
    (
        "force with lease",
        "feat/x",
        "git push --force-with-lease",
        "block",
    ),
    (
        "force via plus refspec",
        "feat/x",
        "git push origin +feat/x",
        "block",
    ),
    (
        "push --no-verify",
        "feat/x",
        "git push --no-verify origin feat/x",
        "block",
    ),
    (
        "push -o value is not a refspec",
        "feat/x",
        "git push -o ci.skip origin feat/x",
        "allow",
    ),
    (
        "quoted git text is not a command",
        "main",
        r#"echo "git commit -m x""#,
        "allow",
    ),
    (
        "env prefix",
        "main",
        "GIT_EDITOR=true git commit -m x",
        "block",
    ),
    ("multiline", "main", "git status\ngit commit -m x", "block"),
    (
        "comment ignored",
        "main",
        "git status # git commit",
        "allow",
    ),
    ("not git", "main", "make check", "allow"),
    (
        "checkout main then commit",
        "feat/x",
        "git checkout main && git commit -m x",
        "block",
    ),
    (
        "checkout main then bare push",
        "feat/x",
        "git stash && git checkout main && git push",
        "ask",
    ),
    (
        "checkout unknown then commit",
        "main",
        "git checkout feat/y && git commit -m x",
        "ask",
    ),
    (
        "checkout previous then commit",
        "feat/x",
        "git switch - && git commit -m x",
        "ask",
    ),
    (
        "checkout path keeps branch",
        "feat/x",
        "git checkout -- Cargo.toml && git commit -m x",
        "allow",
    ),
    (
        "heredoc body with quote",
        "feat/x",
        "git commit -F - <<'EOF'\nfix: don't break\nEOF\ngit push origin main",
        "ask",
    ),
    (
        "heredoc body is not run",
        "feat/x",
        "cat <<EOF > notes.txt\ngit push --force\nEOF",
        "allow",
    ),
    (
        "heredoc without space",
        "feat/x",
        "cat<<-EOF\n\tgit add -A\n\tEOF\ngit status",
        "allow",
    ),
    (
        "config hooksPath",
        "feat/x",
        "git config core.hooksPath /dev/null",
        "block",
    ),
    (
        "unset hooksPath",
        "feat/x",
        "git config --unset core.hooksPath",
        "block",
    ),
    (
        "make hooks value is fine",
        "feat/x",
        "git config core.hooksPath scripts/hooks",
        "allow",
    ),
    ("time wrapper", "main", "time git commit -m x", "block"),
    (
        "sudo with flags",
        "main",
        "sudo -E git commit -m x",
        "block",
    ),
    (
        "sh -c force push",
        "feat/x",
        r#"sh -c "git push --force""#,
        "block",
    ),
    (
        "bash -lc commit on main",
        "main",
        "bash -lc 'git commit -m x'",
        "block",
    ),
    ("add cluster with A", "feat/x", "git add -Av", "block"),
    (
        "message starting with -n",
        "feat/x",
        r#"git commit -m "-n fix""#,
        "allow",
    ),
    (
        "signing key containing n",
        "feat/x",
        "git commit -Skeyn -m x",
        "allow",
    ),
    (
        "cd into another repo dir",
        "main",
        "cd ../wt && git commit -m x",
        "block",
    ),
];

#[test]
fn decisions() {
    for &(name, branch, command, want) in CASES {
        let outcome = decide(branch, command);
        assert_eq!(label(&outcome), want, "{name}: {outcome:?}");
    }
}

#[test]
fn commit_on_main_hints_at_branching() {
    let Outcome::Block(reason) = decide("main", "git commit -m x") else {
        panic!("expected a block");
    };
    assert!(reason.contains("git switch -c"), "{reason}");
}
