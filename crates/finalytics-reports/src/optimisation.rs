//! The optimisation report: the weights an optimisation chose, what they are expected to
//! deliver, where their risk comes from, where they sit against each asset and on the efficient
//! frontier, and how they would have done on the history they were fitted to.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Date, Result};
use finalytics_risk::{CovarianceMethod, Matrix, MeanMethod};

use crate::backtest_html::{
    BENCHMARK, Frame, Line, SERIES, capitalise, frame, frequency, key, level, level_tick, lines,
    notes_table, percent, section, span, table,
};
use crate::backtests::RunReport;
use crate::html::{card, esc, heat, legend, ticks, tiles};
use crate::shapes::{Grid, Unit};
use crate::text::{
    L, NOISE, Page, R, WIDTH, bar, covariance_label, covariance_name, fit, mean_label, num,
    pad_left, pct, period_word, plural, share, spark, width, wrap,
};

/// The weights an optimisation chose, with what they are expected to deliver, where their risk
/// comes from, where they sit against each asset and on the efficient frontier, and how they
/// would have done on the history they were fitted to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationReport {
    /// What was optimised and how.
    pub context: OptimisationContext,
    /// What the weights are expected to deliver; absent when the optimisation found no weights.
    pub summary: Option<OptimisationSummary>,
    /// Each asset's weight, expected return, volatility and share of the risk, the largest
    /// weight first.
    pub allocation: Vec<AllocationRow>,
    /// Each group of assets with limits on its total weight, with that weight and its share of
    /// the risk, in name order.
    #[serde(default)]
    pub groups: Vec<GroupRow>,
    /// The correlation of the assets' returns, from the covariance the optimisation used.
    pub correlation: Grid,
    /// The efficient frontier under the same limits, from the least volatile portfolio to the
    /// one with the largest expected return; empty unless the optimisation asked for it.
    pub frontier: Vec<RiskReturn>,
    /// The equal-weight portfolio of the same assets, for comparison.
    pub equal_weight: Option<RiskReturn>,
    /// How the weights, and equal weight, would have done on the history they were fitted to:
    /// held from the start and rebalanced quarterly, without trading costs.
    pub history: Option<OptimisationHistory>,
    /// The report's title, such as "Six asset classes": shown when it is printed or laid out as
    /// a page.
    #[serde(default)]
    pub name: Option<String>,
}

/// What an optimisation report describes: the method, the data and estimates, the limits and
/// how the solver ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationContext {
    /// The engine's version.
    pub version: String,
    /// A SHA-256 digest of the estimates and limits, in hexadecimal: the same inputs give the
    /// same report.
    pub digest: String,
    /// The method's name, such as "Maximum Sharpe".
    pub method: String,
    /// What the method does, in a sentence.
    pub description: String,
    /// How the optimisation ended, such as `optimal`.
    pub status: String,
    /// How it was solved, such as `interior_point`.
    pub solver: String,
    /// The solver's iterations.
    pub iterations: u32,
    /// The assets optimised over, other than the risk-free asset.
    pub assets: Vec<String>,
    /// The risk-free asset, if one of the assets is.
    pub risk_free_asset: Option<String>,
    /// The annual risk-free rate of the Sharpe ratio.
    pub risk_free_rate: f64,
    /// The number of return periods in a year.
    pub periods_per_year: f64,
    /// The covariance estimator.
    pub covariance: CovarianceMethod,
    /// The estimator of the expected returns: `None` when some were given.
    pub mean_returns: Option<MeanMethod>,
    /// The limits on the weights, in words.
    pub limits: Vec<String>,
    /// The date the first return period ends on.
    pub start: Date,
    /// The date the last return period ends on.
    pub end: Date,
    /// For resampled optimisation, how the weights were averaged.
    #[serde(default)]
    pub resampling: Option<ResamplingSummary>,
}

/// How resampled weights were found: the average of the optimal weights for histories simulated
/// from the estimates (Michaud, 1998).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ResamplingSummary {
    /// The number of histories simulated.
    pub samples: usize,
    /// The number of periods in each, as many as the returns the estimates came from.
    pub observations: usize,
    /// The number of simulated problems solved and averaged.
    pub solved: usize,
}

impl ResamplingSummary {
    /// `samples` histories of `observations` periods, of which `solved` were solved.
    pub fn new(samples: usize, observations: usize, solved: usize) -> ResamplingSummary {
        ResamplingSummary {
            samples,
            observations,
            solved,
        }
    }
}

/// What the weights are expected to deliver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationSummary {
    /// The annual expected return.
    pub expected_return: f64,
    /// The annual volatility.
    pub volatility: f64,
    /// The Sharpe ratio, against the risk-free rate.
    pub sharpe_ratio: Option<f64>,
    /// The weighted average of the assets' volatilities over the portfolio's: how much
    /// diversification reduces the risk. One for a single asset.
    pub diversification_ratio: Option<f64>,
    /// The number of equal weights with the same concentration: one over the sum of the squared
    /// weights.
    pub effective_assets: f64,
    /// The asset with the largest weight.
    pub largest_asset: String,
    /// Its weight.
    pub largest_weight: f64,
    /// For minimum CVaR, the expected shortfall the weights reach, which the method minimises;
    /// the shares of risk are then of it.
    #[serde(default)]
    pub expected_shortfall: Option<ShortfallFigure>,
}

/// An expected shortfall over one period of the returns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ShortfallFigure {
    /// The confidence, such as 0.95.
    pub confidence: f64,
    /// The mean loss in the worst `1 - confidence` of the periods, as a positive return.
    pub value: f64,
}

/// A group of assets with limits on its total weight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct GroupRow {
    /// The group's name, such as "Energy".
    pub name: String,
    /// Its assets.
    pub assets: Vec<String>,
    /// The total weight of its assets.
    pub weight: f64,
    /// The smallest total weight allowed.
    pub min: f64,
    /// The largest total weight allowed.
    pub max: f64,
    /// The sum of its assets' shares of the risk.
    pub risk_share: f64,
}

