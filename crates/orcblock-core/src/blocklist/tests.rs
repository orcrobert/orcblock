use jiff::Timestamp;

use super::*;

fn d(s: &str) -> Domain {
    Domain::parse(s).expect("valid domain")
}

fn names(list: &Blocklist) -> Vec<&str> {
    list.domains().map(Domain::as_ascii).collect()
}

#[test]
fn insert_keeps_order_and_rejects_covered() {
    let mut list = Blocklist::default();
    let now = Timestamp::UNIX_EPOCH;
    assert_eq!(list.insert(d("youtube.com"), now), Ok(vec![]));
    assert_eq!(list.insert(d("reddit.com"), now), Ok(vec![]));
    assert_eq!(names(&list), ["youtube.com", "reddit.com"]);

    assert_eq!(
        list.insert(d("youtube.com"), now),
        Err(Covered {
            by: d("youtube.com")
        })
    );
    assert_eq!(
        list.insert(d("m.youtube.com"), now),
        Err(Covered {
            by: d("youtube.com")
        })
    );
    assert_eq!(names(&list), ["youtube.com", "reddit.com"]);
}

#[test]
fn insert_parent_replaces_subdomains() {
    let mut list = Blocklist::default();
    let now = Timestamp::UNIX_EPOCH;
    list.insert(d("old.reddit.com"), now).expect("inserts");
    list.insert(d("x.com"), now).expect("inserts");
    list.insert(d("new.reddit.com"), now).expect("inserts");
    assert_eq!(
        list.insert(d("reddit.com"), now),
        Ok(vec![d("old.reddit.com"), d("new.reddit.com")])
    );
    assert_eq!(names(&list), ["x.com", "reddit.com"]);
}

#[test]
fn remove_needs_an_exact_match() {
    let mut list = Blocklist::default();
    list.insert(d("youtube.com"), Timestamp::UNIX_EPOCH)
        .expect("inserts");
    assert!(!list.remove(&d("m.youtube.com")));
    assert!(list.remove(&d("youtube.com")));
    assert!(list.is_empty());
    assert!(!list.remove(&d("youtube.com")));
}

#[test]
fn deserialize_rejects_overlapping_entries() {
    let ok = r#"[{"domain":"a.com","added_at":"2026-01-01T00:00:00Z"},{"domain":"b.com","added_at":"2026-01-01T00:00:00Z"}]"#;
    let list: Blocklist = serde_json::from_str(ok).expect("valid list");
    assert_eq!(names(&list), ["a.com", "b.com"]);

    let overlapping = r#"[{"domain":"a.com","added_at":"2026-01-01T00:00:00Z"},{"domain":"x.a.com","added_at":"2026-01-01T00:00:00Z"}]"#;
    let err = serde_json::from_str::<Blocklist>(overlapping).expect_err("overlap rejected");
    assert!(
        err.to_string().contains("lists both a.com and x.a.com"),
        "{err}"
    );
}
