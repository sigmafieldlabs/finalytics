//! Business-day calendars.

use serde::{Deserialize, Serialize};

use crate::{Date, Error, Result, Weekday};

/// How to move a date that falls on a non-business day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BusinessDayConvention {
    /// Leave the date as it is.
    Unadjusted,
    /// The next business day.
    Following,
    /// The next business day, unless that is in the next month; then the previous business day.
    ModifiedFollowing,
    /// The previous business day.
    Preceding,
}

/// A business-day calendar: weekend days plus a list of holidays.
///
/// Finalytics ships no holiday data. Callers supply holidays for the markets they need, which
/// keeps the engine free of data that goes out of date.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; }
/// use finalytics::{Calendar, Date};
///
/// let christmas = Date::from_ymd(2026, 12, 25)?;
/// let calendar = Calendar::weekends_only().with_holidays([christmas]);
/// let thursday = Date::from_ymd(2026, 12, 24)?;
/// assert_eq!(calendar.add_business_days(thursday, 1)?, Date::from_ymd(2026, 12, 28)?);
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "CalendarSpec", into = "CalendarSpec")]
pub struct Calendar {
    weekend: [bool; 7],
    holidays: Vec<Date>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct CalendarSpec {
    weekend: Vec<Weekday>,
    holidays: Vec<Date>,
}

impl TryFrom<CalendarSpec> for Calendar {
    type Error = Error;

    fn try_from(spec: CalendarSpec) -> Result<Calendar> {
        Ok(Calendar::new(&spec.weekend)?.with_holidays(spec.holidays))
    }
}

impl From<Calendar> for CalendarSpec {
    fn from(calendar: Calendar) -> CalendarSpec {
        CalendarSpec {
            weekend: calendar.weekend_days(),
            holidays: calendar.holidays,
        }
    }
}

impl Default for Calendar {
    fn default() -> Self {
        Calendar::weekends_only()
    }
}

impl Calendar {
    /// A calendar whose only non-business days are Saturdays and Sundays.
    pub fn weekends_only() -> Calendar {
        let mut weekend = [false; 7];
        weekend[Weekday::Saturday.index()] = true;
        weekend[Weekday::Sunday.index()] = true;
        Calendar {
            weekend,
            holidays: Vec::new(),
        }
    }

    /// A calendar with the given weekend days and no holidays. At least one day of the week must
    /// be a business day.
    pub fn new(weekend: &[Weekday]) -> Result<Calendar> {
        let mut days = [false; 7];
        for day in weekend {
            days[day.index()] = true;
        }
        if days.iter().all(|&w| w) {
            return Err(Error::InvalidCalendar(
                "every day of the week is a weekend day".into(),
            ));
        }
        Ok(Calendar {
            weekend: days,
            holidays: Vec::new(),
        })
    }

    /// This calendar with `holidays` added. Duplicates are ignored.
    pub fn with_holidays(mut self, holidays: impl IntoIterator<Item = Date>) -> Calendar {
        self.holidays.extend(holidays);
        self.holidays.sort_unstable();
        self.holidays.dedup();
        self
    }

    /// The weekend days, Monday first.
    pub fn weekend_days(&self) -> Vec<Weekday> {
        [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ]
        .into_iter()
        .filter(|d| self.weekend[d.index()])
        .collect()
    }

    /// The holidays, in date order.
    pub fn holidays(&self) -> &[Date] {
        &self.holidays
    }

    /// Whether `date` is neither a weekend day nor a holiday.
    pub fn is_business_day(&self, date: Date) -> bool {
        !self.weekend[date.weekday().index()] && self.holidays.binary_search(&date).is_err()
    }

    /// The first business day strictly after `date`.
    pub fn next_business_day(&self, date: Date) -> Result<Date> {
        self.step(date, 1)
    }

    /// The last business day strictly before `date`.
    pub fn previous_business_day(&self, date: Date) -> Result<Date> {
        self.step(date, -1)
    }

    /// Moves `date` to a business day according to `convention`.
    pub fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Result<Date> {
        if self.is_business_day(date) {
            return Ok(date);
        }
        match convention {
            BusinessDayConvention::Unadjusted => Ok(date),
            BusinessDayConvention::Following => self.next_business_day(date),
            BusinessDayConvention::Preceding => self.previous_business_day(date),
            BusinessDayConvention::ModifiedFollowing => {
                let following = self.next_business_day(date)?;
                if following.month() == date.month() {
                    Ok(following)
                } else {
                    self.previous_business_day(date)
                }
            }
        }
    }

    /// The business day `n` business days after `date` (before it, if `n` is negative).
    ///
    /// When `date` is not a business day, counting starts from it all the same: one business day
    /// after a Saturday is the following Monday (on a weekends-only calendar).
    pub fn add_business_days(&self, date: Date, n: i32) -> Result<Date> {
        let direction = n.signum();
        let mut current = date;
        for _ in 0..n.unsigned_abs() {
            current = self.step(current, direction)?;
        }
        Ok(current)
    }

