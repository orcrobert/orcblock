//! Enforcement on Windows: Name Resolution Policy Table rules route each
//! blocked domain and its subdomains to the sinkhole, firewall rules block
//! DNS over TLS and HTTPS, and the DNS cache is flushed after changes.
//!
//! PowerShell is the only interface to both that prints language-neutral
//! output, so it runs two constant scripts: [`STATUS_SCRIPT`] reports the
//! rules this module owns, and [`APPLY_SCRIPT`] carries out [`Op`]s read
//! one per line from stdin, checking each again before acting. No script
//! text is ever built from data. Scripts go in as `-EncodedCommand`, because
//! `powershell.exe` re-parses a `-Command` argument and mangles its quotes.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::net::IpAddr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use orcblock_core::Domain;
use tracing::info;

use super::runner::CommandRunner;
use super::{EnforceError, Enforcer, doh};
use crate::dns::CANARIES;

const POWERSHELL: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";

/// Where NRPT rules send lookups. NRPT has no port, so the sinkhole listens
/// on 53 on Windows.
pub(crate) const SINKHOLE: &str = "127.0.0.1";

/// The firewall rules, all in group `orcblockd`: DNS over TLS everywhere,
/// HTTPS to the DNS-over-HTTPS resolvers, each over TCP and UDP (QUIC).
pub(crate) const FIREWALL_RULES: [&str; 4] = [
    "orcblockd: DNS over TLS (TCP)",
    "orcblockd: DNS over TLS (UDP)",
    "orcblockd: DNS over HTTPS (TCP)",
    "orcblockd: DNS over HTTPS (UDP)",
];

/// Prints one tab-separated line per NRPT rule commented
/// `managed by orcblockd` (`nrpt  name  namespaces  servers`) and per firewall rule in the group
/// (`fw  display name  enabled  action  direction`).
pub(crate) const STATUS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
foreach ($rule in @(Get-DnsClientNrptRule)) {
    if ($rule.Comment -eq 'managed by orcblockd') {
        "nrpt`t$($rule.Name)`t$($rule.Namespace -join ',')`t$($rule.NameServers -join ',')"
    }
}
foreach ($rule in @(Get-NetFirewallRule -Group 'orcblockd' -ErrorAction SilentlyContinue)) {
    "fw`t$($rule.DisplayName)`t$($rule.Enabled)`t$($rule.Action)`t$($rule.Direction)"
}
"#;

/// Carries out the operations on stdin, one per line, in order.
pub(crate) const APPLY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$ips = @()
foreach ($line in @($input)) {
    $op, $arg = "$line".Trim() -split ' ', 2
    switch -CaseSensitive ($op) {
        'nrpt-remove' {
            if ($arg -cnotmatch '^\{[0-9A-Fa-f-]{36}\}$') { throw "not an NRPT rule name: $arg" }
            Remove-DnsClientNrptRule -Name $arg -Force
        }
        'nrpt-add' {
            if ($arg -cnotmatch '^[a-z0-9-]+(\.[a-z0-9-]+)+$') { throw "not a domain: $arg" }
            Add-DnsClientNrptRule -Namespace ".$arg", $arg -NameServers '127.0.0.1' -Comment 'managed by orcblockd' | Out-Null
        }
        'fw-clear' {
            Remove-NetFirewallRule -Group 'orcblockd' -ErrorAction SilentlyContinue
        }
        'fw-ip' {
            $ips += [System.Net.IPAddress]::Parse($arg).ToString()
        }
        'fw-add' {
            foreach ($protocol in 'TCP', 'UDP') {
                New-NetFirewallRule -DisplayName "orcblockd: DNS over TLS ($protocol)" -Group 'orcblockd' -Direction Outbound -Action Block -Protocol $protocol -RemotePort 853 | Out-Null
                New-NetFirewallRule -DisplayName "orcblockd: DNS over HTTPS ($protocol)" -Group 'orcblockd' -Direction Outbound -Action Block -Protocol $protocol -RemotePort 443 -RemoteAddress $ips | Out-Null
            }
        }
        'flush' {
            Clear-DnsClientCache
        }
        default { throw "unknown operation: $op" }
    }
}
"#;

/// An NRPT rule this module owns, as [`STATUS_SCRIPT`] reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NrptRule {
    pub(crate) name: String,
    pub(crate) namespaces: BTreeSet<String>,
    pub(crate) servers: String,
}

impl NrptRule {
    /// The domain this rule correctly blocks, if it is well formed.
    fn domain(&self) -> Option<&str> {
        let bare = self.namespaces.iter().find(|ns| !ns.starts_with('.'))?;
        let wanted: BTreeSet<String> = [format!(".{bare}"), bare.clone()].into();
        (self.namespaces == wanted && self.servers == SINKHOLE).then_some(bare.as_str())
    }
}

/// A firewall rule in group `orcblockd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FirewallRule {
    pub(crate) name: String,
    /// Enabled, blocking and outbound.
    pub(crate) blocks: bool,
}

/// What [`STATUS_SCRIPT`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) nrpt: Vec<NrptRule>,
    pub(crate) firewall: Vec<FirewallRule>,
}

