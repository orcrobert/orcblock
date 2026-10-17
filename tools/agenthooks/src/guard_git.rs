use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::hook::{Env, HookInput, Outcome, clean, join_dir};
use crate::shell::{commands, parse_git};

const MAIN: &str = "main";
const UNKNOWN: &str = "\0unknown";
const HOOKS_PATH: &str = "scripts/hooks";

const COMMIT_OPTS_WITH_VALUE: &[&str] = &[
    "-m",
    "-F",
    "-C",
    "-c",
    "-t",
    "--message",
    "--file",
    "--author",
    "--date",
    "--template",
    "--reuse-message",
    "--reedit-message",
    "--fixup",
    "--squash",
    "--trailer",
];
const PUSH_OPTS_WITH_VALUE: &[&str] =
    &["-o", "--push-option", "--repo", "--receive-pack", "--exec"];

pub(crate) fn run(input: &HookInput, env: &Env<'_>) -> Outcome {
    let mut branches = BranchCache::new(env);
    let mut ask_reasons = Vec::new();
    let mut cwd = clean(&input.cwd);
    for segment in commands(&input.tool_input.command) {
        if segment[0] == "cd" {
            if let Some(dir) = segment.get(1) {
                cwd = join_dir(&cwd, dir);
            }
            continue;
        }
        let Some(call) = parse_git(&segment, &cwd) else {
            continue;
        };
        if call
            .configs
            .iter()
            .any(|c| c.to_ascii_lowercase().starts_with("core.hookspath"))
        {
            return Outcome::Block(
                "Overriding core.hooksPath skips the repo's git hooks. Run the command without it and fix whatever the hooks report.".into(),
            );
        }
        match call.sub.as_str() {
            "config" if changes_hooks_path(&call.args) => {
                return Outcome::Block(format!(
                    "core.hooksPath must stay {HOOKS_PATH} (make hooks sets it). Fix whatever the hooks report instead."
                ));
            }
            "switch" | "checkout" => {
                if let Some(branch) = switch_target(&call.sub, &call.args) {
                    branches.set(&call.dir, branch);
                }
            }
            "add" => {
                if let Some(reason) = check_add(&call.args) {
                    return Outcome::Block(reason.into());
                }
            }
            "commit" => {
                if commit_skips_hooks(&call.args) {
                    return Outcome::Block(
                        "--no-verify skips the commit hooks. Fix what they report instead.".into(),
                    );
                }
                match branches.get(&call.dir).as_str() {
                    MAIN => {
                        return Outcome::Block(
                            "Never commit on main. Create a branch first: git switch -c <type>/<slug>".into(),
                        );
                    }
                    UNKNOWN => ask_reasons.push(
                        "Cannot tell which branch this commit lands on. Commits on main are not allowed.",
                    ),
                    _ => {}
                }
            }
            "push" => {
                let push = Push::parse(&call.args);
                if push.no_verify {
                    return Outcome::Block(
                        "--no-verify skips the push hooks. Fix what they report instead.".into(),
                    );
                }
                if push.force {
                    return Outcome::Block(
                        "Force pushes are not allowed. Add a new commit on top instead.".into(),
                    );
                }
                if push.targets_main(&branches.get(&call.dir)) {
                    ask_reasons.push(
                        "This push may target main. Pushes to main need the user's explicit confirmation.",
                    );
                }
            }
            _ => {}
        }
    }
    if ask_reasons.is_empty() {
        Outcome::Allow
    } else {
        Outcome::Ask(ask_reasons.join(" "))
    }
}

/// Current branch per directory, updated by switches seen earlier in the same
/// command line.
struct BranchCache<'a> {
    env: &'a Env<'a>,
    known: HashMap<PathBuf, String>,
}

impl<'a> BranchCache<'a> {
    fn new(env: &'a Env<'a>) -> Self {
        Self {
            env,
            known: HashMap::new(),
        }
    }

    fn set(&mut self, dir: &Path, branch: String) {
        self.known.insert(dir.to_path_buf(), branch);
    }

