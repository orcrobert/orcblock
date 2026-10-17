//! Claude Code hooks for this repository: guards that keep agents inside the
//! git workflow, a formatter, session context and a stop check.

mod format;
mod guard_files;
mod guard_git;
mod guard_tests;
mod hook;
mod runner;
mod session_context;
mod shell;
mod stop_check;
#[cfg(test)]
mod temp_dir;

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use hook::{Env, HookInput, Outcome};
use runner::SystemRunner;

const USAGE: &str =
    "usage: agenthooks session-context|guard-git|guard-files|guard-tests|format|stop-check";

fn main() -> ExitCode {
    let name = std::env::args().nth(1).unwrap_or_default();
    let mut stdin = String::new();
    let outcome = match io::stdin().read_to_string(&mut stdin) {
        Ok(_) => run(&name, &stdin),
        Err(err) => Err(format!("read hook input: {err}")),
    };
    let (code, stdout, stderr) = match outcome {
        Ok(outcome) => outcome.into_parts(),
        Err(message) => {
            let code = if name.starts_with("guard-") { 2 } else { 1 };
            (
                code,
                String::new(),
                format!("agenthooks {name}: {message}\n"),
            )
        }
    };
    let _ = io::stdout().lock().write_all(stdout.as_bytes());
    let _ = io::stderr().lock().write_all(stderr.as_bytes());
    ExitCode::from(code)
}

fn run(name: &str, stdin: &str) -> Result<Outcome, String> {
    let hook: fn(&HookInput, &Env<'_>) -> Outcome = match name {
        "session-context" => session_context::run,
        "guard-git" => guard_git::run,
        "guard-files" => guard_files::run,
        "guard-tests" => guard_tests::run,
        "format" => format::run,
        "stop-check" => stop_check::run,
        _ => return Err(USAGE.to_owned()),
    };
    let input: HookInput =
        serde_json::from_str(stdin).map_err(|err| format!("decode hook input: {err}"))?;
    let project_dir = std::env::var_os("CLAUDE_PROJECT_DIR")
        .map(PathBuf::from)
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| input.cwd.clone());
    let env = Env {
        project_dir,
        runner: &SystemRunner,
    };
    Ok(hook(&input, &env))
}
