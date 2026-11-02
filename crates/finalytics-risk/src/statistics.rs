//! Statistics of a returns series, on its own and against a benchmark.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::input::{self, Moments, invalid};

/// Conventions for [`statistics`] and [`relative_statistics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct StatisticsOptions {
    /// Return periods in a year: 252 for daily trading-day returns (the default), 52 for weekly,
    /// 12 for monthly.
    pub periods_per_year: f64,
    /// The annual risk-free rate for the Sharpe ratio and alpha. It is converted to a rate per
    /// period by compounding: `(1 + rate)^(1 / periods_per_year) - 1`. The default is 0.
    pub risk_free_rate: f64,
    /// The annual minimum acceptable return for downside deviation and the Sortino ratio,
    /// converted to a rate per period in the same way. The default is 0.
    pub target_return: f64,
}

impl Default for StatisticsOptions {
    fn default() -> Self {
        StatisticsOptions {
            periods_per_year: 252.0,
            risk_free_rate: 0.0,
            target_return: 0.0,
        }
    }
}

impl StatisticsOptions {
    fn validate(&self) -> Result<()> {
        if !(self.periods_per_year.is_finite() && self.periods_per_year > 0.0) {
            return Err(invalid("periods_per_year", "must be greater than zero"));
        }
        for (name, rate) in [
            ("risk_free_rate", self.risk_free_rate),
            ("target_return", self.target_return),
        ] {
            if !(rate.is_finite() && rate > -1.0) {
                return Err(invalid(name, "must be a finite rate above -100%"));
            }
        }
        Ok(())
    }

    /// An annual rate as a rate per period.
    fn per_period(&self, annual: f64) -> f64 {
        (1.0 + annual).powf(1.0 / self.periods_per_year) - 1.0
    }
}

/// Statistics of a returns series.
///
/// Annualised figures scale by the number of periods in a year given in the options. Ratios
/// that divide by a dispersion are absent when the returns do not vary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Statistics {
    /// The number of returns.
    pub observations: usize,
    /// The compounded return over the whole series.
    pub total_return: f64,
    /// The compound annual growth rate: `(1 + total_return)^(periods_per_year / observations) - 1`.
    pub annualised_return: f64,
    /// The annualised standard deviation of the returns (sample, divisor n - 1).
    pub volatility: f64,
    /// The annualised root mean square of the shortfalls below the target return, over all
    /// observations.
    pub downside_deviation: f64,
    /// The mean return in excess of the risk-free rate over its standard deviation, annualised by
    /// the square root of `periods_per_year`.
    pub sharpe_ratio: Option<f64>,
    /// The annualised mean return in excess of the target over the downside deviation.
    pub sortino_ratio: Option<f64>,
    /// The largest fall from a peak, as a negative return (-0.25 for a fall of 25%), measured
    /// from a starting value before the first return.
    pub max_drawdown: f64,
    /// The annualised return over the size of the maximum drawdown; absent without a drawdown.
    pub calmar_ratio: Option<f64>,
    /// The skewness of the returns (population moments).
    pub skewness: Option<f64>,
    /// The kurtosis of the returns less 3, so that a normal distribution has 0 (population
    /// moments).
    pub excess_kurtosis: Option<f64>,
    /// The best single return.
    pub best: f64,
    /// The worst single return.
    pub worst: f64,
}

/// Statistics of a returns series against a benchmark's returns over the same periods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RelativeStatistics {
    /// The number of returns.
    pub observations: usize,
    /// The annualised return less the benchmark's annualised return.
    pub active_return: f64,
    /// The annualised standard deviation of the active returns (returns less benchmark returns).
    pub tracking_error: f64,
    /// The annualised mean active return over the tracking error.
    pub information_ratio: Option<f64>,
    /// The covariance of the returns with the benchmark's over the benchmark's variance.
    pub beta: Option<f64>,
    /// The mean return not explained by beta, in excess of the risk-free rate, annualised by
    /// compounding: `(1 + mean(r - rf - beta (b - rf)))^periods_per_year - 1`.
    pub alpha: Option<f64>,
    /// The correlation of the returns with the benchmark's.
    pub correlation: Option<f64>,
}

