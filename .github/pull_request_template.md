## Summary
<!-- One or two sentences: what this PR changes. -->

## Why
<!-- The problem it solves. Link the spec: docs/specs/NNNN-slug.md -->

## Changes
<!-- Bullets per commit or area. -->

## How it was verified
<!-- Paste the make check result line, then the manual steps you actually ran (dev daemon, CLI, app). -->

## Screenshots
<!-- UI changes: before and after, light and dark. Otherwise "None". -->

## Bypass and safety
<!-- Daemon, protocol or enforcement changes: can this end a block early, change potions, or let another process misuse the daemon? Otherwise "None". -->

## Risks and rollback
<!-- What could break, and how to undo it (previous release, helper reinstall, state file notes). -->

## Follow-ups
<!-- Unrelated problems noticed along the way. "None" if none. -->

Closes #

## Checklist
- [ ] `make check` passes
- [ ] New behaviour is covered by tests
- [ ] Spec checkboxes are ticked
- [ ] Commits follow Conventional Commits, one logical change each
- [ ] No new dependencies, or an ADR is included
- [ ] Wire protocol changes bump `PROTOCOL_VERSION` when old clients would misread them
