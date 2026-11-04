//! The risk report: ex-post statistics, drawdowns, tail risk, the return distribution, the
//! holdings' covariance and correlation, risk decomposition and rolling statistics.

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Result, ValidationReport};
use finalytics_ledger::{Books, HoldingKey, MarketData, Valuation};
use finalytics_perf::Performance;
use finalytics_risk::{
    CovarianceMethod, Drawdown, Matrix, RelativeStatistics, RiskDecomposition, SchemeRisk,
    Statistics, TailMethod, covariance, drawdown_series, drawdowns, group_risk, holding_returns,
    holding_weights, relative_statistics, risk_decomposition, rolling_relative_statistics,
    rolling_statistics, statistics, tail_risk, tracking_error_decomposition,
};

use crate::context::{
    Context, Provenance, ReportKind, ReportOptions, Source, group_of, label, to_f64,
};
use crate::shapes::{Amount, Grid, Histogram, Series, Unit, histogram};

/// Value at risk and expected shortfall by one method at one confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TailRow {
    /// The confidence.
    pub confidence: f64,
    /// The method.
    pub method: TailMethod,
    /// The value at risk over one valuation period, a loss.
    pub value_at_risk: Amount,
    /// The expected shortfall over one valuation period, a loss.
    pub expected_shortfall: Amount,
    /// Whether the expected shortfall was raised to the value at risk, as the Cornish–Fisher
    /// expansion can need when the excess kurtosis is large.
    pub shortfall_at_var: bool,
}

/// The current holdings' risk, from the covariance of their returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct HoldingsRisk {
    /// The first date of the returns: the first valuation date on which every current holding
    /// has a price.
    pub start: Date,
    /// The number of returns.
    pub observations: usize,
    /// How the covariance was estimated.
    pub method: CovarianceMethod,
    /// The annualised covariance of the holdings' returns in the base currency, labelled by
    /// holding.
    pub covariance: Matrix,
    /// Their correlations, for a heatmap.
    pub correlation: Grid,
    /// The annualised volatility of the current weights, by holding.
    pub decomposition: RiskDecomposition,
    /// The decomposition added up by each scheme of the options.
    pub groups: Vec<SchemeRisk>,
    /// The ex-ante tracking error against the benchmark weights of the options, by holding.
    pub tracking_error: Option<RiskDecomposition>,
}

/// The risk report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RiskReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The portfolio's value on the report's date.
    pub value: f64,
    /// The portfolio's time-weighted returns, one per valuation period.
    pub returns: Series,
    /// Statistics of the returns, annualised.
    pub statistics: Statistics,
    /// Statistics against the benchmark, with a benchmark.
    pub relative: Option<RelativeStatistics>,
    /// The fall from the running peak on each valuation date.
    pub drawdown: Series,
    /// The largest drawdowns, deepest first.
    pub drawdowns: Vec<Drawdown>,
    /// Value at risk and expected shortfall by every method at every confidence of the
    /// options.
    pub tail: Vec<TailRow>,
    /// The distribution of the returns, with the normal distribution's for comparison.
    pub histogram: Histogram,
    /// The current holdings' risk; absent (with a warning) when it cannot be estimated.
    pub holdings: Option<HoldingsRisk>,
    /// Rolling annualised volatility, Sharpe ratio and, with a benchmark, beta and tracking
    /// error.
    pub rolling: Vec<Series>,
}

