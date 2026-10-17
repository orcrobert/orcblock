use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::hook::{Env, HookInput, Outcome};
use crate::runner::RunError;

const BUDGET: Duration = Duration::from_secs(100);
const ALLOWED_COMMENTS: &[&str] = &["///", "//!", "// SAFETY:"];

pub(crate) fn run(input: &HookInput, env: &Env<'_>) -> Outcome {
    if input.stop_hook_active {
        return Outcome::Allow;
    }
    let deadline = Instant::now() + BUDGET;

    let mut changed = BTreeSet::new();
    let base = env.git(&["merge-base", "origin/main", "HEAD"]);
    if !base.is_empty() {
        changed.extend(rust_files(&env.git(&["diff", "--name-only", &base])));
    }
    changed.extend(rust_files(&env.git(&["diff", "--name-only", "HEAD"])));
    let untracked = rust_files(&env.git(&["ls-files", "--others", "--exclude-standard"]));
    changed.extend(untracked.iter().cloned());

    let packages = changed_packages(&env.project_dir, &changed);
    if packages.is_empty() {
        return Outcome::Allow;
    }

    let mut problems = Vec::new();
    let mut selectors: Vec<&str> = Vec::new();
    for package in &packages {
        selectors.extend(["-p", package]);
    }
    let steps: [(&str, Vec<&str>); 2] = [
        (
            "clippy",
            [
                &["clippy", "--quiet", "--all-targets"][..],
                &selectors,
                &["--", "-D", "warnings"],
            ]
            .concat(),
        ),
        ("test", [&["test", "--quiet"][..], &selectors].concat()),
    ];
    for (name, args) in steps {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match env.runner.run(&env.project_dir, "cargo", &args, remaining) {
            Ok(_) => {}
            Err(RunError::TimedOut) => {
                problems.push(format!(
                    "stop-check ran out of time during cargo {name}; this is not a failure. Run make test yourself before finishing."
                ));
                break;
            }
            Err(RunError::Failed(out) | RunError::Spawn(out)) => {
                problems.push(format!("cargo {name} failed:\n{}", out.trim()));
                break;
            }
        }
    }

    let mut comments = added_comments(&env.git(&["diff", "-U0", "HEAD", "--", "*.rs"]));
    for file in &untracked {
        if let Ok(src) = fs::read_to_string(env.project_dir.join(file)) {
            comments.extend(comments_in_file(file, &src));
        }
    }
    if !comments.is_empty() {
        problems.push(format!(
            "Uncommitted changes add these comment lines. AGENTS.md rule 10: delete any that narrate what the code does or restate a name; keep only why-comments. If every one is justified, you may stop.\n{}",
            comments.join("\n")
        ));
    }

    if problems.is_empty() {
        Outcome::Allow
    } else {
        Outcome::Block(problems.join("\n\n"))
    }
}

fn rust_files(listing: &str) -> Vec<String> {
    listing
        .split_whitespace()
        .filter(|f| {
            Path::new(f)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
        })
        .map(str::to_owned)
        .collect()
}

/// Maps changed files to the names of the workspace packages that own them,
/// skipping deleted files and fixtures.
fn changed_packages(root: &Path, files: &BTreeSet<String>) -> BTreeSet<String> {
    files
        .iter()
        .map(PathBuf::from)
        .filter(|f| !f.iter().any(|c| c == "testdata"))
        .filter(|f| root.join(f).exists())
        .filter_map(|f| owning_package(root, &f))
        .collect()
}

fn owning_package(root: &Path, file: &Path) -> Option<String> {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if let Ok(manifest) = fs::read_to_string(root.join(d).join("Cargo.toml"))
            && let Some(name) = package_name(&manifest)
        {
            return Some(name);
        }
        dir = d.parent();
    }
    None
}

fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package
            && let Some(value) = line.strip_prefix("name").map(str::trim_start)
            && let Some(value) = value.strip_prefix('=')
        {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

fn added_comments(diff: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut file = "";
    let mut line = 0usize;
    for l in diff.lines() {
        if let Some(path) = l.strip_prefix("+++ ") {
            file = path.strip_prefix("b/").unwrap_or(path);
        } else if l.starts_with("@@") {
            line = hunk_start(l).unwrap_or(line);
        } else if let Some(added) = l.strip_prefix('+') {
            if let Some(comment) = comment(added) {
                out.push(format!("{file}:{line}: {comment}"));
            }
            line += 1;
        }
    }
    out
}

/// Parses the new-file start line from `@@ -a,b +c,d @@`.
fn hunk_start(header: &str) -> Option<usize> {
    let plus = header
        .split_whitespace()
        .find(|part| part.starts_with('+'))?;
    plus[1..].split(',').next()?.parse().ok()
}

fn comments_in_file(name: &str, src: &str) -> Vec<String> {
    src.lines()
        .enumerate()
        .filter_map(|(i, l)| comment(l).map(|c| format!("{name}:{}: {c}", i + 1)))
        .collect()
}

fn comment(line: &str) -> Option<&str> {
    let line = line.trim();
    let idx = line.find("//")?;
    let code = &line[..idx];
    if code.matches('"').count() % 2 == 1 || code.ends_with(':') || code.contains("r#\"") {
        return None;
    }
    let comment = &line[idx..];
    if ALLOWED_COMMENTS.iter().any(|p| comment.starts_with(p)) {
        return None;
    }
    Some(comment)
}

#[cfg(test)]
mod tests;
