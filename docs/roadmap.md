# Roadmap

What exists and what is next, in order. `/status` reads this file. Big items get a spec under docs/specs before work starts.

## 1. Agentic workflow
- [x] AGENTS.md, CLAUDE.md, Claude Code settings, hooks (agenthooks crate), subagents and skills
- [x] CI, nightly advisories, release workflow, the @claude bot, dependabot, templates
- [x] Licence (AGPL-3.0), code of conduct, contributing and security policies, ADRs

## 2. Rust rewrite, macOS (spec 0001)
- [x] orcblock-core: domains, blocklist, sessions, potion ledger, state machine
- [x] orcblock-ipc: wire protocol and client
- [x] orcblockd: store, service, tick loop, socket server, dry-run enforcer, dev mode
- [x] orcblockd: DNS sinkhole
- [x] orcblockd: macOS enforcer (resolver files, pf anchor, cache flush) and install/uninstall
- [x] orcblock CLI
- [x] Tauri app: helper setup, sites list with favicons, durations, countdown, potions, themes
- [x] Orc mascot: placeholder line mark for sleeping and guarding
- [ ] Bundle: .app and .dmg with the daemon as a sidecar

## 3. Polish
- [ ] Final orc mark and app icon (see docs/mascot.md)
- [ ] Menu bar item showing the time left
- [ ] Notifications when a block starts, has 5 minutes left and ends
- [ ] Extend a running block (+15 min, +1 h)
- [ ] Schedules (block every weekday 9:00-12:00)

## 4. Distribution
- [ ] Developer ID signing and notarization
- [ ] Homebrew cask (`brew install --cask orcblock`)
- [ ] Move the helper install to `SMAppService` once the app is signed
- [ ] Auto-update

## 5. Later
- [ ] Block apps as well as websites: the daemon watches launches during a block and terminates blocked bundle IDs (NSWorkspace notifications or Endpoint Security). Needs a spec and probably signing.
- [ ] Network Extension content filter as the enforcement layer once signed (ADR 0002)
- [ ] Windows and Linux daemons behind the same `Enforcer` trait
