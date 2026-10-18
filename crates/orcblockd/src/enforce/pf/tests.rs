use super::*;

#[test]
fn rules_name_the_table_and_ports() {
    let rules = rules();
    assert!(rules.starts_with("table <orcblock_doh> const { 8.8.8.8, 8.8.4.4,"));
    assert!(rules.contains("to any port 853"));
    assert!(rules.contains("to <orcblock_doh> port 443"));
    assert_eq!(rules.lines().count(), 3);
}

#[test]
fn parses_the_enable_token() {
    let stderr = "No ALTQ support in kernel\npf enabled\nToken : 13905671538935542863\n";
    assert_eq!(parse_token(stderr).as_deref(), Some("13905671538935542863"));
    assert_eq!(parse_token("pf already enabled\n"), None);
    assert_eq!(parse_token("Token : -rf\n"), None);
}

#[test]
fn reads_pf_status() {
    assert!(is_enabled(
        "Status: Enabled for 0 days 00:01:02           Debug: Urgent\n"
    ));
    assert!(!is_enabled("Status: Disabled for 0 days 00:00:00\n"));
}
