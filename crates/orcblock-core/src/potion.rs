use std::fmt;

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

/// Sleep potions granted each calendar month.
pub const POTIONS_PER_MONTH: u8 = 5;

/// A calendar month in the system's time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "MonthRepr", into = "MonthRepr")]
pub struct Month {
    year: i16,
    month: i8,
}

/// A year and month that do not form a valid [`Month`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{year}-{month:02} is not a valid month")]
pub struct MonthError {
    year: i16,
    month: i8,
}

impl Month {
    /// Builds a month; `month` is 1-12 and `year` within jiff's supported range.
    pub fn new(year: i16, month: i8) -> Result<Self, MonthError> {
        Date::new(year, month, 1)
            .map(Self::of)
            .map_err(|_| MonthError { year, month })
    }

    /// The month containing `date`.
    #[must_use]
    pub fn of(date: Date) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    /// The first day of this month.
    #[must_use]
    pub fn first_day(self) -> Date {
        Date::new(self.year, self.month, 1).unwrap_or(Date::MAX)
    }

    /// The month after this one, saturating at the last supported month.
    #[must_use]
    pub fn next(self) -> Self {
        self.first_day()
            .last_of_month()
            .tomorrow()
            .map_or(self, Self::of)
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

#[derive(Serialize, Deserialize)]
struct MonthRepr {
    year: i16,
    month: i8,
}

impl TryFrom<MonthRepr> for Month {
    type Error = MonthError;

    fn try_from(repr: MonthRepr) -> Result<Self, Self::Error> {
        Self::new(repr.year, repr.month)
    }
}

impl From<Month> for MonthRepr {
    fn from(month: Month) -> Self {
        Self {
            year: month.year,
            month: month.month,
        }
    }
}

/// How many sleep potions have been used in the latest month seen.
///
/// The month only moves forward: asking about an earlier month (a clock set
/// back) is answered as if it were the stored month, so setting the clock
/// back never refills potions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionLedger {
    month: Month,
    used: u8,
}

impl PotionLedger {
    /// A ledger with every potion available in `month`.
    #[must_use]
    pub fn new(month: Month) -> Self {
        Self { month, used: 0 }
    }

    /// Potions left in `now`.
    #[must_use]
    pub fn available(&self, now: Month) -> u8 {
        if now > self.month {
            POTIONS_PER_MONTH
        } else {
            POTIONS_PER_MONTH.saturating_sub(self.used)
        }
    }

    /// The month after the latest one seen, when the next refill happens.
    #[must_use]
    pub fn refill(&self, now: Month) -> Month {
        self.month.max(now).next()
    }

    /// Spends one potion in `now` and returns how many are left, or `None`
    /// when none are left (the ledger is unchanged).
    pub(crate) fn spend(&mut self, now: Month) -> Option<u8> {
        if now > self.month {
            *self = Self::new(now);
        }
        if self.used >= POTIONS_PER_MONTH {
            return None;
        }
        self.used += 1;
        Some(POTIONS_PER_MONTH - self.used)
    }
}

#[cfg(test)]
mod tests;
