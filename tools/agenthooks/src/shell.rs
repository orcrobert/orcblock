//! A deliberately small shell lexer: it understands quotes, escapes, comments,
//! heredocs and the operators that chain commands, which is enough to find
//! every git invocation in a compound command line.

use std::path::{Path, PathBuf};

use crate::hook::join_dir;

const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash"];
const WRAPPERS: &[&str] = &[
    "command", "exec", "env", "time", "nice", "nohup", "sudo", "xargs",
];
const MAX_SHELL_DEPTH: usize = 3;

pub(crate) fn split_commands(line: &str) -> Vec<Vec<String>> {
    let mut lexer = Lexer::default();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' => {
                lexer.in_token = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    lexer.cur.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                lexer.in_token = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && chars.get(i + 1).is_some_and(|n| "\"\\$`".contains(*n)) {
                        i += 1;
                    }
                    lexer.cur.push(chars[i]);
                    i += 1;
                }
            }
            '\\' => {
                if let Some(&next) = chars.get(i + 1) {
                    i += 1;
                    if next != '\n' {
                        lexer.in_token = true;
                        lexer.cur.push(next);
                    }
                }
            }
            '<' if chars.get(i + 1) == Some(&'<') && chars.get(i + 2) == Some(&'<') => {
                lexer.end_token();
                i += 2;
            }
            '<' if chars.get(i + 1) == Some(&'<') => {
                lexer.end_token();
                let (heredoc, last) = read_heredoc_word(&chars, i + 2);
                if !heredoc.word.is_empty() {
                    lexer.heredocs.push(heredoc);
                }
                i = last;
            }
            '\n' if !lexer.heredocs.is_empty() => {
                lexer.end_segment();
                for heredoc in std::mem::take(&mut lexer.heredocs) {
                    i = skip_heredoc_body(&chars, i + 1, &heredoc);
                }
            }
            '#' if !lexer.in_token => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                lexer.end_segment();
            }
            '\n' | ';' | '&' | '|' | '(' | ')' => lexer.end_segment(),
            ' ' | '\t' => lexer.end_token(),
            _ => {
                lexer.in_token = true;
                lexer.cur.push(c);
            }
        }
        i += 1;
    }
    lexer.end_segment();
    lexer.segments
}

#[derive(Default)]
struct Lexer {
    segments: Vec<Vec<String>>,
    tokens: Vec<String>,
    cur: String,
    in_token: bool,
    heredocs: Vec<Heredoc>,
}

impl Lexer {
    fn end_token(&mut self) {
        if self.in_token {
            self.tokens.push(std::mem::take(&mut self.cur));
            self.in_token = false;
        }
    }

    fn end_segment(&mut self) {
        self.end_token();
        if !self.tokens.is_empty() {
            self.segments.push(std::mem::take(&mut self.tokens));
        }
    }
}

struct Heredoc {
    word: String,
    strip_tabs: bool,
}

/// Parses the delimiter after `<<` and returns the index of its last char.
fn read_heredoc_word(chars: &[char], mut i: usize) -> (Heredoc, usize) {
    let strip_tabs = chars.get(i) == Some(&'-');
    if strip_tabs {
        i += 1;
    }
    while matches!(chars.get(i), Some(' ' | '\t')) {
        i += 1;
    }
    let mut word = String::new();
    while let Some(&c) = chars.get(i) {
        if matches!(c, '\'' | '"' | '\\') {
            i += 1;
            continue;
        }
        if matches!(
            c,
            ' ' | '\t' | '\n' | ';' | '&' | '|' | '<' | '>' | '(' | ')'
        ) {
            break;
        }
        word.push(c);
        i += 1;
    }
    (Heredoc { word, strip_tabs }, i.saturating_sub(1))
}

/// Returns the index of the newline that ends the terminator line, or the end
/// of input when the heredoc is never closed.
fn skip_heredoc_body(chars: &[char], start: usize, heredoc: &Heredoc) -> usize {
    let mut i = start;
    while i <= chars.len() {
        let end = chars[i..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(chars.len(), |p| i + p);
        let line: String = chars[i..end].iter().collect();
        let line = if heredoc.strip_tabs {
            line.trim_start_matches('\t')
        } else {
            &line
        };
        if line == heredoc.word {
            return end;
        }
        i = end + 1;
    }
    chars.len()
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GitCall {
    pub(crate) dir: PathBuf,
    pub(crate) configs: Vec<String>,
    pub(crate) sub: String,
    pub(crate) args: Vec<String>,
}

/// Returns the git invocation in a segment, skipping environment assignments,
/// wrappers and git's global options.
pub(crate) fn parse_git(tokens: &[String], cwd: &Path) -> Option<GitCall> {
    let mut i = skip_prefixes(tokens);
    let program = tokens.get(i)?;
    if program != "git" && !program.ends_with("/git") {
        return None;
    }
    let mut call = GitCall {
        dir: cwd.to_path_buf(),
        ..GitCall::default()
    };
    i += 1;
    while i < tokens.len() {
        let token = tokens[i].as_str();
        match token {
            "-C" if i + 1 < tokens.len() => {
                i += 1;
                call.dir = join_dir(&call.dir, &tokens[i]);
            }
            "-c" if i + 1 < tokens.len() => {
                i += 1;
                call.configs.push(tokens[i].clone());
            }
            t if t.starts_with("--git-dir")
                || t.starts_with("--work-tree")
                || t.starts_with("--namespace") =>
            {
                if !t.contains('=') {
                    i += 1;
                }
            }
            t if t.starts_with('-') => {}
            _ => {
                token.clone_into(&mut call.sub);
                call.args = tokens[i + 1..].to_vec();
                return Some(call);
            }
        }
        i += 1;
    }
    Some(call)
}

/// Returns the index of the real command after environment assignments and
/// wrapper commands such as `time` or `sudo -u x`.
pub(crate) fn skip_prefixes(tokens: &[String]) -> usize {
    let mut i = 0;
    while let Some(token) = tokens.get(i) {
        let is_wrapper = WRAPPERS.contains(&token.as_str());
        let is_wrapper_flag =
            token.starts_with('-') && i > 0 && WRAPPERS.contains(&tokens[i - 1].as_str());
        let is_assignment = token.contains('=') && !token.starts_with('-');
        if !is_wrapper && !is_wrapper_flag && !is_assignment {
            return i;
        }
        i += 1;
    }
    i
}

/// Splits a command line and expands `sh -c '<script>'` so the script's
/// commands are checked too.
pub(crate) fn commands(line: &str) -> Vec<Vec<String>> {
    commands_at(line, 0)
}

fn commands_at(line: &str, depth: usize) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for segment in split_commands(line) {
        let i = skip_prefixes(&segment);
        let is_shell = segment.get(i).is_some_and(|program| {
            let base = program.rsplit('/').next().unwrap_or(program);
            SHELLS.contains(&base)
        });
        if depth < MAX_SHELL_DEPTH && is_shell {
            let script = segment[i + 1..].windows(2).find_map(|pair| {
                let flag = &pair[0];
                let is_c = flag == "-c"
                    || (flag.starts_with('-') && !flag.starts_with("--") && flag.ends_with('c'));
                is_c.then(|| pair[1].clone())
            });
            if let Some(script) = script {
                out.extend(commands_at(&script, depth + 1));
            }
        }
        out.push(segment);
    }
    out
}

#[cfg(test)]
mod tests;
