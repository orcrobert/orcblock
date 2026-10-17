use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::runner::Runner;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HookInput {
    #[serde(default)]
    pub(crate) cwd: PathBuf,
    #[serde(default)]
    pub(crate) tool_input: ToolArgs,
    #[serde(default)]
    pub(crate) stop_hook_active: bool,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ToolArgs {
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) file_path: String,
}

pub(crate) struct Env<'a> {
    pub(crate) project_dir: PathBuf,
    pub(crate) runner: &'a dyn Runner,
}

impl Env<'_> {
    /// Runs git in the project directory and returns trimmed stdout, or an
    /// empty string when git fails.
    pub(crate) fn git(&self, args: &[&str]) -> String {
        self.runner
            .run(&self.project_dir, "git", args, Duration::from_secs(10))
            .map(|out| out.trim().to_owned())
            .unwrap_or_default()
    }

    pub(crate) fn rel_path(&self, path: &str) -> PathBuf {
        rel_path(&self.project_dir, Path::new(path))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Allow,
    Block(String),
    Ask(String),
    Context(String),
}

impl Outcome {
    pub(crate) fn into_parts(self) -> (u8, String, String) {
        match self {
            Self::Allow => (0, String::new(), String::new()),
            Self::Context(text) => (0, text, String::new()),
            Self::Block(reason) => (2, String::new(), reason),
            Self::Ask(reason) => {
                let json = serde_json::json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "ask",
                        "permissionDecisionReason": reason,
                    }
                });
                (0, format!("{json}\n"), String::new())
            }
        }
    }
}

/// Lexically cleans a path: removes `.` and resolves `..` against earlier
/// components without touching the filesystem.
pub(crate) fn clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

pub(crate) fn rel_path(project_dir: &Path, path: &Path) -> PathBuf {
    let path = clean(path);
    if !path.is_absolute() || project_dir.as_os_str().is_empty() {
        return path;
    }
    path.strip_prefix(clean(project_dir))
        .map_or(path.clone(), Path::to_path_buf)
}

pub(crate) fn join_dir(base: &Path, path: &str) -> PathBuf {
    if path.starts_with('/') || base.as_os_str().is_empty() {
        clean(Path::new(path))
    } else {
        clean(&base.join(path))
    }
}

#[cfg(test)]
mod tests;
