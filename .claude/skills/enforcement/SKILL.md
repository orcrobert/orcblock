---
name: enforcement
description: How orcblockd blocks sites on macOS - the resolver files, the DNS sinkhole, the pf anchor, cache flushing, the periodic check and the rules that keep a block from ending early. Use when changing anything under crates/orcblockd/src/enforce or crates/orcblockd/src/dns, the install code, or the session timing.
paths: "crates/orcblockd/**"
---

Decision record: docs/decisions/0002-enforcement-daemon.md. The promise to keep: once a block starts, nothing short of its timer or a potion ends it, whether or not the app is running.

## Layers (all owned by `enforce::MacEnforcer`)
1. **Resolver files**: for each blocked domain, `/etc/resolver/<domain>` with `nameserver 127.0.0.1` and `port 53535`. mDNSResponder then sends every lookup for the domain and its subdomains to our sinkhole. Each file starts with the marker line `# managed by orcblockd`; only marked files are ever rewritten or removed.
2. **DNS sinkhole** (`dns::Sinkhole`): UDP and TCP on 127.0.0.1:53535. A/AAAA for blocked names answer `0.0.0.0` / `::` with a short TTL; HTTPS/SVCB answer empty NOERROR so browsers do not try ECH or QUIC hints; everything else gets NXDOMAIN. It never forwards.
3. **Private Relay and DoH**: while a block runs, `mask.icloud.com` and `mask-h2.icloud.com` are sinkholed too (Apple's documented opt-out), and the pf anchor drops DNS-over-TLS (tcp 853) and HTTPS to the well-known DoH resolver IPs in `enforce::doh::RESOLVERS`, so browsers fall back to the system resolver.
4. **pf anchor** `com.apple/orcblock`: loaded with `/sbin/pfctl -a com.apple/orcblock -f -`, pf enabled with `pfctl -E` and the returned token kept in state, released with `pfctl -X <token>` on clear. At block start the daemon also kills existing connections to the blocked sites' current IPs (`pfctl -k`), resolved through an upstream server, so open tabs drop their keep-alive connections.
5. **Cache flush** after every change: `/usr/bin/dscacheutil -flushcache` and `/usr/bin/killall -HUP mDNSResponder`.

## Lifecycle
- `apply(domains)` is idempotent: it writes what is missing, rewrites what differs, removes marked files that are no longer blocked, reloads the anchor.
- `verify()` runs every 15 s from the tick loop while a block is active; any drift triggers `apply`. On daemon start, if the state says a block is active, `apply` runs before the socket accepts requests.
- `clear()` runs only when core says the session is over (timer or potion).

## Time
- The session stores `remaining`. The tick loop subtracts the monotonic time elapsed since the last tick (`Clock::elapsed`, which counts sleep), never wall-clock differences, so changing the clock does not shorten a block.
- Across a daemon restart, the gap is `wall_now - last_persisted_wall`, clamped to `0..=MAX_RESTART_CREDIT`; a wall clock that moved backwards credits nothing. State is persisted every tick so the gap stays small.

## Rules
- Programs by absolute path through `CommandRunner`; never a shell. Arguments from a `Domain` only.
- Every write is atomic (temp file + fsync + rename in the same directory) and leaves files root-owned 0644.
- Errors while enforcing are logged and retried on the next tick; they never clear the session.
- Dev mode (`--dev`) uses `DryRunEnforcer`, which records calls; tests use it or a `FakeRunner`. Nothing in `make check` touches the system.
- Any change here gets a security-reviewer pass with the "no way out" checklist.

## Windows (`enforce::WinEnforcer`, ADR 0005)
- NRPT rules (namespaces `.domain` and `domain`, name server `127.0.0.1`, comment `managed by orcblockd`) send lookups to the sinkhole on `127.0.0.1:53`; only marked rules are ever changed.
- Four firewall rules in group `orcblockd` drop DNS-over-TLS (853) and HTTPS to `enforce::doh::RESOLVERS`.
- PowerShell gets only the constant status and apply scripts as `-EncodedCommand`; operations go on stdin and the script validates them again. `plan` is pure and tested; `sync` reads, plans and applies.
- Not yet: killing open connections at block start (spec 0003, out of scope).