/// One asset in the weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AllocationRow {
    /// The asset.
    pub asset: String,
    /// Its weight.
    pub weight: f64,
    /// Its annual expected return.
    pub expected_return: f64,
    /// Its annual volatility.
    pub volatility: f64,
    /// Its share of the portfolio's risk: of its variance (its weight times its covariance with
    /// the portfolio, over the portfolio's variance), or for minimum CVaR of its expected
    /// shortfall (its weight times its mean loss in the worst periods, over the expected
    /// shortfall).
    pub risk_share: f64,
}

/// A portfolio's volatility and expected return, a year.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RiskReturn {
    /// The annual volatility.
    pub volatility: f64,
    /// The annual expected return.
    pub expected_return: f64,
}

/// How the weights, and equal weight, would have done on the history they were fitted to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationHistory {
    /// The weights, held from the start and rebalanced quarterly.
    pub weights: RunReport,
    /// Equal weight, held the same way.
    pub equal_weight: RunReport,
}

/// What the optimisation report is built from. The `finalytics` crate assembles it.
#[doc(hidden)]
#[derive(Debug)]
pub struct OptimisationInputs<'a> {
    pub method: String,
    pub description: String,
    pub status: String,
    pub solver: String,
    pub iterations: u32,
    pub labels: &'a [String],
    pub weights: Option<&'a BTreeMap<String, f64>>,
    pub expected_returns: &'a [f64],
    pub mean_returns: Option<MeanMethod>,
    pub covariance: &'a Matrix,
    pub risk_free_asset: Option<String>,
    pub risk_free_rate: f64,
    pub periods_per_year: f64,
    pub covariance_method: CovarianceMethod,
    pub limits: Vec<String>,
    pub start: Date,
    pub end: Date,
    pub frontier: Vec<(f64, f64)>,
    pub history: Option<(RunReport, RunReport)>,
    /// For minimum CVaR, the weights' expected shortfall split by asset, in the order of
    /// `labels`.
    pub shortfall: Option<finalytics_risk::ShortfallDecomposition>,
    /// The groups with limits: name, assets, smallest and largest total weight.
    pub groups: Vec<(String, Vec<String>, f64, f64)>,
    pub resampling: Option<ResamplingSummary>,
}

/// The optimisation report from what the `finalytics` crate assembles.
#[doc(hidden)]
pub fn optimisation_report(inputs: OptimisationInputs<'_>) -> Result<OptimisationReport> {
    let n = inputs.labels.len();
    let cov = |i: usize, j: usize| inputs.covariance.get(i, j);
    let vol: Vec<f64> = (0..n).map(|i| cov(i, i).max(0.0).sqrt()).collect();
    let weights: Option<Vec<f64>> = inputs.weights.map(|w| {
        inputs
            .labels
            .iter()
            .map(|l| w.get(l).copied().unwrap_or(0.0))
            .collect()
    });
    let variance = |w: &[f64]| -> f64 {
        (0..n)
            .map(|i| (0..n).map(|j| w[i] * cov(i, j) * w[j]).sum::<f64>())
            .sum()
    };
    let mut allocation = Vec::with_capacity(n);
    let mut summary = None;
    let shortfall = inputs
        .shortfall
        .as_ref()
        .filter(|s| s.contributions.len() == n);
    if let Some(w) = &weights {
        let var = variance(w);
        for i in 0..n {
            let marginal: f64 = (0..n).map(|j| cov(i, j) * w[j]).sum();
            let risk_share = match shortfall {
                Some(s) => s.contributions[i].share.unwrap_or(0.0),
                None if var > 0.0 => w[i] * marginal / var,
                None => 0.0,
            };
            allocation.push(AllocationRow {
                asset: inputs.labels[i].clone(),
                weight: w[i],
                expected_return: inputs.expected_returns[i],
                volatility: vol[i],
                risk_share,
            });
        }
        allocation.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        let expected: f64 = (0..n).map(|i| w[i] * inputs.expected_returns[i]).sum();
        let volatility = var.max(0.0).sqrt();
        let largest = allocation.first();
        summary = Some(OptimisationSummary {
            expected_return: expected,
            volatility,
            sharpe_ratio: (volatility > 0.0)
                .then(|| (expected - inputs.risk_free_rate) / volatility),
            diversification_ratio: (volatility > 0.0)
                .then(|| (0..n).map(|i| w[i] * vol[i]).sum::<f64>() / volatility),
            effective_assets: 1.0 / w.iter().map(|x| x * x).sum::<f64>().max(f64::MIN_POSITIVE),
            largest_asset: largest.map(|r| r.asset.clone()).unwrap_or_default(),
            largest_weight: largest.map_or(0.0, |r| r.weight),
            expected_shortfall: shortfall.map(|s| ShortfallFigure {
                confidence: s.confidence,
                value: s.expected_shortfall,
            }),
        });
    } else {
        for ((asset, expected), volatility) in
            inputs.labels.iter().zip(inputs.expected_returns).zip(&vol)
        {
            allocation.push(AllocationRow {
                asset: asset.clone(),
                weight: 0.0,
                expected_return: *expected,
                volatility: *volatility,
                risk_share: 0.0,
            });
        }
    }
    let groups = inputs
        .groups
        .iter()
        .map(|(name, assets, min, max)| {
            let of = |pick: fn(&AllocationRow) -> f64| -> f64 {
                allocation
                    .iter()
                    .filter(|a| assets.contains(&a.asset))
                    .map(pick)
                    .sum()
            };
            GroupRow {
                name: name.clone(),
                assets: assets.clone(),
                weight: of(|a| a.weight),
                min: *min,
                max: *max,
                risk_share: of(|a| a.risk_share),
            }
        })
        .collect();
    let correlation = Grid {
        name: "Correlation".to_owned(),
        unit: Unit::Ratio,
        rows: inputs.labels.to_vec(),
        columns: inputs.labels.to_vec(),
        values: (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let d = vol[i] * vol[j];
                        (d > 0.0).then(|| cov(i, j) / d)
                    })
                    .collect()
            })
            .collect(),
    };
    let equal_weight = (n > 0).then(|| {
        let w = vec![1.0 / n as f64; n];
        RiskReturn {
            volatility: variance(&w).max(0.0).sqrt(),
            expected_return: inputs.expected_returns.iter().sum::<f64>() / n as f64,
        }
    });
    let mut hash = Sha256::new();
    for (l, r) in inputs.labels.iter().zip(inputs.expected_returns) {
        hash.update(l.as_bytes());
        hash.update(r.to_le_bytes());
    }
    for v in inputs.covariance.values() {
        hash.update(v.to_le_bytes());
    }
    hash.update(inputs.method.as_bytes());
    for l in &inputs.limits {
        hash.update(l.as_bytes());
    }
    hash.update(inputs.risk_free_rate.to_le_bytes());
    let digest = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(OptimisationReport {
        context: OptimisationContext {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            digest,
            method: inputs.method,
            description: inputs.description,
            status: inputs.status,
            solver: inputs.solver,
            iterations: inputs.iterations,
            assets: inputs.labels.to_vec(),
            risk_free_asset: inputs.risk_free_asset,
            risk_free_rate: inputs.risk_free_rate,
            periods_per_year: inputs.periods_per_year,
            covariance: inputs.covariance_method,
            mean_returns: inputs.mean_returns,
            limits: inputs.limits,
            start: inputs.start,
            end: inputs.end,
            resampling: inputs.resampling,
        },
        summary,
        allocation,
        groups,
        correlation,
        frontier: inputs
            .frontier
            .into_iter()
            .map(|(volatility, expected_return)| RiskReturn {
                volatility,
                expected_return,
            })
            .collect(),
        equal_weight,
        history: inputs
            .history
            .map(|(weights, equal_weight)| OptimisationHistory {
                weights,
                equal_weight,
            }),
        name: None,
    })
}

