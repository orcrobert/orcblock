# Contributing to OrcBlock

## Setup

```sh
git clone https://github.com/orcrobert/orcblock && cd orcblock
make tools hooks
npm ci --prefix app
make check
```

You need macOS, Rust (rustup picks the pinned toolchain from `rust-toolchain.toml`), Node 25 and the Xcode command line tools.

## Running it

`make daemon-dev` runs the helper without root and without touching your system (a dry-run enforcer logs what it would do). `make dev` runs the app against it. Details: `.claude/skills/run-orcblock/SKILL.md`.

## How changes land

- Every change reaches `main` through a pull request made of small [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). The `commit-msg` hook installed by `make hooks` checks the format.
- `make check` is the single gate. CI runs the same target.
- Features start as a spec in `docs/specs/` (see `docs/specs/README.md`). Bugs start with a failing test.
- New dependencies, Rust or npm, need an ADR in `docs/decisions/` (see 0001).
- Anything that touches the helper must not add a way to end a block early; the PR template asks about it.

## Coding agents

This repo is set up for coding agents. `AGENTS.md` holds the rules every agent and human follows; Claude Code also loads `.claude/` (skills, subagents and hooks). Agent-written changes follow the same review bar as any other.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
