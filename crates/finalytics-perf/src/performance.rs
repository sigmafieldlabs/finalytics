//! Time-weighted and money-weighted returns of a portfolio.

use finalytics_core::numeric::CompensatedSum;
use finalytics_core::{Date, Decimal, Error, Observation, Result, TimeSeries, ValidationReport};
use finalytics_ledger::ValueSeries;
use serde::{Deserialize, Serialize};

use crate::Period;

/// When external flows are assumed to happen within the day they are recorded on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FlowTiming {
    /// At the start of the day: a deposit earns that day's return.
    #[default]
    StartOfDay,
    /// At the end of the day: a deposit starts earning the next day.
    EndOfDay,
}

/// The number of days used to annualise returns.
pub const DAYS_PER_YEAR: f64 = 365.25;

/// A return over a reporting period.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PeriodReturn {
    /// The valuation date the period starts from.
    pub start: Date,
    /// The valuation date it ends on.
    pub end: Date,
    /// The cumulative return over the period.
    pub cumulative: f64,
    /// The annualised return, given only for periods of a year or more.
    pub annualised: Option<f64>,
}

/// A return compared with a benchmark's over the same dates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ExcessReturn {
    /// The portfolio's cumulative return.
    pub portfolio: f64,
    /// The benchmark's cumulative return.
    pub benchmark: f64,
    /// `portfolio - benchmark`.
    pub arithmetic: f64,
    /// `(1 + portfolio) / (1 + benchmark) - 1`.
    pub geometric: f64,
}

/// The performance of a portfolio from its daily values and external flows.
///
/// Returns are time-weighted: each day's return removes the effect of deposits and withdrawals,
/// and the days are chain-linked, so the result measures the investment decisions rather than
/// the timing of money in and out. Money-weighted returns ([`Performance::money_weighted`]) do
/// include that timing.
#[derive(Debug, Clone, PartialEq)]
pub struct Performance {
    pub(crate) dates: Vec<Date>,
    pub(crate) values: Vec<f64>,
    pub(crate) flows: Vec<f64>,
    returns: Vec<f64>,
    pub(crate) growth: Vec<f64>,
    pub(crate) timing: FlowTiming,
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}

impl Performance {
    /// Computes daily time-weighted returns from a [`ValueSeries`].
    ///
    /// The first date is the starting point. A day on which the portfolio has nothing invested
    /// (no value and no flow) has a return of zero. A day whose starting value is negative, or
    /// that ends with value but started with none, has no defined return and is reported as an
    /// error.
    pub fn from_values(series: &ValueSeries, timing: FlowTiming) -> Result<Performance> {
        let values: Vec<f64> = series.values.iter().copied().map(to_f64).collect();
        let flows: Vec<f64> = series.flows.iter().copied().map(to_f64).collect();
        Performance::from_parts(series.dates.clone(), values, flows, timing)
    }

    /// Computes daily time-weighted returns from end-of-day values and the external flows since
    /// the previous date (positive into the portfolio).
    pub fn from_parts(
        dates: Vec<Date>,
        values: Vec<f64>,
        flows: Vec<f64>,
        timing: FlowTiming,
    ) -> Result<Performance> {
        if values.len() != dates.len() || flows.len() != dates.len() {
            return Err(Error::LengthMismatch {
                dates: dates.len(),
                values: values.len().min(flows.len()),
            });
        }
        TimeSeries::new(dates.clone(), vec![(); dates.len()])?;
        let mut report = ValidationReport::new();
        let mut returns = Vec::with_capacity(dates.len().saturating_sub(1));
        let mut growth = Vec::with_capacity(dates.len());
        if !dates.is_empty() {
            growth.push(1.0);
        }
        for t in 1..dates.len() {
            let (base, end) = match timing {
                FlowTiming::StartOfDay => (values[t - 1] + flows[t], values[t]),
                FlowTiming::EndOfDay => (values[t - 1], values[t] - flows[t]),
            };
            let r = if base > 0.0 {
                end / base - 1.0
            } else if base == 0.0 && end.abs() < 1e-9 {
                0.0
            } else {
                report.error(
                    format!("values[{t}]"),
                    format!("no return can be measured on {}: the value invested at the start of the day is {base}", dates[t]),
                );
                0.0
            };
            returns.push(r);
            growth.push(growth[t - 1] * (1.0 + r));
        }
        report.into_result(Performance {
            dates,
            values,
            flows,
            returns,
            growth,
            timing,
        })
    }

