# 0002: Enforce blocks from a root daemon with a DNS sinkhole

Status: accepted (2026-10-17)

## Context

The original OrcBlock was a Python script that appended lines to `/etc/hosts` while it ran and removed them on Ctrl+C. Closing the terminal ended the block, editing the file ended the block, and the script had to run as root the whole time. The rewrite must keep a block in place for the time the user chose even when the app is closed, the user logs out or the Mac reboots, with exactly one sanctioned way out: a sleep potion, five per month.

Options considered on macOS:

1. **/etc/hosts** (the old way, SelfControl's first layer). Simple, works in every browser that uses the system resolver. One file shared with the user and other tools, no subdomain wildcards, and the user asked to move away from it.
2. **pf packet filter with IP tables.** Survives app exit, but blocking by IP over-blocks badly: YouTube shares front-end IPs with Google Search, Reddit with everything on Fastly.
3. **Network Extension (content filter or DNS proxy).** The most precise and the hardest to bypass, but it has to be a signed system extension written against Apple frameworks, needs the Network Extension entitlement from a paid Developer ID account, and user approval in System Settings. Not possible for an unsigned, Homebrew-distributed open-source build today.
4. **A root LaunchDaemon that owns the block**, enforcing through per-domain resolver files (`/etc/resolver/<domain>`) that point at its own DNS sinkhole, with pf only for things IP rules do well.

## Decision

Option 4, with option 3 kept as the long-term upgrade path (see docs/roadmap.md).

- `orcblockd` runs as a LaunchDaemon (`/Library/LaunchDaemons/com.orcrobert.orcblockd.plist`, `RunAtLoad`, `KeepAlive`). It owns all state in `/Library/Application Support/OrcBlock/state.json` (root-owned, world-readable) and serves the app and CLI over a Unix socket at `/var/run/orcblockd.sock`.
- Blocking is DNS-based: one marked resolver file per blocked domain sends lookups for the domain and all its subdomains to a sinkhole on 127.0.0.1:53535 that answers `0.0.0.0`/`::`. No collateral blocking, wildcard subdomains for free, and the files are ours alone (unlike /etc/hosts).
- Bypass hardening while a block runs: Private Relay's `mask.icloud.com` and `mask-h2.icloud.com` are sinkholed (Apple's documented network opt-out), and a pf anchor under `com.apple/orcblock` drops DNS-over-TLS and HTTPS to well-known public DoH resolvers so browsers fall back to the system resolver. At block start, connections to the blocked sites' current IPs are killed so open tabs cannot keep streaming.
- The daemon re-applies enforcement every 15 s and at boot, so deleting a resolver file or flushing pf only works for seconds.
- Time is measured with a monotonic clock that counts sleep; changing the wall clock does not shorten a block. Restart gaps are credited from the wall clock but clamped, and never negative.
- Potions are counted per calendar month in the system time zone; a month that goes backwards never refills.
- Uninstalling through `orcblockd uninstall` is refused while a block is active.

## Consequences

- One admin password prompt at first launch to install the helper; after that the app talks to the daemon without privileges.
- An administrator who unloads the LaunchDaemon with `launchctl bootout`, deletes it, and edits pf by hand can still escape. OrcBlock is a commitment device, not parental controls; making that path long and deliberate is the goal.
- Browsers with their own DNS that are not covered by the DoH list can bypass the DNS layer until their resolver is added. Firefox honours the `use-application-dns.net` canary, which the sinkhole answers with NXDOMAIN while a block runs.
- Moving to a Network Extension later replaces only the `enforce` module; core, protocol and UI stay.
