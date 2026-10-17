use jiff::civil::date;

use super::*;

fn m(year: i16, month: i8) -> Month {
    Month::new(year, month).expect("valid month")
}

#[test]
fn month_validation_and_order() {
    assert!(Month::new(2026, 0).is_err());
    assert!(Month::new(2026, 13).is_err());
    assert!(m(2026, 12) < m(2027, 1));
    assert_eq!(Month::of(date(2026, 10, 17)), m(2026, 10));
    assert_eq!(m(2026, 12).next(), m(2027, 1));
    assert_eq!(m(2026, 2).first_day(), date(2026, 2, 1));
    assert_eq!(m(2026, 3).to_string(), "2026-03");
}

#[test]
fn five_potions_per_month() {
    let october = m(2026, 10);
    let mut ledger = PotionLedger::new(october);
    assert_eq!(ledger.available(october), POTIONS_PER_MONTH);
    for left in (0..POTIONS_PER_MONTH).rev() {
        assert_eq!(ledger.spend(october), Some(left));
    }
    assert_eq!(ledger.available(october), 0);
    assert_eq!(ledger.spend(october), None);
    assert_eq!(
        ledger.available(october),
        0,
        "a failed spend changes nothing"
    );
}

#[test]
fn new_month_refills() {
    let mut ledger = PotionLedger::new(m(2026, 10));
    ledger.spend(m(2026, 10));
    ledger.spend(m(2026, 10));
    assert_eq!(ledger.available(m(2026, 10)), 3);
    assert_eq!(ledger.available(m(2026, 11)), 5);
    assert_eq!(ledger.refill(m(2026, 10)), m(2026, 11));

    assert_eq!(ledger.spend(m(2027, 1)), Some(4));
    assert_eq!(ledger.available(m(2027, 1)), 4);
}

#[test]
fn clock_set_back_never_refills() {
    let mut ledger = PotionLedger::new(m(2026, 10));
    for _ in 0..5 {
        ledger.spend(m(2026, 10));
    }
    assert_eq!(ledger.available(m(2026, 9)), 0);
    assert_eq!(ledger.spend(m(2026, 9)), None);
    assert_eq!(ledger.refill(m(2026, 9)), m(2026, 11));
}

#[test]
fn serde_round_trips_and_rejects_bad_months() {
    let ledger = PotionLedger::new(m(2026, 10));
    let json = serde_json::to_string(&ledger).expect("serializes");
    assert_eq!(json, r#"{"month":{"year":2026,"month":10},"used":0}"#);
    assert_eq!(
        serde_json::from_str::<PotionLedger>(&json).expect("parses"),
        ledger
    );
    assert!(
        serde_json::from_str::<PotionLedger>(r#"{"month":{"year":2026,"month":13},"used":0}"#)
            .is_err()
    );
}