/// Risk measured from the first valuation date to the report's date.
pub fn risk_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<RiskReport> {
    let cx = Context::new(books, market, source, options)?;
    let values = books.value_series(&cx.dates, market)?;
    let mut warnings = values.warnings.clone();
    let performance = Performance::from_values(&values, options.timing)?;
    let returns_ts = performance.returns();
    let returns = returns_ts.values().to_vec();
    let stats_options = cx.statistics_options();
    let stats = statistics(&returns, &stats_options)?;
    let value = values.values.last().copied().map_or(0.0, to_f64);

    let benchmark = cx.benchmark_returns();
    let relative = benchmark
        .as_ref()
        .and_then(|b| relative_statistics(&returns, b, &stats_options).ok());

    let growth = performance.growth_of(1.0);
    let (drawdown, mut largest) = match (drawdown_series(&growth), drawdowns(&growth)) {
        (Ok(series), Ok(all)) => (series, all),
        (Err(e), _) | (_, Err(e)) => {
            warnings.warning("drawdown", format!("drawdowns could not be measured: {e}"));
            (Default::default(), Vec::new())
        }
    };
    largest.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    largest.truncate(options.top);

    let mut tail = Vec::new();
    for &confidence in &options.confidences {
        for method in [
            TailMethod::Historical,
            TailMethod::Parametric,
            TailMethod::CornishFisher,
        ] {
            match tail_risk(&returns, confidence, method) {
                Ok(t) => tail.push(TailRow {
                    confidence,
                    method,
                    value_at_risk: Amount {
                        fraction: t.value_at_risk,
                        value: t.value_at_risk * value,
                    },
                    expected_shortfall: Amount {
                        fraction: t.expected_shortfall,
                        value: t.expected_shortfall * value,
                    },
                    shortfall_at_var: t.shortfall_at_var,
                }),
                Err(e) => warnings.warning("tail", format!("{method:?} at {confidence}: {e}")),
            }
        }
    }

    let valuation = books.value_on(cx.as_of, market)?;
    let holdings = match holdings_risk(&cx, &valuation) {
        Ok(e) => Some(e.risk),
        Err(e) => {
            warnings.warning(
                "holdings",
                format!("the holdings' risk could not be estimated: {e}"),
            );
            None
        }
    };

    let window = options.rolling_window;
    let mut rolling = Vec::new();
    let returns_dates = returns_ts.dates();
    if returns.len() >= window {
        let dates = returns_dates[window - 1..].to_vec();
        if let Ok(stats) = rolling_statistics(&returns, window, &stats_options) {
            rolling.push(Series {
                name: format!("Rolling volatility ({window} periods)"),
                unit: Unit::Fraction,
                dates: dates.clone(),
                values: stats.iter().map(|s| s.volatility).collect(),
            });
            rolling.push(Series {
                name: format!("Rolling Sharpe ratio ({window} periods)"),
                unit: Unit::Ratio,
                dates: dates.clone(),
                values: stats
                    .iter()
                    .map(|s| s.sharpe_ratio.unwrap_or(f64::NAN))
                    .collect(),
            });
        }
        if let Some(b) = &benchmark
            && let Ok(rel) = rolling_relative_statistics(&returns, b, window, &stats_options)
        {
            rolling.push(Series {
                name: format!("Rolling beta ({window} periods)"),
                unit: Unit::Ratio,
                dates: dates.clone(),
                values: rel.iter().map(|r| r.beta.unwrap_or(f64::NAN)).collect(),
            });
            rolling.push(Series {
                name: format!("Rolling tracking error ({window} periods)"),
                unit: Unit::Fraction,
                dates,
                values: rel.iter().map(|r| r.tracking_error).collect(),
            });
        }
        for series in &mut rolling {
            // Undefined figures, such as a Sharpe ratio over a window without volatility,
            // are left as gaps rather than numbers.
            if series.values.iter().any(|v| v.is_nan()) {
                let (dates, values): (Vec<Date>, Vec<f64>) = series
                    .dates
                    .iter()
                    .zip(&series.values)
                    .filter(|(_, v)| !v.is_nan())
                    .map(|(d, v)| (*d, *v))
                    .unzip();
                series.dates = dates;
                series.values = values;
            }
        }
    } else {
        warnings.warning(
            "rolling",
            format!(
                "{} returns are fewer than the rolling window of {window}",
                returns.len()
            ),
        );
    }

    Ok(RiskReport {
        provenance: cx.provenance(ReportKind::Risk, warnings),
        value,
        returns: Series {
            name: "Return".to_owned(),
            unit: Unit::Fraction,
            dates: returns_dates.to_vec(),
            values: returns.clone(),
        },
        statistics: stats,
        relative,
        drawdown: Series {
            name: "Drawdown".to_owned(),
            unit: Unit::Fraction,
            dates: drawdown.dates().to_vec(),
            values: drawdown.values().to_vec(),
        },
        drawdowns: largest,
        tail,
        histogram: histogram("Returns", Unit::Fraction, &returns, options.histogram_bins),
        holdings,
        rolling,
    })
}

impl Context<'_> {
    /// The benchmark's returns between consecutive valuation dates, with a benchmark that has
    /// a price on every date.
    pub(crate) fn benchmark_returns(&self) -> Option<Vec<f64>> {
        let levels = self.benchmark_levels()?;
        Some(levels.windows(2).map(|p| p[1] / p[0] - 1.0).collect())
    }
}

/// The current holdings' risk, with the holdings, their weights and their annualised mean
/// returns, in the same order.
pub(crate) struct Estimated {
    pub(crate) risk: HoldingsRisk,
    pub(crate) keys: Vec<HoldingKey>,
    pub(crate) weights: Vec<f64>,
    pub(crate) means: Vec<f64>,
}

