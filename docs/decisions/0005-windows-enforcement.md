# 0005: Enforce blocks on Windows with NRPT rules, the firewall and a SYSTEM task

Status: accepted (2026-10-18)

## Context

ADR 0002 puts a root daemon in charge of blocks and enforces them through per-domain resolver files pointing at a DNS sinkhole. Windows needs the same guarantees (a block holds with the app closed and across reboots) without new dependencies (ADR 0001) and without `unsafe`.

## Decision

- **Running the daemon**: a scheduled task `\OrcBlock\orcblockd` as SYSTEM, triggered at boot and every 5 minutes, with "do not start a new instance" while one runs. A real Windows service would need the service control protocol (`windows-service` or raw `windows-sys` calls); a task needs only `schtasks.exe` and an XML file, and the repetition doubles as a keep-alive.
- **Routing lookups**: Name Resolution Policy Table rules, the Windows equivalent of `/etc/resolver/<domain>`: one rule per domain with namespaces `.domain` and `domain` pointing at `127.0.0.1`. Chrome and Edge turn off their built-in resolver when NRPT rules exist, and Firefox uses the system resolver unless DoH is on, which the canary turns off. NRPT name servers have no port, so the sinkhole listens on `127.0.0.1:53` on Windows. Rejected: the hosts file (no subdomains, shared with the user, the thing the rewrite moved away from).
- **DoH and DoT**: Windows Firewall rules in group `orcblockd` play the part of the pf anchor: block TCP/UDP 853 and TCP/UDP 443 to the same resolver list.
- **Talking to the system**: the NRPT and firewall cmdlets through `powershell.exe`, because their `netsh`/registry equivalents print localized text or need a DNS client restart. PowerShell only ever runs two constant scripts; data goes in on stdin as one validated operation per line.
- **IPC**: TCP on `127.0.0.1:47811`. Rust's standard library has no `AF_UNIX` on Windows, and a named pipe that unprivileged clients can write to needs a custom security descriptor, which means `windows-sys` and `unsafe`. The socket on macOS is world-writable too, so the trust model is unchanged; the server closes a connection after the first line that is not a request, so browsers cannot smuggle commands to it.
- **Install**: the per-machine NSIS installer already runs elevated, so its hooks install the helper; the app's Setup screen does the same through a UAC prompt.

## Consequences

- One UAC prompt, during installation.
- A verify every 15 s starts PowerShell (about half a second of CPU). Acceptable for now; reading the NRPT registry keys directly is the optimization if it matters.
- Port 53 on 127.0.0.1 must be free; while a local DNS proxy or any other process holds it, the daemon keeps counting down and keeps its rules but retries the bind, and lookups go to whatever holds the port.
- When Group Policy defines any NRPT rule (DirectAccess, some VPNs), Windows ignores local NRPT rules, so blocking does not work on such machines. Detecting this and saying so is a follow-up.
- `base64` (already allowed for the app) enters the daemon's tree to encode `-EncodedCommand`.
- Existing connections are not killed at block start.
- An administrator can delete the task, the rules or turn the firewall off. As on macOS, the goal is a deliberate, slow path out, not prevention.
- A Network Extension equivalent (a WFP callout driver) stays out of reach without signing; this ADR covers unsigned builds.
