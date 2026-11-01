//! Time series indexed by date.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{Date, Error, Result};

/// Values indexed by strictly increasing dates.
///
/// Construction checks the ordering, so every `TimeSeries` can be searched by date. A series is
/// serialised as `{"dates": [...], "values": [...]}`, and deserialising checks it again.
///
/// ```
/// use finalytics_core::{Date, TimeSeries};
///
/// let d = |s: &str| s.parse::<Date>().unwrap();
/// let prices = TimeSeries::new(vec![d("2026-10-30"), d("2026-11-02")], vec![101.5, 102.0])?;
/// assert_eq!(prices.get(d("2026-11-02")), Some(&102.0));
/// assert_eq!(prices.as_of(d("2026-11-01")), Some((d("2026-10-30"), &101.5)));
/// # Ok::<(), finalytics_core::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeSeries<T> {
    dates: Vec<Date>,
    values: Vec<T>,
}

impl<T> Default for TimeSeries<T> {
    fn default() -> Self {
        TimeSeries {
            dates: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T> TimeSeries<T> {
    /// A series from dates in strictly increasing order and one value per date.
    pub fn new(dates: Vec<Date>, values: Vec<T>) -> Result<TimeSeries<T>> {
        if dates.len() != values.len() {
            return Err(Error::LengthMismatch {
                dates: dates.len(),
                values: values.len(),
            });
        }
        check_increasing(&dates)?;
        Ok(TimeSeries { dates, values })
    }

    /// A series from `(date, value)` pairs in any order. Two values on the same date are an error.
    pub fn from_unsorted(pairs: impl IntoIterator<Item = (Date, T)>) -> Result<TimeSeries<T>> {
        let mut pairs: Vec<(Date, T)> = pairs.into_iter().collect();
        pairs.sort_by_key(|(date, _)| *date);
        let (dates, values): (Vec<Date>, Vec<T>) = pairs.into_iter().unzip();
        check_increasing(&dates)?;
        Ok(TimeSeries { dates, values })
    }

    /// The number of observations.
    pub fn len(&self) -> usize {
        self.dates.len()
    }

    /// Whether the series has no observations.
    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }

    /// The dates, in increasing order.
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }

    /// The values, in date order.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The dates and the values.
    pub fn into_parts(self) -> (Vec<Date>, Vec<T>) {
        (self.dates, self.values)
    }

    /// The observations in date order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Date, &T)> + '_ {
        self.dates.iter().copied().zip(self.values.iter())
    }

    /// The value on `date`, if there is one.
    pub fn get(&self, date: Date) -> Option<&T> {
        self.dates
            .binary_search(&date)
            .ok()
            .map(|i| &self.values[i])
    }

    /// The first observation.
    pub fn first(&self) -> Option<(Date, &T)> {
        self.dates.first().copied().zip(self.values.first())
    }

    /// The last observation.
    pub fn last(&self) -> Option<(Date, &T)> {
        self.dates.last().copied().zip(self.values.last())
    }

    /// The last observation on or before `date`.
    pub fn as_of(&self, date: Date) -> Option<(Date, &T)> {
        let i = self.dates.partition_point(|&d| d <= date);
        (i > 0).then(|| (self.dates[i - 1], &self.values[i - 1]))
    }

    /// A series with each value transformed by `f`.
    pub fn map<U>(&self, f: impl FnMut(&T) -> U) -> TimeSeries<U> {
        TimeSeries {
            dates: self.dates.clone(),
            values: self.values.iter().map(f).collect(),
        }
    }
}

impl<T: Clone> TimeSeries<T> {
    /// The observations from `start` to `end`, both included.
    pub fn between(&self, start: Date, end: Date) -> TimeSeries<T> {
        let lo = self.dates.partition_point(|&d| d < start);
        let hi = self.dates.partition_point(|&d| d <= end).max(lo);
        TimeSeries {
            dates: self.dates[lo..hi].to_vec(),
            values: self.values[lo..hi].to_vec(),
        }
    }

    /// This series on the dates `dates` (strictly increasing), filling dates without an
    /// observation according to `fill`. Every filled value says where it came from.
    pub fn reindex(&self, dates: &[Date], fill: Fill) -> Result<TimeSeries<Observation<T>>> {
        check_increasing(dates)?;
        let mut values = Vec::with_capacity(dates.len());
        let mut next = 0; // index of the first own date after the last one consumed
        for &date in dates {
            while next < self.dates.len() && self.dates[next] <= date {
                next += 1;
            }
            let observation = match next.checked_sub(1) {
                Some(i) if self.dates[i] == date => Observation::Observed(self.values[i].clone()),
                Some(i) => match fill {
                    Fill::Previous { max_days }
                        if max_days.is_none_or(|max| {
                            i64::from(self.dates[i].days_until(date)) <= i64::from(max)
                        }) =>
                    {
                        Observation::Carried {
                            value: self.values[i].clone(),
                            from: self.dates[i],
                        }
                    }
                    _ => Observation::Missing,
                },
                None => Observation::Missing,
            };
            values.push(observation);
        }
        Ok(TimeSeries {
            dates: dates.to_vec(),
            values,
        })
    }
}

