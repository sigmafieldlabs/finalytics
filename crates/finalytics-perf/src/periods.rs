//! Reporting periods.

use finalytics_core::{Date, Result};
use serde::{Deserialize, Serialize};

/// A reporting period ending on an as-of date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case", tag = "period")]
#[non_exhaustive]
pub enum Period {
    /// From the end of the previous month.
    MonthToDate,
    /// From the end of the previous calendar quarter.
    QuarterToDate,
    /// From the end of the previous calendar year.
    YearToDate,
    /// The `years` years before the as-of date (1, 3, 5 …).
    Years {
        /// The number of years.
        years: u32,
    },
    /// From the first valuation.
    SinceInception,
    /// From `start` to the as-of date.
    Since {
        /// The start of the period.
        start: Date,
    },
}

impl Period {
    /// The date the period starts from (the last day before it, for calendar periods), or `None`
    /// for [`Period::SinceInception`], which starts at the first valuation.
    pub fn start_boundary(self, as_of: Date) -> Result<Option<Date>> {
        let (year, month, _) = as_of.ymd();
        Ok(Some(match self {
            Period::MonthToDate => Date::from_ymd(year, month, 1)?.add_days(-1)?,
            Period::QuarterToDate => {
                let first_month = (month - 1) / 3 * 3 + 1;
                Date::from_ymd(year, first_month, 1)?.add_days(-1)?
            }
            Period::YearToDate => Date::from_ymd(year, 1, 1)?.add_days(-1)?,
            Period::Years { years } => {
                let months = i32::try_from(years.saturating_mul(12)).unwrap_or(i32::MAX);
                as_of.add_months(-months)?
            }
            Period::SinceInception => return Ok(None),
            Period::Since { start } => start,
        }))
    }

    /// A short label, such as `"YTD"` or `"3Y"`.
    pub fn label(self) -> String {
        match self {
            Period::MonthToDate => "MTD".into(),
            Period::QuarterToDate => "QTD".into(),
            Period::YearToDate => "YTD".into(),
            Period::Years { years } => format!("{years}Y"),
            Period::SinceInception => "ITD".into(),
            Period::Since { start } => format!("since {start}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn boundaries() {
        let as_of = d("2026-11-02");
        let start = |p: Period| p.start_boundary(as_of).unwrap();
        assert_eq!(start(Period::MonthToDate), Some(d("2026-10-31")));
        assert_eq!(start(Period::QuarterToDate), Some(d("2026-09-30")));
        assert_eq!(start(Period::YearToDate), Some(d("2025-12-31")));
        assert_eq!(start(Period::Years { years: 3 }), Some(d("2023-11-02")));
        assert_eq!(start(Period::SinceInception), None);
        assert_eq!(
            Period::QuarterToDate
                .start_boundary(d("2026-02-15"))
                .unwrap(),
            Some(d("2025-12-31"))
        );
        assert_eq!(
            Period::Years { years: 1 }
                .start_boundary(d("2024-02-29"))
                .unwrap(),
            Some(d("2023-02-28"))
        );
        assert_eq!(Period::Years { years: 3 }.label(), "3Y");
    }
}
