use std::path::Path;

use crate::hook::{Env, HookInput, Outcome};

const TEST_DIRS: &[&str] = &["tests", "testdata", "benches", "fuzz"];

pub(crate) fn run(input: &HookInput, env: &Env<'_>) -> Outcome {
    let rel = env.rel_path(&input.tool_input.file_path);
    if is_test_path(&rel) {
        return Outcome::Allow;
    }
    Outcome::Block(format!(
        "The test-writer only edits tests: tests.rs and *.test.ts files, and anything under tests/, testdata/, benches/ or fuzz/. If {} needs a change to become testable, report it back instead of editing it.",
        rel.display()
    ))
}

fn is_test_path(rel: &Path) -> bool {
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let in_test_dir = rel
        .parent()
        .is_some_and(|dir| dir.iter().any(|c| TEST_DIRS.iter().any(|t| c == *t)));
    !rel.is_absolute()
        && !rel.starts_with("..")
        && (name == "tests.rs" || name.ends_with(".test.ts") || in_test_dir)
}

#[cfg(test)]
mod tests;
