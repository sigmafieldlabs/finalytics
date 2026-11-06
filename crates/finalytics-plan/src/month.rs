//! Calendar months, the plan's unit of time.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use finalytics_core::{Date, Error, Result};

/// A calendar month, written as `"2030-06"`. A date such as `"2030-06-15"` reads as its month.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    /// Months since January of year 0.
    index: i32,
}

impl Month {
    /// The month `month` (1 to 12) of `year`.
    pub fn new(year: i32, month: u32) -> Result<Month> {
        if !(1..=12).contains(&month) {
            return Err(Error::InvalidDate(format!(
                "{month} is not a month from 1 to 12"
            )));
        }
        if !(1..=9999).contains(&year) {
            return Err(Error::InvalidDate(format!(
                "{year} is not a year from 1 to 9999"
            )));
        }
        Ok(Month {
            index: year * 12 + month as i32 - 1,
        })
    }

    /// The month `date` falls in.
    pub fn of(date: Date) -> Month {
        Month {
            index: date.year() * 12 + date.month() as i32 - 1,
        }
    }

    /// The year.
    pub fn year(self) -> i32 {
        self.index.div_euclid(12)
    }

    /// The month of the year, from 1 to 12.
    pub fn month(self) -> u32 {
        self.index.rem_euclid(12) as u32 + 1
    }

    /// The month `n` months later (earlier when `n` is negative).
    pub fn plus(self, n: i32) -> Month {
        Month {
            index: self.index + n,
        }
    }

    /// The number of months from this month to `later`: negative when `later` comes first.
    pub fn months_until(self, later: Month) -> i32 {
        later.index - self.index
    }

    /// The month's first day.
    pub fn first_day(self) -> Date {
        Date::from_ymd(self.year(), self.month(), 1).expect("a month's first day is a date")
    }

    /// The month's last day.
    pub fn last_day(self) -> Date {
        self.first_day().end_of_month()
    }

    /// The month's short name and year, such as "Jun 2030".
    pub fn short_name(self) -> String {
        format!("{} {}", &NAMES[self.month() as usize - 1][..3], self.year())
    }

    /// The month's full name and year, such as "June 2030".
    pub fn long_name(self) -> String {
        format!("{} {}", NAMES[self.month() as usize - 1], self.year())
    }
}

const NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year(), self.month())
    }
}

impl fmt::Debug for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Month({self})")
    }
}

impl FromStr for Month {
    type Err = Error;

    /// Parses `YYYY-MM`, or a date `YYYY-MM-DD` as its month.
    fn from_str(s: &str) -> Result<Month> {
        if s.len() == 10 {
            return s.parse::<Date>().map(Month::of);
        }
        let bytes = s.as_bytes();
        let well_formed = bytes.len() == 7
            && bytes[4] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || b.is_ascii_digit());
        if !well_formed {
            return Err(Error::InvalidDate(format!(
                "{s:?} is not a month in the form YYYY-MM"
            )));
        }
        let year: i32 = s[0..4].parse().expect("four digits");
        let month: u32 = s[5..7].parse().expect("two digits");
        Month::new(year, month)
    }
}

impl Serialize for Month {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Month {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Month, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Month {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Month".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "A calendar month, written as \"2030-06\"; a date such as \"2030-06-15\" reads as its month."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_read_write_and_count() {
        let m: Month = "2026-11".parse().unwrap();
        assert_eq!((m.year(), m.month()), (2026, 11));
        assert_eq!(m.to_string(), "2026-11");
        assert_eq!(m.plus(2).to_string(), "2027-01");
        assert_eq!(m.plus(-11).to_string(), "2025-12");
        assert_eq!(m.months_until("2030-06".parse().unwrap()), 43);
        assert_eq!(
            "2030-06-15".parse::<Month>().unwrap().to_string(),
            "2030-06"
        );
        assert_eq!(m.short_name(), "Nov 2026");
        assert_eq!(m.long_name(), "November 2026");
        assert_eq!(m.last_day().to_string(), "2026-11-30");
        assert!("2026-13".parse::<Month>().is_err());
        assert!("2026-1".parse::<Month>().is_err());
        assert!("0000-01".parse::<Month>().is_err());
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"2026-11\"");
        assert_eq!(serde_json::from_str::<Month>(&json).unwrap(), m);
    }
}
