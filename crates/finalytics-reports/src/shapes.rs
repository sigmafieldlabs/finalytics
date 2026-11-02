//! The shapes reports use for charts: series, breakdowns, grids and histograms.

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date};

/// What the numbers of a series, grid or column measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "unit", rename_all = "snake_case")]
pub enum Unit {
    /// An amount of money in a currency.
    Money {
        /// The currency.
        currency: Currency,
    },
    /// A return, weight or share as a fraction: 0.05 is 5%.
    Fraction,
    /// A ratio, such as a Sharpe ratio or a beta.
    Ratio,
    /// The level of an index, such as the growth of 100.
    Level,
    /// A count.
    Count,
}

/// A labelled series of numbers over time, for a line or bar chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Series {
    /// What the series is, such as "Portfolio" or "Rolling volatility (63 days)".
    pub name: String,
    /// What its values measure.
    pub unit: Unit,
    /// The dates.
    pub dates: Vec<Date>,
    /// One value per date.
    pub values: Vec<f64>,
}

/// One part of a whole, for a pie, bar or treemap chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Slice {
    /// The part's name: a holding, a currency, a group.
    pub name: String,
    /// Its value in the base currency.
    pub value: f64,
    /// Its share of the whole.
    pub weight: f64,
}

/// A whole split into parts by one dimension, such as by currency or by asset class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Breakdown {
    /// The dimension: "holding", "currency", "kind", "account", or a classification scheme.
    pub by: String,
    /// The parts, largest first.
    pub slices: Vec<Slice>,
}

/// A table of numbers with labelled rows and columns, for a heatmap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Grid {
    /// What the grid is, such as "Monthly returns".
    pub name: String,
    /// What its values measure.
    pub unit: Unit,
    /// The row labels.
    pub rows: Vec<String>,
    /// The column labels.
    pub columns: Vec<String>,
    /// One row of values per row label, one value per column; absent where there is none.
    pub values: Vec<Vec<Option<f64>>>,
}

/// One bin of a [`Histogram`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bin {
    /// The lower edge, included.
    pub lower: f64,
    /// The upper edge, excluded (included for the last bin).
    pub upper: f64,
    /// The number of observations in the bin.
    pub count: usize,
    /// The share of all observations in the bin.
    pub share: f64,
    /// The share a normal distribution with the observations' mean and standard deviation puts
    /// in the bin, for comparison.
    pub normal_share: f64,
}

/// The distribution of a set of observations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Histogram {
    /// What the observations are.
    pub name: String,
    /// What they measure.
    pub unit: Unit,
    /// Bins of equal width from the smallest observation to the largest.
    pub bins: Vec<Bin>,
}

/// A figure as a fraction and as an amount of money, such as a value at risk.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Amount {
    /// As a fraction of the portfolio's value.
    pub fraction: f64,
    /// As an amount in the base currency.
    pub value: f64,
}

/// Bins of equal width over the finite `values`, with the normal distribution's share of each.
pub(crate) fn histogram(name: &str, unit: Unit, values: &[f64], bins: usize) -> Histogram {
    let values: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = values.len();
    let (lo, hi) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    if n == 0 || bins == 0 || !(lo.is_finite() && hi.is_finite()) {
        return Histogram {
            name: name.to_owned(),
            unit,
            bins: Vec::new(),
        };
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let sd = if n > 1 {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        0.0
    };
    let width = if hi > lo {
        (hi - lo) / bins as f64
    } else {
        1.0
    };
    let bins = if hi > lo { bins } else { 1 };
    let mut counts = vec![0usize; bins];
    for &v in &values {
        let k = (((v - lo) / width) as usize).min(bins - 1);
        counts[k] += 1;
    }
    let cdf = |x: f64| {
        if sd > 0.0 {
            finalytics_risk::normal_cdf((x - mean) / sd)
        } else if x >= mean {
            1.0
        } else {
            0.0
        }
    };
    let bins = counts
        .iter()
        .enumerate()
        .map(|(k, &count)| {
            let lower = lo + width * k as f64;
            let upper = if k + 1 == bins {
                hi.max(lower)
            } else {
                lo + width * (k + 1) as f64
            };
            Bin {
                lower,
                upper,
                count,
                share: count as f64 / n as f64,
                normal_share: cdf(upper) - cdf(lower),
            }
        })
        .collect();
    Histogram {
        name: name.to_owned(),
        unit,
        bins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_are_equal_and_the_last_is_closed() {
        let h = histogram("r", Unit::Fraction, &[0.0, 1.0, 2.0, 3.0, 4.0], 2);
        let counts: Vec<usize> = h.bins.iter().map(|b| b.count).collect();
        assert_eq!(counts, [2, 3]);
        assert_eq!((h.bins[0].lower, h.bins[0].upper), (0.0, 2.0));
        assert_eq!((h.bins[1].lower, h.bins[1].upper), (2.0, 4.0));
        assert_eq!(h.bins[1].share, 0.6);
        let normal: f64 = h.bins.iter().map(|b| b.normal_share).sum();
        assert!(normal > 0.0 && normal < 1.0);
    }

    #[test]
    fn equal_observations_fill_one_bin_and_others_are_left_out() {
        let h = histogram("r", Unit::Fraction, &[0.01; 4], 10);
        assert_eq!(h.bins.len(), 1);
        assert_eq!((h.bins[0].count, h.bins[0].share), (4, 1.0));
        assert!(histogram("r", Unit::Fraction, &[], 10).bins.is_empty());
        let h = histogram("r", Unit::Fraction, &[0.0, f64::NAN, f64::INFINITY], 10);
        assert_eq!((h.bins.len(), h.bins[0].count), (1, 1));
    }
}
