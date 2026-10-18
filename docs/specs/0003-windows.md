# 0003: Windows support

Status: draft (maintainer's goal, 2026-10-18: "get this thing working on windows too")

## Problem
OrcBlock only runs on macOS. The core, the protocol, the CLI and the UI are portable, but the daemon talks to launchd, `/etc/resolver`, pf and a Unix socket, and the app installs its helper through an AppleScript password prompt. Windows users cannot use it at all.

## Behaviour
1. The user downloads `OrcBlock-setup.exe` and runs it. Windows asks once for administrator rights (UAC). The installer puts the app in `C:\Program Files\OrcBlock` and installs and starts the helper, so the first launch goes straight to Home.
2. If the helper is missing or outdated (installed by hand, or removed), the app shows the same Setup screen as on macOS; **Install helper** shows the UAC prompt instead of a password prompt. Cancelling leaves the app on Setup with "Installation cancelled".
3. Everything else behaves as on macOS: sites, durations, blocking, potions, themes. Blocks survive closing the app, signing out and rebooting.
4. While a block runs, the blocked sites and all their subdomains do not resolve in Edge, Chrome, Firefox or any program that uses the Windows resolver; browsers cannot route around it through the well-known DNS-over-HTTPS resolvers.
5. Uninstalling OrcBlock from Settings removes the helper too, except during a block: then the helper stays until it is uninstalled again after the block.
6. The window has the normal Windows title bar; the top bar does not reserve room for macOS traffic lights.
7. The `orcblock` CLI works against the Windows helper.

## Acceptance criteria
- [ ] Every crate compiles for `x86_64-pc-windows-msvc` with clippy clean, and `cargo test --workspace` passes on Windows in CI. (Every crate but the app is clippy clean cross-compiled from macOS; the app and the tests wait for the first Windows CI run.)
- [x] On Windows the daemon listens on `127.0.0.1:47811` (dev: `127.0.0.1:47812`); the client, CLI and app connect there by default; `--socket`/`ORCBLOCK_SOCKET` take an address.
- [x] The server drops a connection after the first undecodable line, so an HTTP request from a web page to the port never reaches a command.
- [x] `WinEnforcer` keeps one NRPT rule per blocked domain and canary (namespaces `.d` and `d`, name server `127.0.0.1`, comment `managed by orcblockd`) and four firewall rules in group `orcblockd` (TCP and UDP to port 853; TCP and UDP 443 to the DoH resolvers); it only removes rules it marked; verify repairs drift; clear removes everything; the DNS cache is flushed after changes. Tested with a fake runner.
- [x] The sinkhole listens on `127.0.0.1:53` on Windows (NRPT name servers have no port).
- [x] `orcblockd install` on Windows refuses without elevation, copies itself to `C:\Program Files\OrcBlock Helper\orcblockd.exe`, locks `C:\ProgramData\OrcBlock` to SYSTEM and Administrators (users read only), registers the `OrcBlock\orcblockd` scheduled task (SYSTEM, at boot, every 5 minutes if not running, no time limit) and starts it. `uninstall` is refused during a block. Tested with a fake runner under a temp root.
- [x] The monotonic clock on Windows comes from `Instant` (QueryPerformanceCounter).
- [x] The daemon logs to `C:\ProgramData\OrcBlock\orcblockd.log` on Windows.
- [x] The app finds `orcblockd.exe` next to itself, checks its pinned SHA-256 and runs `install` through UAC, mapping a cancelled prompt to "Installation cancelled".
- [x] The UI drops the traffic-light padding and uses Windows wording and log path on Windows; Segoe UI and Consolas are in the font stacks.
- [x] `scripts/bundle-windows.sh` builds a per-machine NSIS installer with the helper as a sidecar; its hooks install the helper after install and uninstall it before uninstall.
- [x] CI runs a Windows job (clippy, tests, bundle); a release publishes the Windows installer next to the dmg.

## Design
See ADR 0005 for the choices.

- **orcblock-ipc**: `Endpoint` is `PathBuf` on Unix and `SocketAddr` on Windows; `Client::send` connects with `UnixStream` or `TcpStream::connect_timeout`. `DEFAULT_SOCKET` is the platform's default as a string.
- **orcblockd**:
  - `server`: `Listener` is `UnixListener` or `TcpListener`; connections are generic over the stream. Close after a decode error (both platforms).
  - `clock`: `SystemClock::new()`; Unix keeps `CLOCK_MONOTONIC` through nix (now a Unix-only dependency), Windows uses `Instant` against an origin taken at construction.
  - `fsutil`, `resolver`, `install`: file modes and directory syncs only on Unix.
  - `layout`: a `Layout` knows its platform; `Layout::windows_under(root)` gives the Windows paths under a temp root in tests.
  - `enforce::windows`: `WinEnforcer<R: CommandRunner>`. One fixed PowerShell script reports the NRPT and firewall rules (tab-separated lines); Rust compares them with what should be there and sends a list of operations (`nrpt-remove {guid}`, `nrpt-add domain`, `fw-reset`, `fw-ip addr`, `flush`) on stdin to a second fixed script, which validates each line before acting. No script text is ever built from data.
  - `enforce::doh`: the DoH resolver list, shared by pf and the firewall.
  - `install::windows`: task XML (UTF-16), `schtasks`, `icacls`, elevation check through `whoami /groups` (mandatory label SIDs, language independent).
  - `runner`: on Windows keeps a fixed set of system variables instead of an empty environment (PowerShell needs `SystemRoot`) and starts children without a console window.
  - `main`: Ctrl+C only on Windows (the task scheduler terminates the process; state is saved on every change and every 15 s).
- **orcblock-app**: helper file name with `EXE_SUFFIX`; Windows install through PowerShell `Start-Process -Verb RunAs` with the helper path in an environment variable.
- **UI**: `lib/platform.ts` sets `data-platform` on `<html>`; TopBar and Setup read it.
- **Bundling**: `tauri.windows.conf.json` (NSIS, per machine, hooks, `.ico`), `windows/hooks.nsh`, `scripts/bundle-windows.sh`, `make bundle-windows`.
- **deny.toml** checks the Windows target too.

Files on a Windows PC:

| Path | Owner | What |
| --- | --- | --- |
| `C:\Program Files\OrcBlock\` | Administrators | The app, with `orcblockd.exe` next to it. |
| `C:\Program Files\OrcBlock Helper\orcblockd.exe` | Administrators | Installed daemon. |
| Task Scheduler `\OrcBlock\orcblockd` | SYSTEM | Starts the daemon at boot and every 5 minutes if it is not running. |
| `C:\ProgramData\OrcBlock\` | SYSTEM, Administrators; Users read | `state.json`, `orcblockd.log`, `orcblockd-task.xml`. |
| NRPT rules, comment `managed by orcblockd` | SYSTEM | Only during a block. |
| Firewall rules, group `orcblockd` | SYSTEM | Only during a block. |

## Bypass and safety
- **Local TCP instead of a socket file**: any local process can connect, as with the 0666 socket on macOS; the only request that ends a block costs a potion. A web page can make the browser connect to 127.0.0.1 but its first line is an HTTP request line, which never decodes, and the connection is closed on the first undecodable line.
- **Port squatting**: a user process that binds 47811 before the daemon makes the daemon exit and the task retry; enforcement (NRPT and firewall rules) stays in place, so the block holds and is only stricter. Binding 53 first would stop the sinkhole answering; lookups for blocked names then fail, which also blocks.
- **State and helper tampering**: the helper lives under Program Files and the state under a ProgramData folder whose ACL `install` resets (owner SYSTEM, inheritance removed, Users read only), so a folder pre-created by a user does not stay theirs.
- **Clock**: QueryPerformanceCounter includes sleep on current Windows. If it did not, blocks would last longer, never shorter. Restart credit is the same clamped wall-clock credit as on macOS.
- **Commands**: every program is called by absolute path under `C:\Windows\System32`; PowerShell only runs two constant scripts; domains reach it on stdin, already parsed as `Domain`, and are validated again by the script.
- **Uninstall** is refused during a block, as on macOS. An administrator can delete the task and the rules by hand; as on macOS, OrcBlock is a commitment device, not parental control.

## Slices
1. ipc and daemon portability: `Endpoint`, TCP listener, close on decode errors, clock, modes, layout platform. Compiles for Windows.
2. `WinEnforcer` with its scripts and tests.
3. Windows install and uninstall, elevation check, logging, `main` wiring.
4. App: helper install through UAC, endpoint parsing; UI platform tweaks.
5. Bundling, CI Windows job, release, deny targets, docs.

## Test plan
Pure logic (status parsing, operation planning, task XML, elevation parsing, Windows layout) is unit-tested on every platform with fakes. CI runs the whole workspace's tests and clippy on `windows-latest` and builds the installer. Manual: install on Windows 11, add youtube.com, start a 5 minute block, check Edge, Chrome and Firefox, `Resolve-DnsName youtube.com`, reboot mid-block, drink a potion, uninstall during and after a block.

## Out of scope
- Killing open connections at block start (macOS does it with `pfctl -k`; Windows has no equivalent without new dependencies). Open tabs keep their existing connections until they reconnect.
- Windows on ARM builds (the x64 build runs under emulation).
- Code signing (SmartScreen will warn, as Gatekeeper does for the ad hoc dmg).
- Linux.

## Open questions
- Whether the Task Scheduler's restart policy needs tuning after real-world crashes; the 5 minute repetition is the fallback.