/// The current holdings' annualised covariance and risk, on the valuation dates from the first
/// on which every holding has a price.
pub(crate) fn holdings_risk(cx: &Context<'_>, valuation: &Valuation) -> Result<Estimated> {
    let weights = holding_weights(valuation)?;
    let keys: Vec<HoldingKey> = weights.keys().cloned().collect();
    let w: Vec<f64> = weights.values().copied().collect();
    let mut start = cx.start;
    for key in &keys {
        if let HoldingKey::Instrument(id) = key
            && let Some((first, _)) = cx.market.prices(id).and_then(|p| p.first())
        {
            start = start.max(first);
        }
    }
    let dates: Vec<Date> = cx.dates.iter().copied().filter(|d| *d >= start).collect();
    // Cash in the base currency is riskless. It is left out of the estimate, so that shrinkage
    // towards the average variance does not lend it risk, and its row and column are zero.
    let riskless = HoldingKey::Cash(cx.base);
    let risky: Vec<HoldingKey> = keys.iter().filter(|k| **k != riskless).cloned().collect();
    let periods = cx.options.periods_per_year;
    let (estimated, observations, columns) = if risky.is_empty() {
        (Vec::new(), dates.len().saturating_sub(1), Vec::new())
    } else {
        let returns = holding_returns(cx.books, &risky, &dates, cx.market)?;
        let estimate = covariance(&returns.table, cx.options.covariance)?;
        (
            estimate.matrix.scaled(periods).values().to_vec(),
            estimate.observations,
            returns.table.columns().to_vec(),
        )
    };
    let place: Vec<Option<usize>> = keys
        .iter()
        .map(|k| risky.iter().position(|r| r == k))
        .collect();
    let (n, m) = (keys.len(), risky.len());
    let mut values = vec![0.0; n * n];
    for (i, a) in place.iter().enumerate() {
        for (j, b) in place.iter().enumerate() {
            if let (Some(a), Some(b)) = (a, b) {
                values[i * n + j] = estimated[a * m + b];
            }
        }
    }
    let labels: Vec<String> = keys.iter().map(label).collect();
    let annual = Matrix::new(labels.clone(), values)?;
    let decomposition = risk_decomposition(&w, &annual)?;
    let groups = cx
        .options
        .group_by
        .iter()
        .map(|scheme| {
            SchemeRisk::new(
                scheme.clone(),
                group_risk(&decomposition, |k, _| group_of(cx.books, &keys[k], scheme)),
            )
        })
        .collect();
    let tracking_error = if cx.options.benchmark_weights.is_empty() {
        None
    } else {
        let mut report = ValidationReport::new();
        let mut b = vec![0.0; keys.len()];
        for (key, weight) in &cx.options.benchmark_weights {
            match keys.iter().position(|x| x == key) {
                Some(i) => b[i] += weight,
                None => report.error(
                    format!("options.benchmark_weights.{key}"),
                    format!("{} is not held, so its risk is not estimated", label(key)),
                ),
            }
        }
        report.into_result(())?;
        Some(tracking_error_decomposition(&w, &b, &annual)?)
    };
    let n = labels.len();
    let correlation = match annual.correlation() {
        Ok(c) => c
            .values()
            .chunks(n)
            .map(|r| r.iter().map(|v| Some(*v)).collect())
            .collect(),
        // Holdings without variance, such as base-currency cash, have no correlation.
        Err(_) => {
            let sd: Vec<f64> = annual.diagonal().iter().map(|v| v.sqrt()).collect();
            (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            (sd[i] > 0.0 && sd[j] > 0.0)
                                .then(|| (annual.get(i, j) / (sd[i] * sd[j])).clamp(-1.0, 1.0))
                        })
                        .collect()
                })
                .collect()
        }
    };
    let means = place
        .iter()
        .map(|p| match p {
            Some(a) => columns[*a].iter().sum::<f64>() / columns[*a].len().max(1) as f64 * periods,
            None => 0.0,
        })
        .collect();
    let risk = HoldingsRisk {
        start,
        observations,
        method: cx.options.covariance,
        covariance: annual,
        correlation: Grid {
            name: "Correlation".to_owned(),
            unit: Unit::Ratio,
            rows: labels.clone(),
            columns: labels,
            values: correlation,
        },
        decomposition,
        groups,
        tracking_error,
    };
    Ok(Estimated {
        risk,
        keys,
        weights: w,
        means,
    })
}
