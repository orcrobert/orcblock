use std::net::IpAddr;

use super::*;

#[test]
fn resolvers_are_ip_addresses() {
    for ip in RESOLVERS {
        assert!(ip.parse::<IpAddr>().is_ok(), "{ip}");
    }
}
