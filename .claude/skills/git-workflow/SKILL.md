---
name: git-workflow
description: How to branch, split commits, write Conventional Commit messages and fill the pull request template in this repo. Use whenever creating a branch, staging, committing, rewording commits or opening a pull request.
user-invocable: false
---

The hooks enforce the hard rules (no commits on main, no force pushes, no git add -A, no --no-verify, Conventional Commit headers). This skill covers the judgement calls. Worked examples: ${CLAUDE_SKILL_DIR}/examples.md.

## Branch
- Start from a fresh main: `git switch main && git pull --ff-only && git switch -c <type>/<kebab-slug>`.
- `<type>` is the commit type of the main change; an issue number is optional: `fix/12-resolver-cleanup`.
- One branch per task. Parallel tasks run in separate worktrees (`claude --worktree`).

## Split commits
1. One logical change per commit. If the summary needs "and", split it.
2. Every commit builds and passes `make test`, so `git bisect` always works.
3. Tests go in the same commit as the code they test.
4. Renames, moves, formatting and refactors get their own commits, before the behaviour change that needs them.
5. A wire protocol change ships in the same commit as both sides that speak it, or the daemon first with the old shape still accepted.
6. Commit after each green step, not one big commit at the end, and leave a minute or two between commits rather than firing a burst.
7. Stage explicit paths, then read `git diff --staged` before every commit. Unstage anything that belongs to another commit.
8. Before the first push, tidy history with `git commit --fixup=<sha>` and `git rebase --autosquash main` (non-interactive: `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash main`). After the first push, only add commits.

## Message
- Header: `type(scope): summary`, at most 72 characters, imperative, lowercase first word, no trailing period.
- Types: feat (user-visible feature), fix, perf, refactor (no behaviour change), test, docs, build (Cargo, Makefile, bundling), ci, chore, revert.
- Scopes: core, ipc, daemon, enforce, dns, cli, app, ui, mascot, agents, ci, deps, docs. Specs use `docs(specs)`. Omit the scope only for repo-wide changes.
- Body: skip for trivial commits; otherwise explain why, constraints and rejected alternatives, wrapped at 72.
- Breaking change: `!` after the scope plus a `BREAKING CHANGE:` footer.
- Footers: `Refs: #n`. Closing issues happens in the PR body, not in commits.
- Keep Claude Code's `Co-Authored-By` trailer.

## Pull request
- Only /ship opens PRs: always draft, base main, title is one Conventional Commit header for the whole change.
- Body follows `.github/pull_request_template.md`. Fill every section; write "None" rather than deleting one.
- How it was verified: paste the `make check` result line and list manual steps you actually ran.
- Aim for under 400 changed lines excluding lockfiles and fixtures. Bigger work becomes several PRs, one per slice.
- Review feedback becomes new commits on the same branch. Never force-push once review starts.
- Never merge, approve or mark ready unless the user asks. Merging is rebase-only, so commits land on main exactly as written.