// ── Words ────────────────────────────────────────────────────────────────────────────────────

/// The most held assets the text report lists one by one; the rest are shown together.
const HELD_LISTED: usize = 20;

fn weight(v: f64) -> String {
    crate::text::weight_pct(v, 1)
}

/// The assets held, the largest first.
fn held(r: &OptimisationReport) -> Vec<&AllocationRow> {
    r.allocation.iter().filter(|a| a.weight >= NOISE).collect()
}

/// The assets not held, as a phrase, if any.
fn not_held(r: &OptimisationReport) -> Option<String> {
    let names: Vec<String> = r
        .allocation
        .iter()
        .filter(|a| a.weight < NOISE)
        .map(|a| a.asset.clone())
        .collect();
    match names.len() {
        0 => None,
        1 => Some(format!("{} is not held", names[0])),
        2..=8 => Some(format!(
            "{} are not held",
            crate::backtest_html::and_list(&names)
        )),
        n => Some(format!("{n} assets are not held")),
    }
}

/// Why an optimisation with `status` (as JSON writes it, such as `infeasible`) found no weights.
fn no_weights(status: &str) -> String {
    serde_json::from_value::<finalytics_opt::SolveStatus>(serde_json::Value::String(
        status.to_owned(),
    ))
    .map_or_else(|_| format!("it ended {status}"), |s| s.reason().to_owned())
}

fn limits(c: &OptimisationContext) -> String {
    if c.limits.is_empty() {
        "fully invested and long only".to_owned()
    } else {
        let limits: Vec<String> = c
            .limits
            .iter()
            .map(|l| crate::backtest_html::limit_in_sentence(l))
            .collect();
        limits.join("; ")
    }
}

/// What the shares of risk are of: the variance, or for minimum CVaR the expected shortfall.
fn risk_basis(r: &OptimisationReport) -> String {
    match r.summary.as_ref().and_then(|s| s.expected_shortfall) {
        Some(es) => format!(
            "the expected shortfall at {}: an asset's weight times its mean loss in the worst \
             {} of {}s, over the portfolio's",
            pct(Some(es.confidence), 0, false),
            pct(Some(1.0 - es.confidence), 0, false),
            period_word(r.context.periods_per_year)
        ),
        None => "the variance: an asset's weight times its covariance with the portfolio, over \
                 the portfolio's variance"
            .to_owned(),
    }
}

/// The optimal expected shortfall as a label and a value, such as ("CVaR 95%", "2.3% a month").
fn shortfall_figure(r: &OptimisationReport) -> Option<(String, String)> {
    let es = r.summary.as_ref()?.expected_shortfall?;
    Some((
        format!("CVaR {}", pct(Some(es.confidence), 0, false)),
        format!(
            "{} a {}",
            pct(Some(es.value), 2, false),
            period_word(r.context.periods_per_year)
        ),
    ))
}

/// The groups as table rows: name, weight, limits and share of risk.
fn group_rows(r: &OptimisationReport) -> Vec<Vec<String>> {
    r.groups
        .iter()
        .map(|g| {
            vec![
                g.name.clone(),
                share(g.weight),
                format!("{} to {}", share(g.min), share(g.max)),
                weight(g.risk_share),
            ]
        })
        .collect()
}

fn estimates(c: &OptimisationContext) -> String {
    format!(
        "{} expected returns and {}, annualised by {}",
        match &c.mean_returns {
            Some(method) => format!("{} mean", capitalise(mean_label(method))),
            None => "The given".to_owned(),
        },
        covariance_name(&c.covariance),
        num(Some(c.periods_per_year), 0)
    )
}

