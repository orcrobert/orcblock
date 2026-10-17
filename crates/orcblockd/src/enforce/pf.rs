//! The pf anchor that stops browsers from resolving around the sinkhole.

/// The anchor, under `com.apple/` because macOS's stock `/etc/pf.conf`
/// already evaluates `anchor "com.apple/*"`.
pub(crate) const ANCHOR: &str = "com.apple/orcblock";

/// Well-known public DNS-over-HTTPS resolvers. HTTPS to these is dropped
/// while a block runs, so browsers fall back to the system resolver.
pub(crate) const DOH_RESOLVERS: [&str; 22] = [
    // Google
    "8.8.8.8",
    "8.8.4.4",
    "2001:4860:4860::8888",
    "2001:4860:4860::8844",
    // Cloudflare, including the family filters
    "1.1.1.1",
    "1.0.0.1",
    "1.1.1.2",
    "1.0.0.2",
    "1.1.1.3",
    "1.0.0.3",
    "2606:4700:4700::1111",
    "2606:4700:4700::1001",
    // Quad9
    "9.9.9.9",
    "149.112.112.112",
    "2620:fe::fe",
    "2620:fe::9",
    // OpenDNS
    "208.67.222.222",
    "208.67.220.220",
    // AdGuard
    "94.140.14.14",
    "94.140.15.15",
    // CleanBrowsing
    "185.228.168.9",
    "185.228.169.9",
];

/// The anchor's rules: drop DNS over TLS and QUIC (port 853) everywhere,
/// and HTTPS to the resolvers above.
pub(crate) fn rules() -> String {
    format!(
        "table <orcblock_doh> const {{ {} }}\n\
         block drop out quick proto {{ tcp, udp }} from any to any port 853\n\
         block drop out quick proto {{ tcp, udp }} from any to <orcblock_doh> port 443\n",
        DOH_RESOLVERS.join(", ")
    )
}

/// The reference token `pfctl -E` prints on stderr (`Token : 123`).
pub(crate) fn parse_token(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let value = value.trim();
        (key.trim() == "Token" && !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| value.to_owned())
    })
}

/// Whether `pfctl -s info` reports pf as enabled.
pub(crate) fn is_enabled(info: &str) -> bool {
    info.lines()
        .any(|line| line.trim_start().starts_with("Status: Enabled"))
}

#[cfg(test)]
mod tests;
