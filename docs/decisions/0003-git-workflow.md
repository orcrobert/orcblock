# 0003: Git workflow

Status: accepted (2026-10-17)

## Context

Most changes will be written by coding agents. History must stay bisectable and reviewable, and no agent may change `main` without a human approving it.

## Decision

- Every change reaches `main` through a draft pull request opened by `/ship`.
- Branches are `<type>/<kebab-slug>` from a fresh main. One task per branch; parallel work in separate worktrees.
- Commits follow Conventional Commits 1.0: one logical change each, every commit builds and passes `make test`, tests in the same commit as their code, refactors in their own commits first. Commits are made as the work progresses, a minute or two apart, not in a burst at the end.
- Merge method is rebase-and-merge only, so logical commits land on main as written and the changelog can be generated from them. Releases are semver tags.
- Enforcement does not rely on agent memory: GitHub branch protection, the `commit-msg` and `pre-commit` git hooks, the `guard-git` Claude Code hook, the `verify` skill and the CI `commits` job.
- Claude Code's `Co-Authored-By` trailer stays on agent commits.

## Consequences

- PR history reads as a series of small, reviewable steps; reverting one step is a clean `git revert`.
- Force pushes are blocked, so review feedback is always new commits.
- Contributors without the hooks installed find out in CI rather than locally; `make hooks` fixes that.