/// Computes the statistics of a returns series of at least two periods.
///
/// ```
/// use finalytics_risk::{StatisticsOptions, statistics};
///
/// let returns = [0.010, -0.004, 0.007, -0.012, 0.003];
/// let stats = statistics(&returns, &StatisticsOptions::default())?;
/// assert_eq!(stats.observations, 5);
/// assert!(stats.max_drawdown < 0.0);
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn statistics(returns: &[f64], options: &StatisticsOptions) -> Result<Statistics> {
    options.validate()?;
    input::returns(returns, "returns", 2)?;
    let p = options.periods_per_year;
    let moments = Moments::of(returns);
    let sd = moments.sample_sd();
    let constant = moments.is_constant();

    let total_return = total_return(returns);
    let annualised_return = annualise(total_return, returns.len(), p);

    let risk_free = options.per_period(options.risk_free_rate);
    let sharpe_ratio = (!constant).then(|| (moments.mean - risk_free) / sd * p.sqrt());

    let target = options.per_period(options.target_return);
    let shortfall = returns
        .iter()
        .map(|r| (r - target).min(0.0).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let downside = shortfall.sqrt();
    let downside_deviation = downside * p.sqrt();
    let sortino_ratio = (downside > 0.0).then(|| (moments.mean - target) * p / downside_deviation);

    let max_drawdown = max_drawdown(returns);
    let calmar_ratio = (max_drawdown < 0.0).then(|| annualised_return / -max_drawdown);

    Ok(Statistics {
        observations: returns.len(),
        total_return,
        annualised_return,
        volatility: if constant { 0.0 } else { sd * p.sqrt() },
        downside_deviation,
        sharpe_ratio,
        sortino_ratio,
        max_drawdown,
        calmar_ratio,
        skewness: moments.skewness(),
        excess_kurtosis: moments.excess_kurtosis(),
        best: returns.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        worst: returns.iter().copied().fold(f64::INFINITY, f64::min),
    })
}

/// Computes statistics of a returns series against a benchmark's returns over the same periods.
pub fn relative_statistics(
    returns: &[f64],
    benchmark: &[f64],
    options: &StatisticsOptions,
) -> Result<RelativeStatistics> {
    options.validate()?;
    input::paired_returns(returns, benchmark, 2)?;
    let p = options.periods_per_year;
    let n = returns.len();

    let active: Vec<f64> = returns.iter().zip(benchmark).map(|(r, b)| r - b).collect();
    let active_moments = Moments::of(&active);
    let tracking_error = if active_moments.is_constant() {
        0.0
    } else {
        active_moments.sample_sd() * p.sqrt()
    };
    let information_ratio =
        (!active_moments.is_constant()).then(|| active_moments.mean * p / tracking_error);

    let r = Moments::of(returns);
    let b = Moments::of(benchmark);
    let covariance = returns
        .iter()
        .zip(benchmark)
        .map(|(x, y)| (x - r.mean) * (y - b.mean))
        .sum::<f64>()
        / n as f64;
    let beta = (!b.is_constant()).then(|| covariance / b.m2);
    let correlation =
        (!b.is_constant() && !r.is_constant()).then(|| covariance / (r.m2 * b.m2).sqrt());
    let risk_free = options.per_period(options.risk_free_rate);
    let alpha = beta.map(|beta| {
        let mean = (r.mean - risk_free) - beta * (b.mean - risk_free);
        (1.0 + mean).powf(p) - 1.0
    });

    let active_return =
        annualise(total_return(returns), n, p) - annualise(total_return(benchmark), n, p);
    Ok(RelativeStatistics {
        observations: n,
        active_return,
        tracking_error,
        information_ratio,
        beta,
        alpha,
        correlation,
    })
}

/// The compounded return of a series of returns.
pub fn total_return(returns: &[f64]) -> f64 {
    returns.iter().fold(1.0, |growth, r| growth * (1.0 + r)) - 1.0
}

/// The largest fall from a peak in the growth of a series of returns, as a negative return,
/// measured from a starting value before the first return. Returns 0 if the series never falls.
pub fn max_drawdown(returns: &[f64]) -> f64 {
    let mut level = 1.0;
    let mut peak: f64 = 1.0;
    let mut worst: f64 = 0.0;
    for r in returns {
        level *= 1.0 + r;
        peak = peak.max(level);
        worst = worst.min(level / peak - 1.0);
    }
    worst
}

fn annualise(total: f64, observations: usize, periods_per_year: f64) -> f64 {
    (1.0 + total).powf(periods_per_year / observations as f64) - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_returns_have_no_ratios() {
        let stats = statistics(&[0.001; 30], &StatisticsOptions::default()).unwrap();
        assert_eq!(stats.volatility, 0.0);
        assert_eq!(stats.sharpe_ratio, None);
        assert_eq!(stats.sortino_ratio, None);
        assert_eq!(stats.calmar_ratio, None);
        assert_eq!(stats.skewness, None);
        assert_eq!(stats.max_drawdown, 0.0);
        assert!((stats.total_return - (1.001f64.powi(30) - 1.0)).abs() < 1e-15);
    }

    #[test]
    fn invalid_input_is_reported_by_position() {
        let Err(finalytics_core::Error::Validation(report)) =
            statistics(&[0.01, f64::NAN, -1.5], &StatisticsOptions::default())
        else {
            panic!("expected a validation error");
        };
        let paths: Vec<_> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["returns[1]", "returns[2]"]);
        assert!(statistics(&[0.01], &StatisticsOptions::default()).is_err());
        let options = StatisticsOptions {
            periods_per_year: 0.0,
            ..Default::default()
        };
        assert!(statistics(&[0.01, 0.02], &options).is_err());
        assert!(
            relative_statistics(&[0.01, 0.02], &[0.01], &StatisticsOptions::default()).is_err()
        );
    }

    #[test]
    fn a_series_against_itself() {
        let r = [0.01, -0.02, 0.015, 0.003, -0.007];
        let rel = relative_statistics(&r, &r, &StatisticsOptions::default()).unwrap();
        assert_eq!(rel.tracking_error, 0.0);
        assert_eq!(rel.information_ratio, None);
        assert!((rel.beta.unwrap() - 1.0).abs() < 1e-15);
        assert!((rel.correlation.unwrap() - 1.0).abs() < 1e-15);
        assert!(rel.alpha.unwrap().abs() < 1e-15);
        assert_eq!(rel.active_return, 0.0);
    }

    #[test]
    fn drawdown_from_the_starting_value() {
        assert!((max_drawdown(&[-0.1, 0.05]) + 0.1).abs() < 1e-15);
        assert!((max_drawdown(&[0.1, -0.5, 0.2, -0.2]) + 0.52).abs() < 1e-15);
        assert_eq!(max_drawdown(&[0.01, 0.02]), 0.0);
    }
}
