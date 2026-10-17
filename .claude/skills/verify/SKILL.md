---
name: verify
description: Runs the project quality gate (make check) and fixes what fails. Use before every commit, after finishing a slice, and whenever asked to check, test or verify changes.
---

1. Run make check. If it passes and nothing under app/ or crates/orcblockd changed, you are done.
2. On failure, read the first failing step, fix the cause in code and run make check again. Never edit tests, lint config, deny rules or hooks to get green.
3. If crates/orcblockd changed, run the run-orcblock recipe for the dev daemon and drive it with the CLI: status, add a site, start a one-minute block, try to remove the site (must fail), check the dry-run enforcer log.
4. If app/ changed, start the app against the dev daemon and click through the changed screens in light and dark.
5. Report what ran, what failed and what you fixed.
