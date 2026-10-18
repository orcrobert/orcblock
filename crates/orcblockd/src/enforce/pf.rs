//! The pf anchor that stops browsers from resolving around the sinkhole.

use super::doh;

/// The anchor, under `com.apple/` because macOS's stock `/etc/pf.conf`
/// already evaluates `anchor "com.apple/*"`.
pub(crate) const ANCHOR: &str = "com.apple/orcblock";

/// The anchor's rules: drop DNS over TLS and QUIC (port 853) everywhere,
/// and HTTPS to the resolvers in [`doh::RESOLVERS`].
pub(crate) fn rules() -> String {
    format!(
        "table <orcblock_doh> const {{ {} }}\n\
         block drop out quick proto {{ tcp, udp }} from any to any port 853\n\
         block drop out quick proto {{ tcp, udp }} from any to <orcblock_doh> port 443\n",
        doh::RESOLVERS.join(", ")
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