    fn get(&mut self, dir: &Path) -> String {
        let runner = self.env.runner;
        self.known
            .entry(dir.to_path_buf())
            .or_insert_with(|| {
                runner
                    .run(
                        dir,
                        "git",
                        &["branch", "--show-current"],
                        Duration::from_secs(5),
                    )
                    .map(|out| out.trim().to_owned())
                    .unwrap_or_default()
            })
            .clone()
    }
}

/// Returns the branch a switch or checkout moves to. A plain checkout argument
/// may be a branch or a path, so anything other than main becomes unknown
/// rather than a guess.
fn switch_target(sub: &str, args: &[String]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "-c" | "-C" | "-b" | "-B" | "--create" | "--force-create" | "--orphan"
                if i + 1 < args.len() =>
            {
                return Some(args[i + 1].clone());
            }
            "--" => return None,
            "-" => return Some(UNKNOWN.into()),
            a if a.starts_with('-') => {}
            a if sub == "switch" || a == MAIN => return Some(a.to_owned()),
            _ => return Some(UNKNOWN.into()),
        }
    }
    None
}

fn changes_hooks_path(args: &[String]) -> bool {
    args.iter()
        .position(|a| a.eq_ignore_ascii_case("core.hooksPath"))
        .is_some_and(|i| args.get(i + 1).is_none_or(|value| value != HOOKS_PATH))
}

fn check_add(args: &[String]) -> Option<&'static str> {
    for arg in args {
        if arg == "--all" || arg == "--no-ignore-removal" || is_short_flag_set(arg, 'A') {
            return Some(
                "git add -A stages everything, including files you did not mean to commit. Stage explicit paths: git add <file>...",
            );
        }
        if matches!(arg.as_str(), "." | "./" | ":/" | "*") {
            return Some(
                "git add . stages everything, including files you did not mean to commit. Stage explicit paths: git add <file>...",
            );
        }
    }
    None
}

/// Finds `-n` in short option clusters, stopping at the first letter that
/// takes a value, so `-nm msg` counts and `-m -n` does not.
fn commit_skips_hooks(args: &[String]) -> bool {
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--" => return false,
            "--no-verify" => return true,
            a if COMMIT_OPTS_WITH_VALUE.contains(&a) => i += 1,
            a if a.starts_with("--") => {}
            a if a.starts_with('-') => {
                for c in a.chars().skip(1) {
                    if c == 'n' {
                        return true;
                    }
                    if "mFCctS".contains(c) {
                        break;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

fn is_short_flag_set(arg: &str, flag: char) -> bool {
    arg.len() >= 2 && arg.starts_with('-') && !arg.starts_with("--") && arg[1..].contains(flag)
}

#[derive(Debug, Default)]
struct Push {
    force: bool,
    no_verify: bool,
    all: bool,
    refspecs: Vec<String>,
}

impl Push {
    fn parse(args: &[String]) -> Self {
        let mut push = Self::default();
        let mut has_remote = false;
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "--" => {}
                "--force" | "--force-if-includes" => push.force = true,
                a if a.starts_with("--force-with-lease") => push.force = true,
                "--no-verify" => push.no_verify = true,
                "--all" | "--mirror" | "--branches" => push.all = true,
                a if PUSH_OPTS_WITH_VALUE.contains(&a) => i += 1,
                a if a.starts_with("--") => {}
                a if a.starts_with('-') => push.force |= is_short_flag_set(a, 'f'),
                _ if !has_remote => has_remote = true,
                a => {
                    push.force |= a.starts_with('+');
                    push.refspecs.push(a.to_owned());
                }
            }
            i += 1;
        }
        push
    }

    fn targets_main(&self, current: &str) -> bool {
        if self.all {
            return true;
        }
        if self.refspecs.is_empty() {
            return current == MAIN || current == UNKNOWN;
        }
        self.refspecs.iter().any(|spec| {
            let spec = spec.trim_start_matches('+');
            let dst = spec.rsplit_once(':').map_or(spec, |(_, dst)| dst);
            let dst = if dst == "HEAD" || dst == "@" {
                current
            } else {
                dst
            };
            let dst = dst.trim_start_matches("refs/heads/");
            dst == MAIN || dst == UNKNOWN
        })
    }
}

#[cfg(test)]
mod tests;
