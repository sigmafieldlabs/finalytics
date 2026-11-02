//! The overview report: headline figures, growth against the benchmark, allocation, risk
//! contributions, each holding's expected return and risk, and the efficient frontier.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Result, ValidationReport};
use finalytics_ledger::{Books, HoldingKey, MarketData};
use finalytics_opt::{FrontierPoint, Problem};
use finalytics_perf::{Performance, Period};
use finalytics_risk::{TailMethod, statistics, tail_risk};

use crate::context::{
    Context, Provenance, ReportKind, ReportOptions, Source, group_of, label, to_f64,
};
use crate::risk::{Estimated, HoldingsRisk, holdings_risk};
use crate::shapes::{Amount, Breakdown, Series, Slice, Unit};

/// The headline figures of the overview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Headline {
    /// The portfolio's value in the base currency.
    pub value: f64,
    /// The gain over the last valuation period: the change in value less the flows.
    pub period_gain: Option<f64>,
    /// The time-weighted return over the last valuation period.
    pub period_return: Option<f64>,
    /// The time-weighted return year to date.
    pub year_to_date: Option<f64>,
    /// The time-weighted return since the first valuation.
    pub since_inception: Option<f64>,
    /// The annualised return since the first valuation, for a year or more.
    pub since_inception_annualised: Option<f64>,
    /// The annualised volatility.
    pub volatility: f64,
    /// The Sharpe ratio.
    pub sharpe_ratio: Option<f64>,
    /// The maximum drawdown, a negative return.
    pub max_drawdown: f64,
    /// The confidence of the value at risk and expected shortfall: the first of the options.
    pub confidence: Option<f64>,
    /// The historical value at risk over one valuation period.
    pub value_at_risk: Option<Amount>,
    /// The historical expected shortfall over one valuation period.
    pub expected_shortfall: Option<Amount>,
}

/// One holding's weight, expected return and risk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct HoldingRiskReturn {
    /// The holding.
    pub holding: HoldingKey,
    /// Its share of the portfolio's value.
    pub weight: f64,
    /// Its annual expected return: given in the options, or its historical mean.
    pub expected_return: f64,
    /// Whether the expected return was given rather than the historical mean.
    pub expected_return_given: bool,
    /// Its annualised volatility.
    pub volatility: f64,
    /// Its share of the portfolio's volatility.
    pub risk_share: Option<f64>,
}

/// A portfolio's expected return and volatility, for a point on a risk–return chart.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PortfolioPoint {
    /// The annual expected return.
    pub expected_return: f64,
    /// The annualised volatility.
    pub volatility: f64,
}

/// The overview report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OverviewReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The headline figures.
    pub headline: Headline,
    /// The portfolio's value on each valuation date.
    pub value: Series,
    /// The growth of 100 at the time-weighted return.
    pub growth: Series,
    /// The benchmark's growth of 100, with a benchmark.
    pub benchmark_growth: Option<Series>,
    /// The value split by holding and by each scheme of the options.
    pub allocation: Vec<Breakdown>,
    /// The holdings' risk; absent (with a warning) when it cannot be estimated.
    pub risk: Option<HoldingsRisk>,
    /// Each holding's weight, expected return and risk.
    pub holdings: Vec<HoldingRiskReturn>,
    /// The efficient frontier of the holdings, long only and fully invested, from the expected
    /// returns and the covariance; absent (with a warning) when it cannot be found.
    pub frontier: Option<Vec<FrontierPoint>>,
    /// The current portfolio on the same axes as the frontier.
    pub current: Option<PortfolioPoint>,
}