    /// The valuation dates.
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }

    /// The daily returns, each indexed by the date it ends on.
    pub fn returns(&self) -> TimeSeries<f64> {
        TimeSeries::new(
            self.dates.iter().skip(1).copied().collect(),
            self.returns.clone(),
        )
        .unwrap_or_default()
    }

    /// The growth of `start` invested on the first date, at the time-weighted return.
    pub fn growth_of(&self, start: f64) -> TimeSeries<f64> {
        TimeSeries::new(
            self.dates.clone(),
            self.growth.iter().map(|g| g * start).collect(),
        )
        .unwrap_or_default()
    }

    /// The portfolio values, in the base currency.
    pub fn values(&self) -> TimeSeries<f64> {
        TimeSeries::new(self.dates.clone(), self.values.clone()).unwrap_or_default()
    }

    /// The last valuation date on or before `date`, as an index.
    fn index_as_of(&self, date: Date) -> Option<usize> {
        self.dates.partition_point(|&d| d <= date).checked_sub(1)
    }

    /// The cumulative time-weighted return from the valuation on or before `start` to the one on
    /// or before `end`.
    pub fn time_weighted(&self, start: Date, end: Date) -> Option<f64> {
        let (i, j) = (self.index_as_of(start)?, self.index_as_of(end)?);
        (i <= j).then(|| self.growth[j] / self.growth[i] - 1.0)
    }

    /// The return over `period`, ending on the last valuation on or before `as_of` (the last
    /// valuation when `as_of` is `None`). Returns `None` when the history does not reach back to
    /// the start of the period.
    pub fn period(&self, period: Period, as_of: Option<Date>) -> Option<PeriodReturn> {
        let end_index = match as_of {
            Some(date) => self.index_as_of(date)?,
            None => self.dates.len().checked_sub(1)?,
        };
        let end = self.dates[end_index];
        let start_index = match period.start_boundary(as_of.unwrap_or(end)).ok()? {
            None => 0,
            Some(boundary) => self.index_as_of(boundary)?,
        };
        let start = self.dates[start_index];
        let cumulative = self.growth[end_index] / self.growth[start_index] - 1.0;
        let days = start.days_until(end);
        let annualised = (f64::from(days) >= 365.0).then(|| annualise(cumulative, days));
        Some(PeriodReturn {
            start,
            end,
            cumulative,
            annualised,
        })
    }

    /// The money-weighted return (internal rate of return, annualised, actual/365 days) from the
    /// valuation on or before `start` to the one on or before `end`. The starting value counts
    /// as money invested on the start date; flows after it count on their own dates.
    ///
    /// Returns `None` if the rate cannot be found between -99.99% and +1,000,000%.
    pub fn money_weighted(&self, start: Date, end: Date) -> Option<f64> {
        let (i, j) = (self.index_as_of(start)?, self.index_as_of(end)?);
        if i >= j {
            return None;
        }
        let origin = self.dates[i];
        let mut cash_flows = vec![(0.0, -self.values[i])];
        for t in i + 1..=j {
            if self.flows[t] != 0.0 {
                let mut years = f64::from(origin.days_until(self.dates[t])) / 365.0;
                if self.timing == FlowTiming::StartOfDay {
                    years -= 1.0 / 365.0;
                }
                cash_flows.push((years.max(0.0), -self.flows[t]));
            }
        }
        cash_flows.push((
            f64::from(origin.days_until(self.dates[j])) / 365.0,
            self.values[j],
        ));
        internal_rate_of_return(&cash_flows)
    }

    /// The Modified Dietz return from the valuation on or before `start` to the one on or before
    /// `end`: the gain divided by the starting value plus the flows weighted by the time they
    /// were invested.
    pub fn modified_dietz(&self, start: Date, end: Date) -> Option<f64> {
        let (i, j) = (self.index_as_of(start)?, self.index_as_of(end)?);
        if i >= j {
            return None;
        }
        let total_days = f64::from(self.dates[i].days_until(self.dates[j]));
        let mut net = CompensatedSum::new();
        let mut weighted = CompensatedSum::new();
        for t in i + 1..=j {
            let elapsed = f64::from(self.dates[i].days_until(self.dates[t]));
            let invested_days = match self.timing {
                FlowTiming::StartOfDay => total_days - elapsed + 1.0,
                FlowTiming::EndOfDay => total_days - elapsed,
            };
            net.add(self.flows[t]);
            weighted.add(self.flows[t] * invested_days / total_days);
        }
        let denominator = self.values[i] + weighted.value();
        (denominator > 0.0).then(|| (self.values[j] - self.values[i] - net.value()) / denominator)
    }

    /// The portfolio's return over `period` against a benchmark given as index levels (or
    /// prices), looked up on the same start and end dates (carried forward up to
    /// `max_carry_days`).
    pub fn excess_return(
        &self,
        period: Period,
        as_of: Option<Date>,
        benchmark: &TimeSeries<f64>,
        max_carry_days: u32,
    ) -> Option<ExcessReturn> {
        let p = self.period(period, as_of)?;
        let level = |date: Date| -> Option<f64> {
            match benchmark
                .reindex(
                    &[date],
                    finalytics_core::Fill::Previous {
                        max_days: Some(max_carry_days),
                    },
                )
                .ok()?
                .values()[0]
            {
                Observation::Observed(v) | Observation::Carried { value: v, .. } if v > 0.0 => {
                    Some(v)
                }
                _ => None,
            }
        };
        let b = level(p.end)? / level(p.start)? - 1.0;
        Some(ExcessReturn {
            portfolio: p.cumulative,
            benchmark: b,
            arithmetic: p.cumulative - b,
            geometric: (1.0 + p.cumulative) / (1.0 + b) - 1.0,
        })
    }
}