/// Parses [`STATUS_SCRIPT`]'s output; lines it does not recognise are
/// skipped.
pub(crate) fn parse_status(stdout: &str) -> Observed {
    let mut observed = Observed::default();
    for line in stdout.lines() {
        let fields: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
        match fields.as_slice() {
            ["nrpt", name, namespaces, servers] => observed.nrpt.push(NrptRule {
                name: (*name).to_owned(),
                namespaces: namespaces
                    .split(',')
                    .filter(|ns| !ns.is_empty())
                    .map(str::to_ascii_lowercase)
                    .collect(),
                servers: (*servers).to_owned(),
            }),
            ["fw", name, enabled, action, direction] => observed.firewall.push(FirewallRule {
                name: (*name).to_owned(),
                blocks: *enabled == "True" && *action == "Block" && *direction == "Outbound",
            }),
            _ => {}
        }
    }
    observed
}

/// One step for [`APPLY_SCRIPT`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Op {
    /// Remove the NRPT rule with this name.
    RemoveNrpt(String),
    /// Route this domain and its subdomains to the sinkhole.
    AddNrpt(String),
    /// Remove every firewall rule in the group.
    ClearFirewall,
    /// A DNS-over-HTTPS resolver for the next [`Op::AddFirewall`].
    FirewallIp(IpAddr),
    /// Create the firewall rules.
    AddFirewall,
    /// Clear the DNS client cache.
    Flush,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RemoveNrpt(name) => write!(f, "nrpt-remove {name}"),
            Self::AddNrpt(domain) => write!(f, "nrpt-add {domain}"),
            Self::ClearFirewall => f.write_str("fw-clear"),
            Self::FirewallIp(ip) => write!(f, "fw-ip {ip}"),
            Self::AddFirewall => f.write_str("fw-add"),
            Self::Flush => f.write_str("flush"),
        }
    }
}

/// The steps that make `observed` route exactly `wanted` to the sinkhole
/// and hold the firewall rules if `firewall`. Empty when nothing drifted.
pub(crate) fn plan(observed: &Observed, wanted: &BTreeSet<String>, firewall: bool) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut kept = BTreeSet::new();
    for rule in &observed.nrpt {
        match rule.domain() {
            Some(domain) if wanted.contains(domain) && kept.insert(domain.to_owned()) => {}
            _ => ops.push(Op::RemoveNrpt(rule.name.clone())),
        }
    }
    ops.extend(
        wanted
            .difference(&kept)
            .map(|domain| Op::AddNrpt(domain.clone())),
    );
    let nrpt_changed = !ops.is_empty();

    let names: BTreeSet<&str> = observed.firewall.iter().map(|r| r.name.as_str()).collect();
    let firewall_ok = if firewall {
        observed.firewall.len() == FIREWALL_RULES.len()
            && names == FIREWALL_RULES.into_iter().collect()
            && observed.firewall.iter().all(|rule| rule.blocks)
    } else {
        observed.firewall.is_empty()
    };
    if !firewall_ok {
        ops.push(Op::ClearFirewall);
        if firewall {
            ops.extend(
                doh::RESOLVERS
                    .iter()
                    .filter_map(|ip| ip.parse().ok())
                    .map(Op::FirewallIp),
            );
            ops.push(Op::AddFirewall);
        }
    }
    if nrpt_changed {
        ops.push(Op::Flush);
    }
    ops
}

/// Enforces blocks on Windows through NRPT rules and the firewall.
#[derive(Debug)]
pub struct WinEnforcer<R> {
    runner: R,
}

impl<R: CommandRunner> WinEnforcer<R> {
    /// An enforcer that runs PowerShell through `runner`.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn powershell(&self, script: &str, stdin: Option<&str>) -> Result<String, EnforceError> {
        self.runner
            .run(
                POWERSHELL,
                &[
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-EncodedCommand",
                    &encode_command(script),
                ],
                stdin,
            )
            .map(|output| output.stdout)
    }

    /// Brings the system in line and returns whether anything changed.
    fn sync(&self, wanted: &BTreeSet<String>, firewall: bool) -> Result<bool, EnforceError> {
        let observed = parse_status(&self.powershell(STATUS_SCRIPT, None)?);
        let ops = plan(&observed, wanted, firewall);
        if ops.is_empty() {
            return Ok(false);
        }
        let input = ops.iter().fold(String::new(), |mut input, op| {
            // Writing to a String cannot fail.
            let _ = writeln!(input, "{op}");
            input
        });
        self.powershell(APPLY_SCRIPT, Some(&input))?;
        info!(steps = ops.len(), "NRPT and firewall rules updated");
        Ok(true)
    }
}

/// A script as `-EncodedCommand` expects it: base64 of UTF-16LE.
pub(crate) fn encode_command(script: &str) -> String {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    BASE64.encode(utf16)
}

fn wanted(domains: &BTreeSet<Domain>) -> BTreeSet<String> {
    domains
        .iter()
        .map(|domain| domain.as_ascii().to_owned())
        .chain(CANARIES.iter().map(|&name| name.to_owned()))
        .collect()
}

impl<R: CommandRunner> Enforcer for WinEnforcer<R> {
    fn apply(&mut self, domains: &BTreeSet<Domain>) -> Result<(), EnforceError> {
        self.sync(&wanted(domains), true).map(drop)
    }

    fn verify(&mut self, domains: &BTreeSet<Domain>) -> Result<bool, EnforceError> {
        self.sync(&wanted(domains), true)
    }

    fn clear(&mut self) -> Result<(), EnforceError> {
        self.sync(&BTreeSet::new(), false).map(drop)
    }
}

#[cfg(test)]
mod tests;
