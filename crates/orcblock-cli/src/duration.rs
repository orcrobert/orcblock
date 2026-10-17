use std::time::Duration;

/// A duration typed by a person, such as `25m`, `2h`, `1h30m`, `1d` or a
/// bare number of minutes.
pub(crate) fn parse(input: &str) -> Result<Duration, String> {
    let input = input.trim().to_ascii_lowercase();
    if input.is_empty() {
        return Err("give a duration like 25m, 2h or 1h30m".to_owned());
    }
    if let Ok(minutes) = input.parse::<u64>() {
        return Ok(Duration::from_secs(minutes.saturating_mul(60)));
    }
    let mut total: u64 = 0;
    let mut digits = String::new();
    for c in input.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let unit = match c {
            'd' => 86_400,
            'h' => 3_600,
            'm' => 60,
            's' => 1,
            _ => return Err(format!("{input:?} is not a duration; try 25m, 2h or 1h30m")),
        };
        let value: u64 = digits
            .parse()
            .map_err(|_| format!("{input:?} is not a duration; put a number before {c:?}"))?;
        total = total.saturating_add(value.saturating_mul(unit));
        digits.clear();
    }
    if !digits.is_empty() {
        return Err(format!(
            "{input:?} is not a duration; end it with d, h, m or s"
        ));
    }
    Ok(Duration::from_secs(total))
}

#[cfg(test)]
mod tests;
