---
name: bug
description: Fixes a bug by reproducing it with a failing test first. Use only when the user runs /bug.
disable-model-invocation: true
argument-hint: "[description or #issue]"
---

Bug: $ARGUMENTS

1. Read the issue if a number is given, find the code path and write the smallest test that fails for the right reason.
2. git switch main && git pull --ff-only && git switch -c fix/<slug>
3. Fix the cause, not the symptom. Any refactor needed goes in its own commit first.
4. Commit the test and the fix together: fix(<scope>): <what now works>, with Refs: #n.
5. Say it is ready for /ship.
