use super::*;

#[test]
fn answers_for_names_under_blocked_domains() {
    let blocked = Blocked::default();
    assert!(!blocked.is_active());
    assert!(!blocked.blocks_name("youtube.com"));

    blocked.replace(BTreeSet::from([
        Domain::parse("youtube.com").expect("valid")
    ]));
    let shared = blocked.clone();
    assert!(shared.is_active());
    assert!(shared.blocks_name("www.YouTube.com."));
    assert!(!shared.blocks_name("notyoutube.com"));

    blocked.replace(BTreeSet::new());
    assert!(!shared.is_active());
}
