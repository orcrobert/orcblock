use std::path::Path;

use super::*;
use crate::runner::fake::FakeRunner;
use crate::temp_dir::TempDir;

fn context(root: &Path, branch: &'static str) -> String {
    let runner = FakeRunner::new(move |line| match line {
        "git branch --show-current" => Ok(format!("{branch}\n")),
        "git status -sb" => Ok("## feat/x\n".into()),
        "git log --oneline -5" => Ok("abc feat: x\n".into()),
        other => panic!("unexpected {other}"),
    });
    let env = Env {
        project_dir: root.to_path_buf(),
        runner: &runner,
    };
    match run(&HookInput::default(), &env) {
        Outcome::Context(text) => text,
        other => panic!("expected context, got {other:?}"),
    }
}

#[test]
fn warns_on_main_and_shows_git_state() {
    let dir = TempDir::new();
    let text = context(dir.path(), "main");
    assert!(text.contains("Branch: main"));
    assert!(text.contains("WARNING: you are on main"));
    assert!(text.contains("Recent commits:\nabc feat: x"));
}

#[test]
fn prefers_newest_approved_spec_with_open_items() {
    let dir = TempDir::new();
    dir.write(
        "docs/specs/0000-template.md",
        "Status: approved\n- [ ] template\n",
    );
    dir.write(
        "docs/specs/0001-old.md",
        "Status: approved\n- [ ] old item\n",
    );
    dir.write(
        "docs/specs/0002-done.md",
        "Status: approved\n- [x] finished\n",
    );
    dir.write("docs/specs/0003-draft.md", "Status: draft\n- [ ] not yet\n");
    dir.write("docs/roadmap.md", "- [ ] roadmap item\n");
    let text = context(dir.path(), "feat/x");
    assert!(!text.contains("WARNING"));
    assert!(
        text.contains("Active work: docs/specs/0001-old.md\n- [ ] old item"),
        "{text}"
    );
}

#[test]
fn falls_back_to_roadmap() {
    let dir = TempDir::new();
    dir.write(
        "docs/roadmap.md",
        "# Roadmap\n- [x] done\n- [ ] next thing\n",
    );
    let text = context(dir.path(), "feat/x");
    assert!(
        text.contains("Active work: docs/roadmap.md\n- [ ] next thing"),
        "{text}"
    );
}
