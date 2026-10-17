---
name: build
description: Implements an approved spec from docs/specs slice by slice with logical commits. Use only when the user runs /build.
disable-model-invocation: true
argument-hint: "[spec path or number]"
---

Spec: $ARGUMENTS

1. The spec must say Status: approved. If not, stop and ask.
2. git switch main && git pull --ff-only && git switch -c feat/<spec-slug>
3. First commit: the spec, docs(specs): add NNNN <slug>.
4. For each slice, in order: tests first where AGENTS.md requires it, then the smallest implementation that passes, committed in logical steps per the git-workflow skill. Tick the slice's acceptance criteria in the spec within the slice's last commit.
5. If the spec proves wrong, stop and propose a spec change instead of improvising.
6. When every slice is done, say so and suggest /ship.
