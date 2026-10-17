use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::hook::{Env, HookInput, Outcome, rel_path};

const MAX_OPEN_ITEMS: usize = 10;

pub(crate) fn run(_input: &HookInput, env: &Env<'_>) -> Outcome {
    let mut out = String::new();
    let branch = env.git(&["branch", "--show-current"]);
    let _ = writeln!(out, "Branch: {branch}");
    if branch == "main" {
        out.push_str("WARNING: you are on main. Before changing anything: git switch main && git pull --ff-only && git switch -c <type>/<slug>\n");
    }
    for (title, args) in [
        ("Status", &["status", "-sb"][..]),
        ("Recent commits", &["log", "--oneline", "-5"][..]),
    ] {
        let text = env.git(args);
        if !text.is_empty() {
            let _ = write!(out, "\n{title}:\n{text}\n");
        }
    }
    if let Some((path, items)) = active_work(&env.project_dir) {
        let _ = writeln!(out, "\nActive work: {}", path.display());
        for item in items {
            let _ = writeln!(out, "{item}");
        }
    }
    Outcome::Context(out)
}

/// The newest approved spec with open items, else the roadmap's open items.
fn active_work(root: &Path) -> Option<(PathBuf, Vec<String>)> {
    let mut specs: Vec<PathBuf> = fs::read_dir(root.join("docs/specs"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_spec(path))
        .collect();
    specs.sort_unstable_by(|a, b| b.cmp(a));

    let spec = specs.into_iter().find_map(|path| {
        let doc = fs::read_to_string(&path).ok()?;
        if !doc.lines().any(|l| l.trim() == "Status: approved") {
            return None;
        }
        let items = open_items(&doc);
        (!items.is_empty()).then_some((path, items))
    });
    let found = spec.or_else(|| {
        let path = root.join("docs/roadmap.md");
        let items = open_items(&fs::read_to_string(&path).ok()?);
        (!items.is_empty()).then_some((path, items))
    });
    found.map(|(path, items)| (rel_path(root, &path), items))
}

fn is_spec(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let bytes = name.as_bytes();
    bytes.len() > 8
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && !name.starts_with("0000-")
        && Path::new(name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn open_items(doc: &str) -> Vec<String> {
    doc.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- [ ] "))
        .take(MAX_OPEN_ITEMS)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
