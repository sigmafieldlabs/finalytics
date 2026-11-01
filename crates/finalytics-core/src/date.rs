//! Civil dates.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Error, Result};

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// The position in the week, from 0 for Monday to 6 for Sunday.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A calendar date in the proleptic Gregorian calendar, from 0001-01-01 to 9999-12-31.
///
/// A `Date` is stored as the number of days since 1970-01-01, so comparisons and day arithmetic
/// are cheap. It is written and read as ISO 8601 (`2026-11-02`), including in serialised form.
///
/// ```
/// use finalytics_core::Date;
///
/// let date: Date = "2024-02-29".parse()?;
/// assert_eq!(date.add_days(1)?, Date::from_ymd(2024, 3, 1)?);
/// assert_eq!(date.add_months(12)?.to_string(), "2025-02-28");
/// # Ok::<(), finalytics_core::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date(i32);

const MIN_DAYS: i32 = -719_162; // 0001-01-01
const MAX_DAYS: i32 = 2_932_896; // 9999-12-31

impl Date {
    /// The earliest supported date, 0001-01-01.
    pub const MIN: Date = Date(MIN_DAYS);
    /// The latest supported date, 9999-12-31.
    pub const MAX: Date = Date(MAX_DAYS);
    /// 1970-01-01, day zero of the internal count.
    pub const EPOCH: Date = Date(0);

    /// Creates a date from its year, month (1 to 12) and day of the month.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Date> {
        if !(1..=9999).contains(&year) {
            return Err(Error::InvalidDate(format!(
                "year {year} is outside 1 to 9999"
            )));
        }
        if !(1..=12).contains(&month) {
            return Err(Error::InvalidDate(format!(
                "month {month} is outside 1 to 12"
            )));
        }
        let last = days_in_month(year, month);
        if day == 0 || day > last {
            return Err(Error::InvalidDate(format!(
                "{year:04}-{month:02} has {last} days, so day {day} does not exist"
            )));
        }
        Ok(Date(days_from_civil(year, month, day)))
    }

    /// Creates a date from the number of days since 1970-01-01.
    pub fn from_days_since_epoch(days: i32) -> Result<Date> {
        if (MIN_DAYS..=MAX_DAYS).contains(&days) {
            Ok(Date(days))
        } else {
            Err(Error::DateOutOfRange)
        }
    }

    /// The number of days since 1970-01-01 (negative before it).
    pub fn days_since_epoch(self) -> i32 {
        self.0
    }

    /// The year, month and day.
    pub fn ymd(self) -> (i32, u32, u32) {
        civil_from_days(self.0)
    }

    /// The year.
    pub fn year(self) -> i32 {
        self.ymd().0
    }

    /// The month, from 1 to 12.
    pub fn month(self) -> u32 {
        self.ymd().1
    }

    /// The day of the month, from 1 to 31.
    pub fn day(self) -> u32 {
        self.ymd().2
    }

    /// The day of the week.
    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday (index 3).
        Weekday::ALL[(self.0 + 3).rem_euclid(7) as usize]
    }

    /// The date `days` days later (or earlier, if negative).
    pub fn add_days(self, days: i32) -> Result<Date> {
        let total = self.0.checked_add(days).ok_or(Error::DateOutOfRange)?;
        Date::from_days_since_epoch(total)
    }

    /// The same day `months` months later (or earlier, if negative), moved back to the last day
    /// of the month when that day does not exist (2026-01-31 plus one month is 2026-02-28).
    pub fn add_months(self, months: i32) -> Result<Date> {
        let (year, month, day) = self.ymd();
        let index = i64::from(year) * 12 + i64::from(month) - 1 + i64::from(months);
        let new_year = i32::try_from(index.div_euclid(12)).map_err(|_| Error::DateOutOfRange)?;
        let new_month = (index.rem_euclid(12) + 1) as u32;
        if !(1..=9999).contains(&new_year) {
            return Err(Error::DateOutOfRange);
        }
        Date::from_ymd(
            new_year,
            new_month,
            day.min(days_in_month(new_year, new_month)),
        )
    }

    /// The number of days from `self` to `other`: positive when `other` is later.
    pub fn days_until(self, other: Date) -> i32 {
        other.0 - self.0
    }

    /// The last day of this date's month.
    pub fn end_of_month(self) -> Date {
        let (year, month, _) = self.ymd();
        Date(days_from_civil(year, month, days_in_month(year, month)))
    }
}

/// Whether `year` is a leap year in the Gregorian calendar.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The number of days in `month` (1 to 12) of `year`.
///
/// # Panics
///
/// Panics if `month` is not between 1 and 12.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => panic!("month {month} is outside 1 to 12"),
    }
}

// Days from civil date and back, after Howard Hinnant's algorithms
// (https://howardhinnant.github.io/date_algorithms.html).
fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32; // 0..=399
    let mp = (month + 9) % 12; // March = 0
    let doy = (153 * mp + 2) / 5 + day - 1; // 0..=365
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // 0..=146096
    era * 146_097 + doe as i32 - 719_468
}

fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe as i32 + era * 400 + i32::from(month <= 2);
    (year, month, day)
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Date({self})")
    }
}

impl FromStr for Date {
    type Err = Error;

    /// Parses an ISO 8601 calendar date, `YYYY-MM-DD`.
    fn from_str(s: &str) -> Result<Date> {
        let bytes = s.as_bytes();
        let well_formed = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
        if !well_formed {
            return Err(Error::InvalidDate(format!(
                "{s:?} is not in the form YYYY-MM-DD"
            )));
        }
        let number = |range: std::ops::Range<usize>| -> u32 {
            s[range]
                .bytes()
                .fold(0, |n, b| n * 10 + u32::from(b - b'0'))
        };
        Date::from_ymd(number(0..4) as i32, number(5..7), number(8..10))
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Date, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Date {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Date".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "format": "date",
            "description": "A calendar date in ISO 8601 form, for example \"2026-11-02\"."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn known_days() {
        assert_eq!(Date::from_ymd(1970, 1, 1).unwrap().days_since_epoch(), 0);
        assert_eq!(
            Date::from_ymd(2000, 3, 1).unwrap().days_since_epoch(),
            11_017
        );
        assert_eq!(Date::from_ymd(1, 1, 1).unwrap(), Date::MIN);
        assert_eq!(Date::from_ymd(9999, 12, 31).unwrap(), Date::MAX);
        assert_eq!(d("1969-12-31").days_since_epoch(), -1);
    }

    #[test]
    fn rejects_impossible_dates() {
        assert!(Date::from_ymd(2023, 2, 29).is_err());
        assert!(Date::from_ymd(1900, 2, 29).is_err());
        assert!(Date::from_ymd(2000, 2, 29).is_ok());
        assert!(Date::from_ymd(2026, 4, 31).is_err());
        assert!(Date::from_ymd(2026, 13, 1).is_err());
        assert!(Date::from_ymd(0, 1, 1).is_err());
        assert!(Date::from_ymd(10_000, 1, 1).is_err());
    }

    #[test]
    fn parses_only_iso_dates() {
        assert_eq!(d("2026-11-02"), Date::from_ymd(2026, 11, 2).unwrap());
        for bad in [
            "2026-1-02",
            "2026/11/02",
            "02-11-2026",
            "2026-11-02T00:00",
            "",
            "abcd-ef-gh",
        ] {
            assert!(bad.parse::<Date>().is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn weekdays() {
        assert_eq!(d("1970-01-01").weekday(), Weekday::Thursday);
        assert_eq!(d("2026-11-02").weekday(), Weekday::Monday);
        assert_eq!(d("2000-01-01").weekday(), Weekday::Saturday);
        assert_eq!(d("1969-12-28").weekday(), Weekday::Sunday);
    }

    #[test]
    fn month_arithmetic_clamps_to_the_month_end() {
        assert_eq!(d("2026-01-31").add_months(1).unwrap(), d("2026-02-28"));
        assert_eq!(d("2024-01-31").add_months(1).unwrap(), d("2024-02-29"));
        assert_eq!(d("2026-03-31").add_months(-1).unwrap(), d("2026-02-28"));
        assert_eq!(d("2026-11-15").add_months(-23).unwrap(), d("2024-12-15"));
        assert_eq!(d("2026-11-15").add_months(14).unwrap(), d("2028-01-15"));
        assert!(Date::MAX.add_months(1).is_err());
        assert!(Date::MIN.add_months(-1).is_err());
    }

    #[test]
    fn day_arithmetic_stays_in_range() {
        assert_eq!(d("2026-12-31").add_days(1).unwrap(), d("2027-01-01"));
        assert!(Date::MAX.add_days(1).is_err());
        assert!(Date::MIN.add_days(-1).is_err());
        assert_eq!(d("2026-01-01").days_until(d("2026-12-31")), 364);
        assert_eq!(d("2024-02-10").end_of_month(), d("2024-02-29"));
    }

    #[test]
    fn serialises_as_iso_text() {
        let date = d("2026-11-02");
        assert_eq!(serde_json::to_string(&date).unwrap(), "\"2026-11-02\"");
        assert_eq!(
            serde_json::from_str::<Date>("\"2026-11-02\"").unwrap(),
            date
        );
        assert!(serde_json::from_str::<Date>("\"2026-02-30\"").is_err());
    }

    proptest! {
        #[test]
        fn days_and_civil_dates_round_trip(days in MIN_DAYS..=MAX_DAYS) {
            let date = Date::from_days_since_epoch(days).unwrap();
            let (y, m, dd) = date.ymd();
            prop_assert_eq!(Date::from_ymd(y, m, dd).unwrap(), date);
            prop_assert_eq!(date.to_string().parse::<Date>().unwrap(), date);
        }

        #[test]
        fn consecutive_days_are_consecutive_weekdays(days in MIN_DAYS..MAX_DAYS) {
            let date = Date::from_days_since_epoch(days).unwrap();
            let next = date.add_days(1).unwrap();
            prop_assert_eq!((date.weekday().index() + 1) % 7, next.weekday().index());
        }
    }
}
