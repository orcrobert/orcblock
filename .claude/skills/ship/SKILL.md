---
name: ship
description: Verifies, reviews, pushes the current branch and opens a draft pull request. Use only when the user runs /ship.
disable-model-invocation: true
allowed-tools: Bash(git push -u origin *) Bash(gh pr create *)
---

## Context
- Branch: !`git branch --show-current`
- Commits: !`git log --oneline origin/main..HEAD 2>/dev/null || true`
- Files: !`git diff --stat origin/main...HEAD 2>/dev/null || true`

## Steps
1. On main: stop.
2. Run the verify skill. Stop on failure.
3. Run the code-reviewer and security-reviewer subagents on the diff, plus ui-reviewer if app/src changed. Fix every blocker and should finding in new commits; list nits in the PR.
4. Every commit header must match Conventional Commits. If one doesn't and the branch was never pushed, fix it with fixup commits and an autosquash rebase.
5. git push -u origin HEAD
6. gh pr create --draft --base main with a Conventional Commit title for the whole change and the body from .github/pull_request_template.md, every section filled, Closes #n when there is an issue.
7. Reply with the PR URL. Never merge, approve or mark ready.