// ── The page ─────────────────────────────────────────────────────────────────────────────────

impl OptimisationReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own, or
    /// the method's): the weights and what they are expected to deliver, where the risk comes
    /// from, where the weights sit against each asset and on the efficient frontier (when the
    /// report has it), the history the weights were fitted to, and the method.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let title = name.or(self.name.as_deref()).unwrap_or(&c.method);
        let rf_name = c.risk_free_asset.as_deref().unwrap_or("the risk-free rate");
        let held = held(self);
        let colour = |asset: &str| -> &str {
            held.iter()
                .position(|a| a.asset == asset)
                .and_then(|k| SERIES.get(k).copied())
                .unwrap_or("var(--muted-fill)")
        };
        let mut body = String::new();

        // Summary.
        let mut summary = String::new();
        match &self.summary {
            Some(s) => {
                let mut items = vec![
                    (
                        "Expected return".to_owned(),
                        format!("{} a year", percent(s.expected_return)),
                        Some(match &c.mean_returns {
                            Some(method) => {
                                format!("{} mean returns", capitalise(mean_label(method)))
                            }
                            None => "Given expected returns".to_owned(),
                        }),
                    ),
                    (
                        "Volatility".to_owned(),
                        percent(s.volatility),
                        Some(capitalise(&covariance_name(&c.covariance))),
                    ),
                ];
                if let (Some((label, value)), Some(es)) =
                    (shortfall_figure(self), s.expected_shortfall)
                {
                    items.push((
                        label,
                        value,
                        Some(format!(
                            "The least mean loss in the worst {} of {}s",
                            pct(Some(1.0 - es.confidence), 0, false),
                            period_word(c.periods_per_year)
                        )),
                    ));
                }
                items.extend([
                    (
                        "Sharpe ratio".to_owned(),
                        num(s.sharpe_ratio, 2),
                        Some(format!(
                            "Against {rf_name} at {}",
                            percent(c.risk_free_rate)
                        )),
                    ),
                    (
                        "Diversification ratio".to_owned(),
                        num(s.diversification_ratio, 2),
                        Some(format!(
                            "{} assets in effect",
                            num(Some(s.effective_assets), 1)
                        )),
                    ),
                    (
                        "Largest weight".to_owned(),
                        percent(s.largest_weight),
                        Some(s.largest_asset.clone()),
                    ),
                ]);
                summary.push_str(&tiles(&items));
                let mut segs = String::new();
                let mut keys: Vec<(String, &str)> = Vec::new();
                for a in &held {
                    let inside = if a.weight >= 0.07 {
                        pct(Some(a.weight), 0, false)
                    } else {
                        String::new()
                    };
                    let _ = write!(
                        segs,
                        r#"<div class="seg" style="flex:{:.4};background:{}" title="{}: {}"><span>{inside}</span></div>"#,
                        a.weight,
                        colour(&a.asset),
                        esc(&a.asset),
                        esc(&percent(a.weight))
                    );
                    keys.push((
                        format!("{} {}", a.asset, percent(a.weight)),
                        colour(&a.asset),
                    ));
                }
                let rows: Vec<Vec<String>> = self
                    .allocation
                    .iter()
                    .map(|a| {
                        vec![
                            if a.weight >= NOISE {
                                key(&a.asset, colour(&a.asset))
                            } else {
                                esc(&a.asset)
                            },
                            esc(&weight(a.weight)),
                            esc(&percent(a.expected_return)),
                            esc(&percent(a.volatility)),
                            esc(&weight(a.risk_share)),
                        ]
                    })
                    .collect();
                let mut note = format!(
                    "{}. {}. Shares of risk are of {}.",
                    capitalise(&limits(c)),
                    estimates(c),
                    risk_basis(self)
                );
                if let Some(n) = not_held(self) {
                    let _ = write!(note, " {n}.");
                }
                summary.push_str(&card(
                    "The weights",
                    &format!(
                        r#"<div class="segbar" role="img" aria-label="The weights">{segs}</div>{}{}"#,
                        legend(&keys),
                        table(
                            &[
                                "Asset",
                                "Weight",
                                "Expected return",
                                "Volatility",
                                "Share of risk",
                            ]
                            .map(str::to_owned),
                            &rows,
                            1,
                            "cmp",
                        )
                    ),
                    Some(&note),
                ));
                if !self.groups.is_empty() {
                    let rows: Vec<Vec<String>> = group_rows(self)
                        .into_iter()
                        .map(|row| row.iter().map(|cell| esc(cell)).collect())
                        .collect();
                    summary.push_str(&card(
                        "Groups",
                        &table(
                            &["Group", "Weight", "Limits", "Share of risk"].map(str::to_owned),
                            &rows,
                            1,
                            "",
                        ),
                        Some("Each group's total weight against its limits, and its share of the risk."),
                    ));
                }
            }
            None => summary.push_str(&card(
                "No weights",
                &format!(
                    "<p>The optimisation found no weights: {}. The limits: {}.</p>",
                    esc(&no_weights(&c.status)),
                    esc(&limits(c))
                ),
                None,
            )),
        }
        body.push_str(&section(
            "p-summary",
            "Summary",
            "The weights to hold, and what they are expected to deliver",
            &summary,
        ));

        // Risk.
        let mut risk = String::new();
        if self.summary.is_some() {
            let scale = held
                .iter()
                .map(|a| a.weight.max(a.risk_share))
                .fold(0.0, f64::max)
                .max(1e-12);
            let (w, row, x0, x1) = (880.0, 34.0, 140.0, 820.0);
            let h = row * held.len() as f64 + 16.0;
            let mut svg = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
            for (k, a) in held.iter().enumerate() {
                let y = 8.0 + k as f64 * row;
                let _ = write!(
                    svg,
                    r#"<text class="lab" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
                    x0 - 10.0,
                    y + 15.0,
                    esc(&a.asset)
                );
                for (j, (v, c, label)) in [
                    (a.weight, "var(--s1)", "Weight"),
                    (a.risk_share, "var(--s2)", "Share of risk"),
                ]
                .into_iter()
                .enumerate()
                {
                    let bw = ((x1 - x0) * v / scale).max(0.0);
                    let yy = y + j as f64 * 12.0;
                    let _ = write!(
                        svg,
                        r#"<g><title>{} · {label}: {}</title><rect x="{x0}" y="{yy:.1}" width="{bw:.1}" height="10" rx="3" style="fill:{c}"/><text class="val" x="{:.1}" y="{:.1}">{}</text></g>"#,
                        esc(&a.asset),
                        esc(&percent(v)),
                        x0 + bw + 6.0,
                        yy + 9.0,
                        esc(&percent(v))
                    );
                }
            }
            let _ = write!(
                svg,
                r#"<line class="base" x1="{x0}" x2="{x0}" y1="0" y2="{}"/></svg>"#,
                h - 6.0
            );
            let mut note = String::new();
            let most = held
                .iter()
                .max_by(|a, b| (a.risk_share - a.weight).total_cmp(&(b.risk_share - b.weight)));
            let least = held
                .iter()
                .min_by(|a, b| (a.risk_share - a.weight).total_cmp(&(b.risk_share - b.weight)));
            // At a minimum-variance optimum, every asset held off its bounds carries as much of
            // the risk as of the weight, so there is no contrast to point out.
            let gap = |a: &&&AllocationRow| a.risk_share - a.weight;
            match (most, least) {
                (Some(m), Some(l))
                    if m.asset != l.asset && gap(&m).abs().max(gap(&l).abs()) >= 0.005 =>
                {
                    let _ = write!(
                        note,
                        "{} is {} of the weight but {} of the risk; {} is {} of the weight and {} of the risk. ",
                        m.asset,
                        share(m.weight),
                        share(m.risk_share),
                        l.asset,
                        share(l.weight),
                        share(l.risk_share)
                    );
                }
                (Some(_), Some(_)) => note.push_str(
                    "Each asset's share of the risk is its weight, to within half a percentage \
                     point. ",
                ),
                _ => {}
            }
            let _ = write!(note, "Shares of risk are of {}.", risk_basis(self));
            if let Some(n) = not_held(self) {
                let _ = write!(note, " {n}.");
            }
            risk.push_str(&card(
                "Where the risk comes from",
                &format!(
                    "{}{svg}",
                    legend(&[
                        ("Weight".to_owned(), "var(--s1)"),
                        ("Share of risk".to_owned(), "var(--s2)")
                    ])
                ),
                Some(&note),
            ));
        }
        let mut cells = String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
        let shown = self.correlation.rows.len() <= 12;
        if shown {
            for col in &self.correlation.columns {
                let _ = write!(cells, "<th>{}</th>", esc(col));
            }
            cells.push_str("</tr></thead><tbody>");
            for (i, (row, values)) in self
                .correlation
                .rows
                .iter()
                .zip(&self.correlation.values)
                .enumerate()
            {
                let _ = write!(cells, "<tr><th>{}</th>", esc(row));
                for (j, v) in values.iter().enumerate() {
                    let text = num(*v, 2);
                    if i == j {
                        let _ = write!(cells, r#"<td class="diag">{}</td>"#, esc(&text));
                    } else {
                        let _ = write!(cells, "<td {}>{}</td>", heat(*v, 1.0), esc(&text));
                    }
                }
                cells.push_str("</tr>");
            }
            cells.push_str("</tbody></table></div>");
            risk.push_str(&card(
                "Correlation",
                &cells,
                Some(&format!(
                    "From the covariance of the {} returns, {}.",
                    frequency(c.periods_per_year),
                    span(c.start, c.end)
                )),
            ));
        }
        body.push_str(&section(
            "p-risk",
            "Risk",
            "Where the risk comes from, and how the assets move together",
            &risk,
        ));

        // Risk and return: the weights against each asset and equal weight, and the frontier
        // when the optimisation asked for it.
        if let Some(s) = &self.summary {
            let line = format!(
                "The dashed line starts at {rf_name}, {}, and passes through the weights; its \
                 slope is their Sharpe ratio.",
                percent(c.risk_free_rate)
            );
            let (subtitle, title, note) = if self.frontier.len() > 1 {
                (
                    "Where the weights sit among the portfolios the limits allow",
                    "The efficient frontier",
                    format!(
                        "The frontier holds the portfolios with the largest expected return for \
                         each volatility, under the same limits. {line}"
                    ),
                )
            } else {
                (
                    "Where the weights sit against each asset and equal weight",
                    "Expected return and volatility",
                    format!(
                        "{line} To draw the efficient frontier, optimise with the frontier \
                         turned on: frontier=True in Python, frontier: true in JavaScript or \
                         .with_frontier() in Rust. It takes longer, more so with many assets."
                    ),
                )
            };
            body.push_str(&section(
                "p-risk-return",
                "Risk and return",
                subtitle,
                &card(title, &self.frontier_chart(s, &colour), Some(&note)),
            ));
        }

        // History.
        if let Some(h) = &self.history {
            let series = [
                Line {
                    name: "The weights",
                    colour: "var(--s1)",
                    series: &h.weights.growth,
                },
                Line {
                    name: "Equal weight",
                    colour: BENCHMARK,
                    series: &h.equal_weight.growth,
                },
            ];
            let w = &h.weights.statistics;
            let e = &h.equal_weight.statistics;
            let body_html = lines(&series, &level, &level_tick, 260.0, false, None, None)
                + &tiles(&[
                    (
                        "Return".to_owned(),
                        format!("{} a year", percent(w.annualised_return)),
                        Some(format!("Equal weight {}", percent(e.annualised_return))),
                    ),
                    (
                        "Volatility".to_owned(),
                        percent(w.volatility),
                        Some(format!("Equal weight {}", percent(e.volatility))),
                    ),
                    (
                        "Sharpe ratio".to_owned(),
                        num(w.sharpe_ratio, 2),
                        Some(format!("Equal weight {}", num(e.sharpe_ratio, 2))),
                    ),
                    (
                        "Maximum drawdown".to_owned(),
                        percent(w.max_drawdown),
                        Some(format!("Equal weight {}", percent(e.max_drawdown))),
                    ),
                ]);
            body.push_str(&section(
                "p-history",
                "History",
                "How the weights would have done on the returns they were fitted to",
                &card(
                    "On the history it was fitted to",
                    &body_html,
                    Some(
                        "In sample: the weights were chosen knowing these returns, so this \
                         flatters them. Held from the start and rebalanced quarterly, without \
                         trading costs. To test the method on returns it had not seen, backtest it \
                         with a holdout or walk-forward validation.",
                    ),
                ),
            ));
        }

        // Method.
        body.push_str(&section(
            "p-method",
            "Method",
            "How the weights were found",
            &card(
                "How the weights were found",
                &notes_table(&self.method_rows()),
                None,
            ),
        ));

        let meta = format!(
            "{} · {} · {}<br>{} · {}, {} · {} covariance{}",
            esc(&c.method),
            esc(&limits(c)),
            esc(&c.status),
            esc(&plural(c.assets.len(), "asset")),
            frequency(c.periods_per_year),
            esc(&span(c.start, c.end)),
            esc(&capitalise(covariance_label(&c.covariance))),
            c.risk_free_asset
                .as_ref()
                .map_or_else(String::new, |a| format!(" · risk-free {}", esc(a)))
        );
        let mut nav = vec![("p-summary", "Summary"), ("p-risk", "Risk")];
        if self.summary.is_some() {
            nav.push(("p-risk-return", "Risk and return"));
        }
        if self.history.is_some() {
            nav.push(("p-history", "History"));
        }
        nav.push(("p-method", "Method"));
        frame(
            &Frame {
                title,
                brand: Some("Optimisation report"),
                meta: &meta,
                nav: &nav,
                version: &c.version,
                digest: &c.digest,
                source: "the assets' returns",
            },
            &body,
        )
    }

    fn method_rows(&self) -> Vec<(&'static str, String)> {
        let c = &self.context;
        let assets = if c.assets.len() <= 8 {
            crate::backtest_html::and_list(&c.assets)
        } else {
            plural(c.assets.len(), "asset")
        };
        let cash = c
            .risk_free_asset
            .as_ref()
            .map_or_else(String::new, |a| format!("; {a} as the risk-free asset"));
        let mut rows = vec![
            ("Method", format!("{}: {}", c.method, c.description)),
            (
                "Data",
                format!(
                    "{assets}{cash}. {} returns, {}.",
                    capitalise(frequency(c.periods_per_year)),
                    span(c.start, c.end)
                ),
            ),
            ("Estimates", format!("{}.", estimates(c))),
            ("Limits", format!("{}.", capitalise(&limits(c)))),
        ];
        if let Some(r) = &c.resampling {
            rows.push((
                "Resampling",
                format!(
                    "The average of the optimal weights for {} histories of {} {} returns \
                     simulated from the estimates (Michaud, 1998); {} of them solved.",
                    r.samples,
                    r.observations,
                    frequency(c.periods_per_year),
                    r.solved
                ),
            ));
        }
        rows.push((
            "Solver",
            format!(
                "{}, {} after {}.",
                capitalise(&c.solver.replace('_', " ")),
                c.status,
                plural(c.iterations as usize, "iteration")
            ),
        ));
        rows
    }

    /// The frontier, the assets, equal weight and the weights, with the line from the risk-free
    /// rate through the weights.
    fn frontier_chart(
        &self,
        s: &OptimisationSummary,
        colour: &dyn Fn(&str) -> &'static str,
    ) -> String {
        let c = &self.context;
        let (w, h, l, r, t, b) = (880.0, 340.0, 56.0, 150.0, 24.0, 40.0);
        let mut points: Vec<(f64, f64)> = self
            .frontier
            .iter()
            .map(|p| (p.volatility, p.expected_return))
            .collect();
        points.extend(
            self.allocation
                .iter()
                .map(|a| (a.volatility, a.expected_return)),
        );
        points.extend(self.equal_weight.map(|e| (e.volatility, e.expected_return)));
        points.push((s.volatility, s.expected_return));
        let max_x = points.iter().map(|p| p.0).fold(0.0, f64::max) * 1.05;
        let min_y = points.iter().map(|p| p.1).fold(0.0, f64::min);
        let max_y = points.iter().map(|p| p.1).fold(c.risk_free_rate, f64::max) * 1.08;
        let gx = ticks(0.0, max_x.max(1e-6), 5);
        let gy = ticks(min_y, max_y.max(1e-6), 5);
        let (x0, x1) = (gx[0], gx.last().copied().unwrap_or(1.0));
        let (y0, y1) = (gy[0], gy.last().copied().unwrap_or(1.0));
        let x = |v: f64| l + (w - l - r) * (v - x0) / (x1 - x0).max(1e-12);
        let y = |v: f64| t + (h - t - b) * (y1 - v) / (y1 - y0).max(1e-12);
        let ystep = gy.get(1).map_or(1.0, |g| g - gy[0]);
        let yplaces = usize::from(((ystep * 100.0) - (ystep * 100.0).round()).abs() > 1e-9);
        let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
        for v in &gy {
            let _ = write!(
                out,
                r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
                w - r,
                y(*v),
                y(*v),
                l - 8.0,
                y(*v) + 4.0,
                esc(&pct(Some(*v), yplaces, false))
            );
        }
        for v in &gx {
            let _ = write!(
                out,
                r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
                x(*v),
                h - 20.0,
                esc(&pct(Some(*v), 0, false))
            );
        }
        let _ = write!(
            out,
            r#"<text class="tick" x="{:.0}" y="{}" text-anchor="middle">Volatility, a year</text><text class="tick" x="{}" y="{}">Expected return, a year</text>"#,
            (l + w - r) / 2.0,
            h - 2.0,
            l - 44.0,
            t - 10.0
        );
        // The line from the risk-free rate through the weights, to the edge of the chart.
        if let Some(sharpe) = s.sharpe_ratio.filter(|v| *v > 0.0) {
            let rf = c.risk_free_rate;
            let xe = x1.min((y1 - rf) / sharpe);
            let _ = write!(
                out,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" style="stroke:var(--ink2);stroke-dasharray:4 4;stroke-width:1.2"/>"#,
                x(0.0),
                y(rf),
                x(xe),
                y(rf + sharpe * xe)
            );
        }
        if self.frontier.len() > 1 {
            let path: Vec<String> = self
                .frontier
                .iter()
                .map(|p| format!("{:.1},{:.1}", x(p.volatility), y(p.expected_return)))
                .collect();
            let _ = write!(
                out,
                r#"<path class="line" d="M{}" style="stroke:var(--ink2)"/>"#,
                path.join(" L")
            );
        }
        // The assets, labelled, their labels kept apart.
        let mut order: Vec<&AllocationRow> = self.allocation.iter().collect();
        order.sort_by(|a, b| y(a.expected_return).total_cmp(&y(b.expected_return)));
        let mut placed: Vec<(f64, f64)> = Vec::new();
        let label_all = order.len() <= 12;
        for a in order {
            let (cx, cy) = (x(a.volatility), y(a.expected_return));
            let fill = if a.weight >= NOISE {
                colour(&a.asset)
            } else {
                "var(--muted-fill)"
            };
            let _ = write!(
                out,
                r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="5" style="fill:{fill}"><title>{}: return {}, volatility {}</title></circle>"#,
                esc(&a.asset),
                esc(&percent(a.expected_return)),
                esc(&percent(a.volatility))
            );
            if label_all || a.weight >= NOISE {
                let (lx, mut ly) = (cx + 9.0, cy + 4.0);
                while placed
                    .iter()
                    .any(|(px, py)| (ly - py).abs() < 14.0 && (lx - px).abs() < 90.0)
                {
                    ly += 14.0;
                }
                placed.push((lx, ly));
                let _ = write!(
                    out,
                    r#"<text class="lab" x="{lx:.1}" y="{ly:.1}">{}</text>"#,
                    esc(&a.asset)
                );
            }
        }
        if let Some(e) = self.equal_weight {
            let _ = write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="6" style="fill:var(--surface);stroke:var(--ink2);stroke-width:2"><title>Equal weight: return {}, volatility {}</title></circle><text class="lab" x="{:.1}" y="{:.1}">Equal weight</text>"#,
                x(e.volatility),
                y(e.expected_return),
                esc(&percent(e.expected_return)),
                esc(&percent(e.volatility)),
                x(e.volatility) + 10.0,
                y(e.expected_return) + 16.0
            );
        }
        let _ = write!(
            out,
            r#"<circle cx="{:.1}" cy="{:.1}" r="8" style="fill:var(--s1);stroke:var(--surface);stroke-width:2"><title>The weights: return {}, volatility {}</title></circle><text class="lab" x="{:.1}" y="{:.1}" text-anchor="end" style="font-weight:600">The weights</text></svg>"#,
            x(s.volatility),
            y(s.expected_return),
            esc(&percent(s.expected_return)),
            esc(&percent(s.volatility)),
            x(s.volatility) - 12.0,
            y(s.expected_return) - 12.0
        );
        let keys = format!(
            r#"<div class="legend">{}<span class="key"><i class="hollow"></i>Equal weight</span>{}</div>"#,
            key("The weights", "var(--s1)"),
            key("Each asset", "var(--muted-fill)")
        );
        keys + &out
    }
}