/// How [`TimeSeries::reindex`] fills a date that has no observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "method")]
pub enum Fill {
    /// Leave it missing.
    #[default]
    None,
    /// Carry the last earlier observation forward, if it is at most `max_days` calendar days
    /// old (any age when `max_days` is `None`).
    Previous {
        /// The oldest a carried value may be, in calendar days.
        max_days: Option<u32>,
    },
}

/// A value on a date after reindexing: observed, carried forward from an earlier date, or
/// missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation<T> {
    /// A value observed on this date.
    Observed(T),
    /// A value carried forward from the earlier date `from`.
    Carried {
        /// The value.
        value: T,
        /// The date it was observed.
        from: Date,
    },
    /// No value.
    Missing,
}

impl<T> Observation<T> {
    /// The value, observed or carried.
    pub fn value(&self) -> Option<&T> {
        match self {
            Observation::Observed(value) | Observation::Carried { value, .. } => Some(value),
            Observation::Missing => None,
        }
    }

    /// Whether the value was carried forward.
    pub fn is_carried(&self) -> bool {
        matches!(self, Observation::Carried { .. })
    }
}

/// Every date that appears in any of `date_lists`, in increasing order.
pub fn union_of_dates<'a>(date_lists: impl IntoIterator<Item = &'a [Date]>) -> Vec<Date> {
    let mut all: Vec<Date> = date_lists.into_iter().flatten().copied().collect();
    all.sort_unstable();
    all.dedup();
    all
}

/// The dates that appear in every one of `date_lists`, in increasing order.
pub fn intersection_of_dates<'a>(date_lists: impl IntoIterator<Item = &'a [Date]>) -> Vec<Date> {
    let mut lists = date_lists.into_iter();
    let Some(first) = lists.next() else {
        return Vec::new();
    };
    let mut common: Vec<Date> = first.to_vec();
    common.sort_unstable();
    common.dedup();
    for list in lists {
        let mut sorted = list.to_vec();
        sorted.sort_unstable();
        common.retain(|d| sorted.binary_search(d).is_ok());
    }
    common
}

