use std::path::{Path, PathBuf};

use super::*;

fn segs(line: &str) -> Vec<Vec<String>> {
    split_commands(line)
}

fn v(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

#[test]
fn splits_operators_quotes_and_comments() {
    let cases: &[(&str, &str, Vec<Vec<String>>)] = &[
        ("simple", "git status", vec![v(&["git", "status"])]),
        (
            "chained",
            "cd a && git add x; git commit -m 'a b'",
            vec![
                v(&["cd", "a"]),
                v(&["git", "add", "x"]),
                v(&["git", "commit", "-m", "a b"]),
            ],
        ),
        (
            "double quotes",
            r#"echo "a \"b\" c""#,
            vec![v(&["echo", r#"a "b" c"#])],
        ),
        (
            "comment",
            "git status # git push -f",
            vec![v(&["git", "status"])],
        ),
        ("hash in word", "echo a#b", vec![v(&["echo", "a#b"])]),
        (
            "pipe and subshell",
            "(git log | head)",
            vec![v(&["git", "log"]), v(&["head"])],
        ),
        (
            "line continuation",
            "git \\\n status",
            vec![v(&["git", "status"])],
        ),
        (
            "heredoc body skipped",
            "cat <<EOF\ngit push -f\nEOF\ngit status",
            vec![v(&["cat"]), v(&["git", "status"])],
        ),
        (
            "quoted heredoc with tabs",
            "cat <<-'X'\n\tgit add .\n\tX\necho ok",
            vec![v(&["cat"]), v(&["echo", "ok"])],
        ),
        (
            "herestring is not heredoc",
            "cat <<< word\ngit push -f",
            vec![v(&["cat", "word"]), v(&["git", "push", "-f"])],
        ),
        ("empty", "", vec![]),
    ];
    for (name, line, want) in cases {
        assert_eq!(&segs(line), want, "{name}");
    }
}

#[test]
fn parse_git_skips_globals_and_prefixes() {
    let cwd = Path::new("/repo");
    let call = parse_git(
        &v(&[
            "FOO=1",
            "sudo",
            "-E",
            "git",
            "-C",
            "sub",
            "-c",
            "a=b",
            "--no-pager",
            "commit",
            "-m",
            "x",
        ]),
        cwd,
    )
    .expect("is git");
    assert_eq!(call.dir, PathBuf::from("/repo/sub"));
    assert_eq!(call.configs, v(&["a=b"]));
    assert_eq!(call.sub, "commit");
    assert_eq!(call.args, v(&["-m", "x"]));

    let call = parse_git(&v(&["/usr/bin/git", "--git-dir", "/x", "push"]), cwd).expect("is git");
    assert_eq!(call.sub, "push");

    assert!(parse_git(&v(&["echo", "git"]), cwd).is_none());
    assert!(parse_git(&v(&["FOO=1"]), cwd).is_none());
}

#[test]
fn commands_expand_shell_scripts() {
    let all = commands("bash -lc 'cd x && git push -f'");
    assert!(all.contains(&v(&["git", "push", "-f"])), "{all:?}");
    let nested = commands(r#"sh -c "zsh -c 'git add .'""#);
    assert!(nested.contains(&v(&["git", "add", "."])), "{nested:?}");
}

#[test]
fn lexer_never_panics_on_odd_input() {
    let inputs = [
        "'",
        "\"",
        "\\",
        "<<",
        "<<-",
        "cat <<",
        "a <<EOF",
        "#",
        "\"\\",
        "<<'",
        "x\n<<E\n",
        "é <<É\nÉ",
    ];
    for input in inputs {
        let _ = commands(input);
    }
}
