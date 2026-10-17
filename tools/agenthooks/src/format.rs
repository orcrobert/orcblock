//! Never blocks: a formatting failure usually means a syntax error, which the
//! build in stop-check reports with better context.

use std::path::Path;
use std::time::Duration;

use crate::hook::{Env, HookInput, Outcome};

const PRETTIER_EXTENSIONS: &[&str] = &["ts", "js", "svelte", "css", "html", "json"];
const TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) fn run(input: &HookInput, env: &Env<'_>) -> Outcome {
    let path = &input.tool_input.file_path;
    let rel = env.rel_path(path);
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("rs") => {
            let _ = env.runner.run(
                &env.project_dir,
                "rustfmt",
                &["--edition", "2024", path],
                TIMEOUT,
            );
        }
        Some(ext) if PRETTIER_EXTENSIONS.contains(&ext) && rel.starts_with("app/src") => {
            let _ = env.runner.run(
                &env.project_dir.join("app"),
                "npx",
                &["--no-install", "prettier", "--write", path],
                TIMEOUT,
            );
        }
        _ => {}
    }
    Outcome::Allow
}

#[cfg(test)]
mod tests;
