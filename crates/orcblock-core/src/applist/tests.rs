use jiff::Timestamp;

use super::*;

fn entry(id: &str, name: &str) -> AppEntry {
    AppEntry {
        bundle_id: BundleId::parse(id).expect("valid id"),
        name: AppName::parse(name).expect("valid name"),
        added_at: Timestamp::UNIX_EPOCH,
    }
}

fn ids(list: &AppList) -> Vec<&str> {
    list.bundle_ids().map(BundleId::as_str).collect()
}

#[test]
fn insert_keeps_order_and_rejects_duplicates() {
    let mut list = AppList::default();
    assert!(list.insert(entry("com.tinyspeck.slackmacgap", "Slack")));
    assert!(list.insert(entry("com.hnc.Discord", "Discord")));
    assert!(!list.insert(entry("com.hnc.discord", "Discord again")));
    assert_eq!(ids(&list), ["com.tinyspeck.slackmacgap", "com.hnc.discord"]);
    let discord = BundleId::parse("com.hnc.discord").expect("valid id");
    assert_eq!(list.get(&discord).map(|e| e.name.as_str()), Some("Discord"));
}

#[test]
fn remove_reports_whether_it_was_listed() {
    let mut list = AppList::default();
    list.insert(entry("com.hnc.discord", "Discord"));
    let discord = BundleId::parse("com.hnc.discord").expect("valid id");
    assert!(list.remove(&discord));
    assert!(list.is_empty());
    assert!(!list.remove(&discord));
}

#[test]
fn deserialize_rejects_duplicates() {
    let ok = r#"[{"bundle_id":"com.a.app","name":"A","added_at":"2026-01-01T00:00:00Z"}]"#;
    let list: AppList = serde_json::from_str(ok).expect("valid list");
    assert_eq!(ids(&list), ["com.a.app"]);

    let twice = r#"[{"bundle_id":"com.a.app","name":"A","added_at":"2026-01-01T00:00:00Z"},{"bundle_id":"com.A.App","name":"A","added_at":"2026-01-01T00:00:00Z"}]"#;
    let err = serde_json::from_str::<AppList>(twice).expect_err("duplicate rejected");
    assert!(err.to_string().contains("lists com.a.app twice"), "{err}");
}