/// Annualises a cumulative return earned over `days` calendar days.
pub fn annualise(cumulative: f64, days: i32) -> f64 {
    (1.0 + cumulative).powf(DAYS_PER_YEAR / f64::from(days)) - 1.0
}

/// The rate `r` at which `Σ amount / (1 + r)^years` is zero, for `(years, amount)` cash flows.
pub fn internal_rate_of_return(cash_flows: &[(f64, f64)]) -> Option<f64> {
    let npv = |rate: f64| -> f64 {
        let mut sum = CompensatedSum::new();
        for &(years, amount) in cash_flows {
            sum.add(amount / (1.0 + rate).powf(years));
        }
        sum.value()
    };
    // Scan outwards from zero for a sign change, then bisect.
    let grid: Vec<f64> = {
        let mut g = vec![
            -0.9999, -0.99, -0.9, -0.75, -0.5, -0.25, -0.1, 0.0, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0,
            10.0, 100.0, 1_000.0, 10_000.0,
        ];
        g.sort_by(f64::total_cmp);
        g
    };
    let zero = grid.iter().position(|&r| r == 0.0)?;
    let mut order: Vec<usize> = Vec::new();
    for k in 0..grid.len() {
        if zero + k + 1 < grid.len() {
            order.push(zero + k);
        }
        if zero > k {
            order.push(zero - k - 1);
        }
    }
    for i in order {
        let (mut lo, mut hi) = (grid[i], grid[i + 1]);
        let (mut f_lo, f_hi) = (npv(lo), npv(hi));
        if f_lo == 0.0 {
            return Some(lo);
        }
        if !(f_lo.is_finite() && f_hi.is_finite()) || f_lo.signum() == f_hi.signum() {
            continue;
        }
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            let f_mid = npv(mid);
            if f_mid == 0.0 || (hi - lo) < 1e-15 * (1.0 + mid.abs()) {
                return Some(mid);
            }
            if f_mid.signum() == f_lo.signum() {
                lo = mid;
                f_lo = f_mid;
            } else {
                hi = mid;
            }
        }
        return Some(0.5 * (lo + hi));
    }
    None
}
