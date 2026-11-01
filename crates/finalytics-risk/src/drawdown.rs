//! Drawdowns: falls from a peak and the recoveries that follow.

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Result, TimeSeries, ValidationReport};

/// One fall from a peak, to the trough and, if it happened, back to the peak.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Drawdown {
    /// The date of the peak the fall started from.
    pub peak: Date,
    /// The date of the lowest point.
    pub trough: Date,
    /// The first date at or above the peak again, or `None` if the series has not recovered.
    pub recovery: Option<Date>,
    /// The fall from the peak to the trough, as a negative return (-0.25 for a fall of 25%).
    pub depth: f64,
    /// Calendar days from the peak to the trough.
    pub days_to_trough: i32,
    /// Calendar days from the trough to the recovery, if it happened.
    pub days_to_recovery: Option<i32>,
}

impl Drawdown {
    /// Calendar days from the peak to the recovery, or to `last` (the last date observed) if the
    /// series has not recovered.
    pub fn duration(&self, last: Date) -> i32 {
        self.peak.days_until(self.recovery.unwrap_or(last))
    }
}

/// Every drawdown in a series of levels (values or a growth index), in date order.
///
/// To list the worst first, sort by depth:
///
/// ```
/// use finalytics_core::{Date, TimeSeries};
/// use finalytics_risk::drawdowns;
///
/// let dates: Vec<Date> = ["2026-01-02", "2026-01-05", "2026-01-06", "2026-01-07", "2026-01-08"]
///     .iter()
///     .map(|d| d.parse().unwrap())
///     .collect();
/// let levels = TimeSeries::new(dates, vec![100.0, 95.0, 101.0, 90.0, 92.0])?;
/// let mut found = drawdowns(&levels)?;
/// found.sort_by(|a, b| a.depth.total_cmp(&b.depth));
/// assert_eq!(found[0].peak.to_string(), "2026-01-06");
/// assert_eq!(found[0].recovery, None);
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn drawdowns(levels: &TimeSeries<f64>) -> Result<Vec<Drawdown>> {
    check_levels(levels)?;
    let dates = levels.dates();
    let values = levels.values();
    let mut found = Vec::new();
    let Some(&first) = values.first() else {
        return Ok(found);
    };
    let (mut peak, mut peak_at) = (first, 0);
    let mut trough: Option<(f64, usize)> = None;
    for (t, &value) in values.iter().enumerate().skip(1) {
        if value >= peak {
            if let Some((low, low_at)) = trough.take() {
                found.push(episode(dates, peak, peak_at, low, low_at, Some(t)));
            }
            peak = value;
            peak_at = t;
        } else if trough.is_none_or(|(low, _)| value < low) {
            trough = Some((value, t));
        }
    }
    if let Some((low, low_at)) = trough {
        found.push(episode(dates, peak, peak_at, low, low_at, None));
    }
    Ok(found)
}

/// The fall below the running peak on each date, as a negative return (0 at a peak).
pub fn drawdown_series(levels: &TimeSeries<f64>) -> Result<TimeSeries<f64>> {
    check_levels(levels)?;
    let mut peak = f64::NEG_INFINITY;
    Ok(levels.map(|&value| {
        peak = peak.max(value);
        value / peak - 1.0
    }))
}

fn episode(
    dates: &[Date],
    peak: f64,
    peak_at: usize,
    low: f64,
    low_at: usize,
    recovered_at: Option<usize>,
) -> Drawdown {
    let recovery = recovered_at.map(|t| dates[t]);
    Drawdown {
        peak: dates[peak_at],
        trough: dates[low_at],
        recovery,
        depth: low / peak - 1.0,
        days_to_trough: dates[peak_at].days_until(dates[low_at]),
        days_to_recovery: recovery.map(|r| dates[low_at].days_until(r)),
    }
}

fn check_levels(levels: &TimeSeries<f64>) -> Result<()> {
    let mut report = ValidationReport::new();
    for (k, value) in levels.values().iter().enumerate() {
        if !(value.is_finite() && *value > 0.0) {
            report.error(
                format!("levels[{k}]"),
                "must be a finite number greater than zero",
            );
        }
    }
    report.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(values: &[f64]) -> TimeSeries<f64> {
        let start: Date = "2026-01-01".parse().unwrap();
        let dates = (0..values.len())
            .map(|i| start.add_days(i32::try_from(i).unwrap()).unwrap())
            .collect();
        TimeSeries::new(dates, values.to_vec()).unwrap()
    }

    #[test]
    fn episodes_with_and_without_recovery() {
        let levels = series(&[100.0, 110.0, 99.0, 88.0, 105.0, 110.0, 112.0, 100.8, 106.0]);
        let found = drawdowns(&levels).unwrap();
        assert_eq!(found.len(), 2);

        let first = &found[0];
        assert_eq!(first.peak.to_string(), "2026-01-02");
        assert_eq!(first.trough.to_string(), "2026-01-04");
        assert_eq!(
            first.recovery.map(|d| d.to_string()),
            Some("2026-01-06".into())
        );
        assert!((first.depth + 0.2).abs() < 1e-15);
        assert_eq!(first.days_to_trough, 2);
        assert_eq!(first.days_to_recovery, Some(2));
        assert_eq!(first.duration(levels.dates()[8]), 4);

        let second = &found[1];
        assert_eq!(second.peak.to_string(), "2026-01-07");
        assert_eq!(second.recovery, None);
        assert!((second.depth + 0.1).abs() < 1e-15);
        assert_eq!(second.duration(levels.dates()[8]), 2);

        let underwater = drawdown_series(&levels).unwrap();
        assert_eq!(underwater.values()[1], 0.0);
        assert!((underwater.values()[3] + 0.2).abs() < 1e-15);
        assert_eq!(underwater.values()[5], 0.0);
    }

    #[test]
    fn a_rising_series_has_none() {
        assert!(
            drawdowns(&series(&[1.0, 1.0, 2.0, 3.0]))
                .unwrap()
                .is_empty()
        );
        assert!(drawdowns(&series(&[])).unwrap().is_empty());
        assert!(drawdowns(&series(&[1.0, 0.0])).is_err());
    }
}
