---
name: status
description: Reports project progress, open pull requests and the next three tasks. Use only when the user runs /status.
disable-model-invocation: true
allowed-tools: Bash(gh pr list *) Bash(git log *) Bash(git status *)
---

## Context
- Branch and working tree: !`git status -sb`
- Recent commits on main: !`git log --oneline -10 origin/main 2>/dev/null || git log --oneline -10`
- Open pull requests: !`gh pr list --state open --json number,title,isDraft,headRefName --jq '.[] | "#\(.number) \(if .isDraft then "[draft] " else "" end)\(.title) (\(.headRefName))"' 2>/dev/null || echo "gh unavailable"`
- Specs: !`grep -H '^Status:' docs/specs/[0-9]*.md 2>/dev/null | grep -v 0000-template || echo "no specs yet"`

## Steps
1. Read docs/roadmap.md. Count its checked and unchecked boxes per section.
2. Note any approved spec that is not done, and any open PR that belongs to it.
3. Reply in this shape, nothing more:
   - Roadmap: X of Y items done, with one line per section that has open items.
   - In flight: approved specs and open PRs, one line each with their state.
   - Next three tasks: the next unchecked items in roadmap order, skipping anything already in flight. For each, say whether it needs /feature, /bug or a plain change.
4. Do not start any of the tasks.
