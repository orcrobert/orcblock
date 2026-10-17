use jiff::civil::date;
use orcblock_core::Domain;
use orcblock_ipc::{Block, Potions, Site};

use super::*;

fn idle() -> Status {
    Status {
        daemon_version: "0.1.0".into(),
        mode: Mode::Live,
        sites: vec![],
        block: None,
        potions: Potions {
            available: 5,
            per_month: 5,
            refill_on: date(2026, 11, 1),
        },
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
         No sites yet; add one with `orcblock add youtube.com`\n\
         Sleep potions: 5 of 5 left (more on 2026-11-01)\n"
    );

    let domain = Domain::parse("bücher.de").expect("valid");
    let mut blocking = idle();
    blocking.mode = Mode::DryRun;
    blocking.sites = vec![Site {
        display: domain.to_unicode(),
        domain: domain.clone(),
        added_at: Timestamp::UNIX_EPOCH,
    }];
    blocking.block = Some(Block {
        domains: vec![domain],
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
         Sites:\n  bücher.de\n\
         Sleep potions: 3 of 5 left (more on 2026-11-01)\n"
    );
}
