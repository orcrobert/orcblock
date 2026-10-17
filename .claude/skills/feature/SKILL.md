---
name: feature
description: Turns a one-line feature idea or GitHub issue into a spec in docs/specs and stops for approval. Use only when the user runs /feature.
disable-model-invocation: true
argument-hint: "[idea or #issue]"
---

Feature request: $ARGUMENTS

1. If it is an issue number, read it with gh issue view.
2. Explore the relevant code with the Explore subagent and read docs/architecture.md and docs/roadmap.md.
3. Copy ${CLAUDE_SKILL_DIR}/spec-template.md to docs/specs/NNNN-<slug>.md (next free number) and fill every section. Acceptance criteria must be testable; each slice must be shippable on its own and under ~300 changed lines.
4. If the feature touches the daemon, the wire protocol or enforcement, fill the "Bypass and safety" section: every new way the block or potions could be affected.
5. Put unknowns under Open questions instead of guessing.
6. Stop. Show the spec path and a five-line summary, then wait. No branch, no code, no commit.
