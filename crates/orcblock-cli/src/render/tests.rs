use jiff::civil::date;
use orcblock_core::{AppName, BundleId, Domain, ListId, ListName};
use orcblock_ipc::{App, Block, BlockedLaunch, List, Potions, Site};

use super::*;

fn idle() -> Status {
    Status {
        daemon_version: "0.1.0".into(),
        mode: Mode::Live,
        lists: vec![List {
            id: ListId::new(1),
            name: ListName::parse("My list").expect("valid name"),
            sites: vec![],
            apps: vec![],
        }],
        block: None,
        potions: Potions {
            available: 5,
            per_month: 5,
            refill_on: date(2026, 11, 1),
        },
        blocked_launches: vec![],
    }
}

#[test]
fn remaining_is_compact() {
    assert_eq!(remaining(45), "45 s");
    assert_eq!(remaining(750), "12 min 30 s");
    assert_eq!(remaining(7200), "2 h");
    assert_eq!(remaining(7500), "2 h 5 min");
}

#[test]
fn renders_idle_and_blocking() {
    assert_eq!(
        status(&idle(), &TimeZone::UTC),
        "OrcBlock helper 0.1.0\n\
         The orc is asleep: nothing is blocked\n\
         Nothing listed yet; add a site with `orcblock add youtube.com` or an app with `orcblock add-app <bundle-id>`\n\
         Sleep potions: 5 of 5 left (more on 2026-11-01)\n"
    );

    let domain = Domain::parse("bücher.de").expect("valid");
    let mut blocking = idle();
    blocking.mode = Mode::DryRun;
    blocking.lists[0].sites = vec![Site {
        display: domain.to_unicode(),
        domain: domain.clone(),
        added_at: Timestamp::UNIX_EPOCH,
    }];
    blocking.block = Some(Block {
        lists: vec![ListId::new(1)],
        domains: vec![domain],
        apps: vec![],
        started_at: Timestamp::UNIX_EPOCH,
        duration_secs: 3600,
        remaining_secs: 1500,
        ends_at: "2026-10-17T13:00:00Z".parse().expect("timestamp"),
    });
    blocking.potions.available = 3;
    assert_eq!(
        status(&blocking, &TimeZone::UTC),
        "OrcBlock helper 0.1.0 (dry run: nothing is really blocked)\n\
         The orc is guarding: 25 min 0 s left, until Sat 13:00\n\
         My list (blocked):\n  bücher.de\n\
         Sleep potions: 3 of 5 left (more on 2026-11-01)\n"
    );
}

#[test]
fn renders_apps_and_closed_launches() {
    let slack = BundleId::parse("com.tinyspeck.slackmacgap").expect("valid id");
    let name = AppName::parse("Slack").expect("valid name");
    let mut listed = idle();
    listed.lists[0].apps = vec![App {
        bundle_id: slack.clone(),
        name: name.clone(),
        added_at: Timestamp::UNIX_EPOCH,
    }];
    listed.blocked_launches = vec![BlockedLaunch {
        id: 1,
        bundle_id: slack,
        name,
        at: "2026-10-17T12:41:00Z".parse().expect("timestamp"),
    }];
    assert_eq!(
        status(&listed, &TimeZone::UTC),
        "OrcBlock helper 0.1.0\n\
         The orc is asleep: nothing is blocked\n\
         My list:\n  Slack (com.tinyspeck.slackmacgap)\n\
         Closed by the orc: Slack at 12:41\n\
         Sleep potions: 5 of 5 left (more on 2026-11-01)\n"
    );
}

#[test]
fn renders_every_list_and_marks_blocked_ones() {
    let mut two = idle();
    two.lists.push(List {
        id: ListId::new(2),
        name: ListName::parse("Work").expect("valid name"),
        sites: vec![],
        apps: vec![],
    });
    two.block = Some(Block {
        lists: vec![ListId::new(2)],
        domains: vec![],
        apps: vec![],
        started_at: Timestamp::UNIX_EPOCH,
        duration_secs: 3600,
        remaining_secs: 3600,
        ends_at: "2026-10-17T13:00:00Z".parse().expect("timestamp"),
    });
    assert_eq!(
        status(&two, &TimeZone::UTC),
        "OrcBlock helper 0.1.0\n\
         The orc is guarding: 1 h left, until Sat 13:00\n\
         My list:\n  empty\n\
         Work (blocked):\n  empty\n\
         Sleep potions: 5 of 5 left (more on 2026-11-01)\n"
    );
}
