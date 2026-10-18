//! The DNS-over-HTTPS resolvers blocked while a block runs.

/// Well-known public DNS-over-HTTPS resolvers. HTTPS to these is blocked
/// while a block runs (pf on macOS, the firewall on Windows), so browsers
/// fall back to the system resolver.
pub(crate) const RESOLVERS: [&str; 22] = [
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

#[cfg(test)]
mod tests;