// ── The text ─────────────────────────────────────────────────────────────────────────────────

impl OptimisationReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own) if given.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let name = name.or(self.name.as_deref());
        let mut page = Page::new(None);
        page.rule('━');
        let left = "FINALYTICS  ·  OPTIMISATION";
        page.line(format!(
            "{left}{}",
            pad_left(&span(c.start, c.end), WIDTH.saturating_sub(width(left)))
        ));
        if let Some(name) = name {
            page.line(name);
        }
        page.line(format!("{} · {} · {}", c.method, limits(c), c.status));
        page.line(format!(
            "{} · {} · {} covariance{}",
            plural(c.assets.len(), "asset"),
            frequency(c.periods_per_year),
            capitalise(covariance_label(&c.covariance)),
            c.risk_free_asset
                .as_ref()
                .map_or_else(String::new, |a| format!(" · {a} as cash"))
        ));
        page.rule('━');
        match &self.summary {
            Some(s) => {
                page.section("Weights");
                let largest = self
                    .allocation
                    .iter()
                    .map(|a| a.weight)
                    .fold(0.0, f64::max)
                    .max(1e-12);
                // The assets held, largest first; when there are many, the smaller ones
                // together.
                let held = held(self);
                let listed = if held.len() > HELD_LISTED + 1 {
                    HELD_LISTED
                } else {
                    held.len()
                };
                let mut rows: Vec<Vec<String>> = held
                    .iter()
                    .take(listed)
                    .map(|a| {
                        vec![
                            a.asset.clone(),
                            weight(a.weight),
                            bar(a.weight / largest, 20).trim_end().to_owned(),
                            pct(Some(a.expected_return), 1, false),
                            pct(Some(a.volatility), 1, false),
                            weight(a.risk_share),
                        ]
                    })
                    .collect();
                if held.len() > listed {
                    let rest = &held[listed..];
                    let total =
                        |f: fn(&AllocationRow) -> f64| rest.iter().map(|a| f(a)).sum::<f64>();
                    let w = total(|a| a.weight);
                    rows.push(vec![
                        format!("{} more held", rest.len()),
                        weight(w),
                        bar(w / largest, 20).trim_end().to_owned(),
                        String::new(),
                        String::new(),
                        weight(total(|a| a.risk_share)),
                    ]);
                }
                page.table(
                    &["Asset", "Weight", "", "Return", "Volatility", "Risk"],
                    &rows,
                    Some(&[L, R, L, R, R, R]),
                );
                page.blank();
                let mut note = format!(
                    "Return and volatility a year; risk is the share of {}.",
                    match s.expected_shortfall {
                        Some(es) => format!(
                            "the expected shortfall at {}",
                            pct(Some(es.confidence), 0, false)
                        ),
                        None => "the portfolio's variance".to_owned(),
                    }
                );
                if let Some(n) = not_held(self) {
                    let _ = write!(note, " {n}.");
                }
                page.note(&note);
                if !self.groups.is_empty() {
                    page.section("Groups");
                    page.table(
                        &["Group", "Weight", "Limits", "Risk"],
                        &group_rows(self),
                        Some(&[L, R, R, R]),
                    );
                }
                page.section("Expected");
                let mut pairs = vec![
                    ("Expected return".to_owned(), percent(s.expected_return)),
                    ("Volatility".to_owned(), percent(s.volatility)),
                ];
                pairs.extend(shortfall_figure(self));
                pairs.extend([
                    ("Sharpe ratio".to_owned(), num(s.sharpe_ratio, 2)),
                    (
                        "Diversification ratio".to_owned(),
                        num(s.diversification_ratio, 2),
                    ),
                    (
                        "Assets in effect".to_owned(),
                        num(Some(s.effective_assets), 1),
                    ),
                    (
                        "Largest weight".to_owned(),
                        format!(
                            "{} {}",
                            percent(s.largest_weight),
                            fit(&s.largest_asset, 14)
                        ),
                    ),
                ]);
                page.pairs(&pairs);
            }
            None => {
                page.section("No weights");
                page.note(&format!(
                    "The optimisation found no weights: {}. The limits: {}.",
                    no_weights(&c.status),
                    limits(c)
                ));
            }
        }
        if self.correlation.rows.len() <= 6 {
            page.section("Correlation");
            let short: Vec<String> = self.correlation.columns.iter().map(|c| fit(c, 9)).collect();
            let head: Vec<&str> = std::iter::once("")
                .chain(short.iter().map(String::as_str))
                .collect();
            let rows: Vec<Vec<String>> = self
                .correlation
                .rows
                .iter()
                .zip(&self.correlation.values)
                .map(|(r, values)| {
                    std::iter::once(r.clone())
                        .chain(values.iter().map(|v| num(*v, 2)))
                        .collect()
                })
                .collect();
            page.table(&head, &rows, None);
        }
        if let Some(h) = &self.history {
            page.section("On the history it was fitted to (in sample)");
            let w = &h.weights;
            let e = &h.equal_weight;
            let cells = 46;
            for (label, run) in [("The weights", w), ("Equal weight", e)] {
                page.line(format!(
                    "  {}  {}  {}",
                    crate::text::pad_right(label, 13),
                    spark(&run.growth.values, cells),
                    pad_left(&level(*run.growth.values.last().unwrap_or(&100.0)), 6)
                ));
            }
            page.blank();
            page.note(&format!(
                "Return {} a year, volatility {}, Sharpe ratio {}, worst fall {}. In sample, so \
                 it flatters the weights: a backtest tests them on returns they had not seen.",
                percent(w.statistics.annualised_return),
                percent(w.statistics.volatility),
                num(w.statistics.sharpe_ratio, 2),
                percent(w.statistics.max_drawdown)
            ));
        }
        page.section("How the weights were found");
        let rows = self.method_rows();
        let lw = rows.iter().map(|(l, _)| width(l)).max().unwrap_or(0);
        for (label, text) in &rows {
            for (k, line) in wrap(text, WIDTH - 4 - lw).into_iter().enumerate() {
                let label = if k == 0 { *label } else { "" };
                page.line(format!("  {}  {line}", crate::text::pad_right(label, lw)));
            }
        }
        page.blank();
        page.line(format!(
            "  Finalytics {} · inputs {}",
            c.version,
            c.digest.chars().take(12).collect::<String>()
        ));
        page.finish()
    }
}

impl fmt::Display for OptimisationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}

/// Weights and their figures as text, for the optimisation's raw results.
#[doc(hidden)]
pub fn weights_text(
    title: &str,
    weights: &BTreeMap<String, f64>,
    figures: &[(String, String)],
) -> String {
    let mut parts: Vec<(&String, f64)> = weights.iter().map(|(a, w)| (a, *w)).collect();
    parts.sort_by(|a, b| b.1.total_cmp(&a.1));
    let lw = parts
        .iter()
        .map(|(a, _)| width(a).min(24))
        .max()
        .unwrap_or(0);
    let mut page = Page::new(None);
    page.line(title);
    page.blank();
    for (asset, w) in parts {
        page.line(format!(
            "  {}  {}",
            crate::text::pad_right(&fit(asset, 24), lw),
            pad_left(&weight(w), 6)
        ));
    }
    if !figures.is_empty() {
        page.blank();
        let line: Vec<String> = figures.iter().map(|(k, v)| format!("{k} {v}")).collect();
        for l in wrap(&line.join(" · "), WIDTH - 2) {
            page.line(format!("  {l}"));
        }
    }
    page.finish()
}