/// The overview on the report's date.
pub fn overview_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<OverviewReport> {
    let cx = Context::new(books, market, source, options)?;
    let values = books.value_series(&cx.dates, market)?;
    let mut warnings: ValidationReport = values.warnings.clone();
    let performance = Performance::from_values(&values, options.timing)?;
    let returns = performance.returns().values().to_vec();
    let stats = statistics(&returns, &cx.statistics_options())?;
    let value_now = values.values.last().copied().map_or(0.0, to_f64);
    let n = cx.dates.len();
    let (period_gain, period_return) = if n >= 2 {
        let gain = to_f64(values.values[n - 1])
            - to_f64(values.values[n - 2])
            - to_f64(values.flows[n - 1]);
        (Some(gain), returns.last().copied())
    } else {
        (None, None)
    };
    let ytd = performance.period(Period::YearToDate, Some(cx.as_of));
    let itd = performance.period(Period::SinceInception, Some(cx.as_of));
    let confidence = options.confidences.first().copied();
    let tail = confidence.and_then(|c| tail_risk(&returns, c, TailMethod::Historical).ok());
    let headline = Headline {
        value: value_now,
        period_gain,
        period_return,
        year_to_date: ytd.map(|p| p.cumulative),
        since_inception: itd.as_ref().map(|p| p.cumulative),
        since_inception_annualised: itd.as_ref().and_then(|p| p.annualised),
        volatility: stats.volatility,
        sharpe_ratio: stats.sharpe_ratio,
        max_drawdown: stats.max_drawdown,
        confidence,
        value_at_risk: tail.as_ref().map(|t| Amount {
            fraction: t.value_at_risk,
            value: t.value_at_risk * value_now,
        }),
        expected_shortfall: tail.as_ref().map(|t| Amount {
            fraction: t.expected_shortfall,
            value: t.expected_shortfall * value_now,
        }),
    };

    let growth = performance.growth_of(100.0);
    let growth = Series {
        name: "Portfolio".to_owned(),
        unit: Unit::Level,
        dates: growth.dates().to_vec(),
        values: growth.values().to_vec(),
    };
    let benchmark_growth = cx.benchmark_levels().map(|l| Series {
        name: options
            .benchmark
            .as_ref()
            .map_or_else(String::new, ToString::to_string),
        unit: Unit::Level,
        dates: cx.dates.clone(),
        values: l.iter().map(|v| v / l[0] * 100.0).collect(),
    });

    let valuation = books.value_on(cx.as_of, market)?;
    let total = to_f64(valuation.total_base);
    let mut by_holding: BTreeMap<HoldingKey, f64> = BTreeMap::new();
    for p in &valuation.positions {
        *by_holding
            .entry(HoldingKey::Instrument(p.instrument.clone()))
            .or_default() += to_f64(p.market_value_base);
    }
    for c in valuation.cash.iter().filter(|c| !c.amount.is_zero()) {
        *by_holding.entry(HoldingKey::Cash(c.currency)).or_default() += to_f64(c.value_base);
    }
    let breakdown = |by: &str, name_of: &dyn Fn(&HoldingKey) -> String| {
        let mut parts: BTreeMap<String, f64> = BTreeMap::new();
        for (key, v) in &by_holding {
            *parts.entry(name_of(key)).or_default() += v;
        }
        let mut slices: Vec<Slice> = parts
            .into_iter()
            .map(|(name, value)| Slice {
                name,
                value,
                weight: if total > 0.0 { value / total } else { 0.0 },
            })
            .collect();
        slices.sort_by(|a, b| {
            b.value
                .total_cmp(&a.value)
                .then_with(|| a.name.cmp(&b.name))
        });
        Breakdown {
            by: by.to_owned(),
            slices,
        }
    };
    let mut allocation = vec![breakdown("holding", &label)];
    for scheme in &options.group_by {
        allocation.push(breakdown(scheme, &|k| group_of(books, k, scheme)));
    }

    let (risk, holdings, frontier, current) = match holdings_risk(&cx, &valuation) {
        Err(e) => {
            warnings.warning(
                "risk",
                format!("the holdings' risk could not be estimated: {e}"),
            );
            (None, Vec::new(), None, None)
        }
        Ok(Estimated {
            risk,
            keys,
            weights,
            means,
        }) => {
            let given: BTreeMap<_, _> = options.expected_returns.iter().cloned().collect();
            let expected: Vec<(f64, bool)> = keys
                .iter()
                .zip(&means)
                .map(|(key, mean)| match key {
                    HoldingKey::Instrument(id) => match given.get(id) {
                        Some(r) => (*r, true),
                        None => (*mean, false),
                    },
                    HoldingKey::Cash(_) => (*mean, false),
                })
                .collect();
            let holdings: Vec<HoldingRiskReturn> = keys
                .iter()
                .enumerate()
                .map(|(i, key)| HoldingRiskReturn {
                    holding: key.clone(),
                    weight: weights[i],
                    expected_return: expected[i].0,
                    expected_return_given: expected[i].1,
                    volatility: risk.covariance.get(i, i).max(0.0).sqrt(),
                    risk_share: risk.decomposition.contributions[i].share,
                })
                .collect();
            let mu: Vec<f64> = expected.iter().map(|(r, _)| *r).collect();
            let current = PortfolioPoint {
                expected_return: weights.iter().zip(&mu).map(|(w, r)| w * r).sum(),
                volatility: risk.decomposition.volatility,
            };
            let frontier = match Problem::new(risk.covariance.clone())
                .with_expected_returns(mu)
                .frontier(options.frontier_points)
            {
                Ok(points) => Some(points),
                Err(e) => {
                    warnings.warning(
                        "frontier",
                        format!("the efficient frontier could not be found: {e}"),
                    );
                    None
                }
            };
            (Some(risk), holdings, frontier, Some(current))
        }
    };

    Ok(OverviewReport {
        provenance: cx.provenance(ReportKind::Overview, warnings),
        headline,
        value: Series {
            name: "Value".to_owned(),
            unit: Unit::Money { currency: cx.base },
            dates: cx.dates.clone(),
            values: values.values.iter().copied().map(to_f64).collect(),
        },
        growth,
        benchmark_growth,
        allocation,
        risk,
        holdings,
        frontier,
        current,
    })
}
