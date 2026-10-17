# Security policy

## Reporting a vulnerability

Please do not open a public issue. Report it privately through [GitHub security advisories](https://github.com/orcrobert/orcblock/security/advisories/new).

Include what you found, how to reproduce it and the impact you expect. You will get an acknowledgement within 7 days, and a fix or a plan within 30 days for confirmed issues.

## Supported versions

Until 1.0, only the latest release receives security fixes.

## Scope

In scope: the `orcblockd` helper (it runs as root), its socket and state file, the OrcBlock app and the `orcblock` CLI. A way for another local user or process to make the helper do something beyond starting blocks and reporting status is a vulnerability.

Ways for the Mac's own administrator to end a block early (unloading the LaunchDaemon, editing pf by hand) are known and documented in `docs/decisions/0002-enforcement-daemon.md`. Report them as regular issues if you find one that is easy, not as security reports.