    /// The number of business days `d` with `start < d <= end`. It is negative, counting the
    /// other way, when `end` is before `start`.
    pub fn business_days_between(&self, start: Date, end: Date) -> i32 {
        if end < start {
            return -self.business_days_between(end, start);
        }
        let total = start.days_until(end);
        let weeks = total / 7;
        let working_per_week = self.weekend.iter().filter(|&&w| !w).count() as i32;
        let mut count = weeks * working_per_week;
        // The remaining days after the whole weeks.
        let mut day = start.days_since_epoch() + weeks * 7;
        let last = end.days_since_epoch();
        while day < last {
            day += 1;
            let weekday = (day + 3).rem_euclid(7) as usize;
            if !self.weekend[weekday] {
                count += 1;
            }
        }
        // Holidays on working weekdays in the interval.
        let lo = self.holidays.partition_point(|&h| h <= start);
        let hi = self.holidays.partition_point(|&h| h <= end);
        let holidays = self.holidays[lo..hi]
            .iter()
            .filter(|h| !self.weekend[h.weekday().index()])
            .count() as i32;
        count - holidays
    }

    fn step(&self, date: Date, direction: i32) -> Result<Date> {
        let mut current = date;
        loop {
            current = current.add_days(direction)?;
            if self.is_business_day(current) {
                return Ok(current);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    fn uk_end_of_2026() -> Calendar {
        Calendar::weekends_only().with_holidays([d("2026-12-25"), d("2026-12-28"), d("2027-01-01")])
    }

    #[test]
    fn weekends_and_holidays_are_not_business_days() {
        let cal = uk_end_of_2026();
        assert!(cal.is_business_day(d("2026-12-24")));
        assert!(!cal.is_business_day(d("2026-12-25")));
        assert!(!cal.is_business_day(d("2026-12-26")));
        assert!(!cal.is_business_day(d("2026-12-28")));
        assert_eq!(
            cal.next_business_day(d("2026-12-24")).unwrap(),
            d("2026-12-29")
        );
        assert_eq!(
            cal.previous_business_day(d("2027-01-04")).unwrap(),
            d("2026-12-31")
        );
    }

    #[test]
    fn conventions() {
        let cal = Calendar::weekends_only();
        let saturday = d("2026-10-31");
        assert_eq!(
            cal.adjust(saturday, BusinessDayConvention::Unadjusted)
                .unwrap(),
            saturday
        );
        assert_eq!(
            cal.adjust(saturday, BusinessDayConvention::Following)
                .unwrap(),
            d("2026-11-02")
        );
        assert_eq!(
            cal.adjust(saturday, BusinessDayConvention::Preceding)
                .unwrap(),
            d("2026-10-30")
        );
        assert_eq!(
            cal.adjust(saturday, BusinessDayConvention::ModifiedFollowing)
                .unwrap(),
            d("2026-10-30")
        );
        let business = d("2026-11-03");
        assert_eq!(
            cal.adjust(business, BusinessDayConvention::Following)
                .unwrap(),
            business
        );
    }

    #[test]
    fn counts_business_days() {
        let cal = uk_end_of_2026();
        assert_eq!(
            cal.business_days_between(d("2026-12-24"), d("2027-01-04")),
            4
        );
        assert_eq!(
            cal.business_days_between(d("2027-01-04"), d("2026-12-24")),
            -4
        );
        assert_eq!(
            cal.business_days_between(d("2026-11-02"), d("2026-11-02")),
            0
        );
        assert_eq!(
            cal.add_business_days(d("2026-12-24"), 4).unwrap(),
            d("2027-01-04")
        );
        assert_eq!(
            cal.add_business_days(d("2027-01-04"), -4).unwrap(),
            d("2026-12-24")
        );
    }

    #[test]
    fn rejects_a_calendar_without_business_days() {
        use Weekday::*;
        let all = [
            Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
        ];
        assert!(Calendar::new(&all).is_err());
        let gulf = Calendar::new(&[Friday, Saturday]).unwrap();
        assert!(gulf.is_business_day(d("2026-11-01")));
        assert!(!gulf.is_business_day(d("2026-10-30")));
    }

    #[test]
    fn serialises_with_its_weekend_and_holidays() {
        let cal = uk_end_of_2026();
        let json = serde_json::to_string(&cal).unwrap();
        assert_eq!(
            json,
            r#"{"weekend":["saturday","sunday"],"holidays":["2026-12-25","2026-12-28","2027-01-01"]}"#
        );
        assert_eq!(serde_json::from_str::<Calendar>(&json).unwrap(), cal);
        let invalid = r#"{"weekend":["monday","tuesday","wednesday","thursday","friday","saturday","sunday"],"holidays":[]}"#;
        assert!(serde_json::from_str::<Calendar>(invalid).is_err());
    }

    proptest! {
        #[test]
        fn counting_agrees_with_stepping(start in -1000i32..20_000, n in 0i32..400) {
            let cal = uk_end_of_2026().with_holidays([d("1970-03-02"), d("2000-01-03")]);
            let start = Date::from_days_since_epoch(start).unwrap();
            let end = cal.add_business_days(start, n).unwrap();
            prop_assert_eq!(cal.business_days_between(start, end), n);
            prop_assert!(cal.is_business_day(end) || n == 0);
            if cal.is_business_day(start) {
                prop_assert_eq!(cal.add_business_days(end, -n).unwrap(), start);
            }
        }
    }
}
