use super::*;

#[test]
fn parses_human_durations() {
    let cases = [
        ("25", 25 * 60),
        ("25m", 25 * 60),
        ("2h", 7200),
        ("1h30m", 5400),
        ("1D", 86_400),
        (" 90s ", 90),
        ("1d2h3m4s", 93_784),
    ];
    for (input, secs) in cases {
        assert_eq!(parse(input), Ok(Duration::from_secs(secs)), "{input}");
    }
}

#[test]
fn rejects_nonsense() {
    for input in ["", "h", "1x", "1h30", "-5m", "1.5h"] {
        assert!(parse(input).is_err(), "{input}");
    }
}
