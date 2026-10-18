# 0003: Windows support

Status: implemented, awaiting the first Windows CI run (maintainer's goal, 2026-10-18: "get this thing working on windows too")

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
- **Port squatting**: any process may bind loopback ports below 1024 on Windows. One that holds 127.0.0.1:53 while the daemon restarts (an update, a crash) and answers truthfully unblocks lookups for as long as it runs; the daemon keeps counting the block down, keeps the rules in place and retries the bind every 5 seconds, so the block resumes when the port frees. The task starts at boot, before anyone logs in, which makes the window small. One that holds 47811 makes the app and CLI talk to it instead; it can lie to them but cannot end a block. Binding exclusively (`SO_EXCLUSIVEADDRUSE`) needs a dependency and is a follow-up.
- **State and helper tampering**: the helper lives under Program Files and the state under a ProgramData folder that `install` locks (owner SYSTEM, inheritance removed, Users read only, not recursive). Any user can create that folder first, so `install` keeps an existing one only if it is a real directory whose SDDL shows it locked; otherwise it moves the folder aside and makes a new one, and fails if files appear in the new one before it is locked. Only the installed daemon writes the log there.
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

## Follow-ups

Found in review and left for later; none lets a standard user end a block.

- A corrupt state file starts the daemon with no block (existing code on both platforms); it should keep the rules and refuse requests until fixed.
- If the data directory stops looking locked (an administrator changed it, or `Get-Acl` fails), the installed daemon exits on every start and nothing ticks or listens on port 53 while the NRPT rules stay. It should keep enforcing without saving and report the problem in `Status`.
- NRPT: a more specific rule without our comment, or any Group Policy NRPT rule, overrides ours; `verify` should report both. Compare every relevant firewall rule field and the profile state too.
- Bind 127.0.0.1:53 and 47811 exclusively (`SO_EXCLUSIVEADDRUSE`; needs a dependency and an ADR).
- A `<SecurityDescriptor>` on the scheduled task.
- Rotate or cap `orcblockd.log`.
- The potion month comes from the local time zone, which a standard user can change by up to a day.
- The app should pass the helper's canonical path to the elevated run, in case a junction higher up is retargeted after the hash check (needs testing `\\?\` paths with `Start-Process`).
- A directory moved aside as `Tampered` stays in ProgramData, locked; harmless, but untidy.
- Kill open connections to blocked sites at block start, as pf does on macOS.
- Verify on a Windows VM: the NSIS hooks during an update, `share_mode` with `Start-Process`, and `Instant` across hibernation.