fn check_increasing(dates: &[Date]) -> Result<()> {
    for pair in dates.windows(2) {
        if pair[1] == pair[0] {
            return Err(Error::DuplicateDate(pair[0]));
        }
        if pair[1] < pair[0] {
            return Err(Error::UnorderedDates {
                previous: pair[0],
                next: pair[1],
            });
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct RawSeries<T> {
    dates: Vec<Date>,
    values: Vec<T>,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for TimeSeries<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let raw = RawSeries::<T>::deserialize(deserializer)?;
        TimeSeries::new(raw.dates, raw.values).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    fn sample() -> TimeSeries<f64> {
        TimeSeries::new(
            vec![d("2026-10-28"), d("2026-10-30"), d("2026-11-02")],
            vec![1.0, 2.0, 3.0],
        )
        .unwrap()
    }

    #[test]
    fn construction_checks_order_and_lengths() {
        let (a, b) = (d("2026-10-28"), d("2026-10-30"));
        assert_eq!(
            TimeSeries::new(vec![b, a], vec![1, 2]),
            Err(Error::UnorderedDates {
                previous: b,
                next: a
            })
        );
        assert_eq!(
            TimeSeries::new(vec![a, a], vec![1, 2]),
            Err(Error::DuplicateDate(a))
        );
        assert_eq!(
            TimeSeries::new(vec![a], vec![1, 2]),
            Err(Error::LengthMismatch {
                dates: 1,
                values: 2
            })
        );
        let sorted = TimeSeries::from_unsorted([(b, "b"), (a, "a")]).unwrap();
        assert_eq!(sorted.values(), ["a", "b"]);
        assert!(TimeSeries::from_unsorted([(a, 1), (a, 2)]).is_err());
    }

    #[test]
    fn lookups() {
        let s = sample();
        assert_eq!(s.get(d("2026-10-30")), Some(&2.0));
        assert_eq!(s.get(d("2026-10-29")), None);
        assert_eq!(s.as_of(d("2026-10-29")), Some((d("2026-10-28"), &1.0)));
        assert_eq!(s.as_of(d("2026-10-27")), None);
        assert_eq!(s.as_of(d("2027-01-01")), Some((d("2026-11-02"), &3.0)));
        assert_eq!(s.first(), Some((d("2026-10-28"), &1.0)));
        assert_eq!(s.last(), Some((d("2026-11-02"), &3.0)));
        assert_eq!(
            s.between(d("2026-10-29"), d("2026-11-02")).values(),
            [2.0, 3.0]
        );
        assert!(s.between(d("2026-11-03"), d("2026-11-01")).is_empty());
        assert_eq!(s.map(|v| v * 10.0).values(), [10.0, 20.0, 30.0]);
    }

    #[test]
    fn reindex_marks_carried_and_missing_values() {
        let s = sample();
        let grid = [
            d("2026-10-27"),
            d("2026-10-28"),
            d("2026-10-29"),
            d("2026-11-01"),
            d("2026-11-02"),
        ];
        let none = s.reindex(&grid, Fill::None).unwrap();
        assert_eq!(
            none.values(),
            [
                Observation::Missing,
                Observation::Observed(1.0),
                Observation::Missing,
                Observation::Missing,
                Observation::Observed(3.0)
            ]
        );
        let carried = s
            .reindex(&grid, Fill::Previous { max_days: Some(1) })
            .unwrap();
        assert_eq!(
            carried.values(),
            [
                Observation::Missing,
                Observation::Observed(1.0),
                Observation::Carried {
                    value: 1.0,
                    from: d("2026-10-28")
                },
                Observation::Missing, // 2 days after 2026-10-30
                Observation::Observed(3.0)
            ]
        );
        let any_age = s.reindex(&grid, Fill::Previous { max_days: None }).unwrap();
        assert_eq!(
            any_age.values()[3],
            Observation::Carried {
                value: 2.0,
                from: d("2026-10-30")
            }
        );
        assert!(s.reindex(&[grid[1], grid[0]], Fill::None).is_err());
    }

    #[test]
    fn date_sets() {
        let a = [d("2026-01-01"), d("2026-01-03")];
        let b = [d("2026-01-02"), d("2026-01-03")];
        assert_eq!(
            union_of_dates([&a[..], &b[..]]),
            [d("2026-01-01"), d("2026-01-02"), d("2026-01-03")]
        );
        assert_eq!(intersection_of_dates([&a[..], &b[..]]), [d("2026-01-03")]);
        assert!(intersection_of_dates(std::iter::empty()).is_empty());
    }

    #[test]
    fn serialisation_checks_the_dates() {
        let json = serde_json::to_string(&sample()).unwrap();
        assert_eq!(
            json,
            r#"{"dates":["2026-10-28","2026-10-30","2026-11-02"],"values":[1.0,2.0,3.0]}"#
        );
        assert_eq!(
            serde_json::from_str::<TimeSeries<f64>>(&json).unwrap(),
            sample()
        );
        let unordered = r#"{"dates":["2026-10-30","2026-10-28"],"values":[1.0,2.0]}"#;
        assert!(serde_json::from_str::<TimeSeries<f64>>(unordered).is_err());
    }

    proptest! {
        #[test]
        fn reindexing_onto_its_own_dates_changes_nothing(days in proptest::collection::btree_set(0i32..20_000, 0..60)) {
            let dates: Vec<Date> = days.iter().map(|&n| Date::from_days_since_epoch(n).unwrap()).collect();
            let values: Vec<i32> = days.iter().copied().collect();
            let series = TimeSeries::new(dates.clone(), values.clone()).unwrap();
            let same = series.reindex(&dates, Fill::Previous { max_days: None }).unwrap();
            let observed: Vec<Option<i32>> = same.values().iter().map(|o| o.value().copied()).collect();
            prop_assert_eq!(observed, values.into_iter().map(Some).collect::<Vec<_>>());
            prop_assert!(same.values().iter().all(|o| !o.is_carried()));
        }

        #[test]
        fn as_of_never_looks_ahead(days in proptest::collection::btree_set(0i32..5_000, 1..40), probe in 0i32..5_000) {
            let dates: Vec<Date> = days.iter().map(|&n| Date::from_days_since_epoch(n).unwrap()).collect();
            let series = TimeSeries::new(dates, vec![(); days.len()]).unwrap();
            let probe = Date::from_days_since_epoch(probe).unwrap();
            if let Some((found, _)) = series.as_of(probe) {
                prop_assert!(found <= probe);
                prop_assert!(series.dates().iter().all(|&d| d <= found || d > probe));
            } else {
                prop_assert!(series.dates()[0] > probe);
            }
        }
    }
}
