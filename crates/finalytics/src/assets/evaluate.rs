//! Given weights valued on the assets' estimates: the figures and the report an optimisation
//! gives the weights it chooses, for weights chosen some other way, such as a portfolio's own.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};
use finalytics_opt::Bound;
use finalytics_reports::OptimisationReport;

use super::{AssetGroup, Assets, CASH, FRONTIER_POINTS, Method, OptimiseOptions};

/// The options valuing weights takes: the estimates', and the limits of the frontier.
const TAKES: [&str; 8] = [
    "risk_free_rate",
    "expected_returns",
    "max_weight",
    "bounds",
    "asset_bounds",
    "groups",
    "min_effective_assets",
    "cash",
];

/// Given weights valued on the assets' estimates: the report, laid out as an optimisation's,
/// and the raw figures. Printing it prints the report, and showing it shows the report as a
/// page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct GivenWeights {
    /// The report on the weights: what they are expected to deliver, where their risk comes
    /// from, where they sit against each asset (and on the efficient frontier, if asked for)
    /// and how they would have done on the assets' history.
    pub report: OptimisationReport,
    /// The raw figures.
    pub results: Evaluation,
}

impl std::fmt::Display for GivenWeights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

/// The figures of given weights on the assets' estimates: those an optimisation gives the
/// weights it chooses, from the same covariance and expected returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Evaluation {
    /// The weights by asset, as given.
    pub weights: BTreeMap<String, f64>,
    /// The annual expected return.
    pub expected_return: f64,
    /// The annual volatility.
    pub volatility: f64,
    /// `(expected_return - risk_free_rate) / volatility`; absent when the weights are
    /// riskless.
    pub sharpe_ratio: Option<f64>,
    /// The annual risk-free rate of the Sharpe ratio.
    pub risk_free_rate: f64,
    /// Each asset's share of the variance, `w_i (Σw)_i / w'Σw`, adding up to one; zero for
    /// every asset when the weights are riskless.
    pub risk_shares: BTreeMap<String, f64>,
}

/// The figures as text: each weight, the largest first, with the expected return, volatility
/// and Sharpe ratio.
impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pct = |v: f64| format!("{:.1}%", v * 100.0).replace('-', "−");
        let figures = vec![
            ("Expected return".to_owned(), pct(self.expected_return)),
            ("volatility".to_owned(), pct(self.volatility)),
            (
                "Sharpe ratio".to_owned(),
                self.sharpe_ratio
                    .map_or_else(|| "–".to_owned(), |v| format!("{v:.2}").replace('-', "−")),
            ),
            ("risk-free".to_owned(), pct(self.risk_free_rate)),
        ];
        f.write_str(&finalytics_reports::weights_text(
            "Evaluation · Given weights",
            &self.weights,
            &figures,
        ))
    }
}

impl Assets {
    /// Given weights valued on the estimates optimisation uses, built step by step;
    /// [`Evaluate::build`] values them. The weights are by asset and add up to one; they may
    /// hold the risk-free asset and other cash, such as a portfolio's own weights, which
    /// [`Portfolio::weights`](crate::Portfolio::weights) gives:
    ///
    /// ```
    /// use finalytics::datasets;
    ///
    /// let portfolio = datasets::portfolio();
    /// let valued = portfolio.assets()?.evaluate(portfolio.weights()?).build()?;
    /// let current = portfolio.report().build()?.overview.current.unwrap();
    /// assert!((valued.results.volatility - current.volatility).abs() < 1e-12);
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn evaluate<K: ToString>(
        &self,
        weights: impl IntoIterator<Item = (K, f64)>,
    ) -> Evaluate<'_> {
        Evaluate {
            assets: self,
            weights: weights
                .into_iter()
                .map(|(asset, w)| (asset.to_string(), w))
                .collect(),
            options: OptimiseOptions::default(),
            name: None,
        }
    }

    /// `weights` by asset valued as [`Assets::evaluate`] values them, with the options it
    /// takes from `options`: the risk-free rate and expected returns, and the frontier with its
    /// limits. Its method is not used. Every problem is reported together.
    pub fn evaluate_with(
        &self,
        weights: &BTreeMap<String, f64>,
        options: &OptimiseOptions,
    ) -> Result<GivenWeights> {
        let mut report = ValidationReport::new();
        for name in crate::methods::given(options) {
            if !TAKES.contains(&name) {
                report.error(
                    format!("options.{name}"),
                    format!("valuing given weights does not use {name}"),
                );
            } else if crate::methods::is_limit(name) && !options.frontier {
                report.error(
                    format!("options.{name}"),
                    "limits apply to the efficient frontier: ask for it with frontier",
                );
            }
        }
        // The frontier is that of the risky assets, under the limits, as an optimisation's.
        let mut frontier = options.clone();
        frontier.method = Method::MinimumVariance;
        report.merge("", self.check_options(&frontier, false));

        let rf_asset = self.risk_free_asset();
        let at_rate = rf_asset.is_none() && !self.labels().iter().any(|l| l == CASH);
        if weights.is_empty() {
            report.error("weights", "give the weights by asset, adding up to one");
        }
        let mut known = true;
        for (asset, w) in weights {
            let path = format!("weights.{asset}");
            if !(self.labels().contains(asset) || (at_rate && asset == CASH)) {
                report.error(path, format!("unknown asset {asset}"));
                known = false;
            } else if !w.is_finite() {
                report.error(path, format!("must be a finite number, not {w}"));
                known = false;
            }
        }
        let total: f64 = weights.values().sum();
        if known && !weights.is_empty() && (total - 1.0).abs() > 1e-6 {
            report.error(
                "weights",
                match rf_asset {
                    Some(cash) => format!(
                        "add up to {total}, not one; give {cash}, the risk-free asset, any cash"
                    ),
                    None if at_rate => format!("add up to {total}, not one; give {CASH} any cash"),
                    None => format!("add up to {total}, not one"),
                },
            );
        }
        report.into_result(())?;

        // The risky assets, then the cash the weights hold, in the assets' order.
        let mut labels: Vec<String> = self.risky_assets().into_iter().map(str::to_owned).collect();
        let cash: Vec<String> = self
            .cash_assets()
            .into_iter()
            .map(str::to_owned)
            .chain(at_rate.then(|| CASH.to_owned()))
            .filter(|c| weights.contains_key(c))
            .collect();
        labels.extend(cash.iter().cloned());
        let rf = options.risk_free_rate.unwrap_or(self.risk_free_rate);
        let covariance = self.covariance_of(&labels)?;
        let mut expected = self.expected_of(&labels)?;
        for (i, label) in labels.iter().enumerate() {
            if let Some(r) = options.expected_returns.get(label) {
                expected[i] = *r;
            } else if label == CASH && at_rate {
                expected[i] = rf;
            }
        }
        let w: Vec<f64> = labels
            .iter()
            .map(|l| weights.get(l).copied().unwrap_or(0.0))
            .collect();
        let n = w.len();
        let marginal: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| covariance.get(i, j) * w[j]).sum())
            .collect();
        let variance: f64 = w.iter().zip(&marginal).map(|(a, b)| a * b).sum();
        let volatility = variance.max(0.0).sqrt();
        let expected_return: f64 = w.iter().zip(&expected).map(|(a, b)| a * b).sum();
        let results = Evaluation {
            weights: weights.clone(),
            expected_return,
            volatility,
            sharpe_ratio: (volatility > 0.0).then(|| (expected_return - rf) / volatility),
            risk_free_rate: rf,
            risk_shares: labels
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let share = if variance > 0.0 {
                        w[i] * marginal[i] / variance
                    } else {
                        0.0
                    };
                    (l.clone(), share)
                })
                .collect(),
        };

        let points = if options.frontier {
            self.problem(&frontier)
                .and_then(|(problem, _)| problem.frontier(FRONTIER_POINTS))
                .map(|points| {
                    points
                        .iter()
                        .map(|p| (p.volatility, p.expected_return))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        // The history, with cash in other currencies held as the assets it is, against equal
        // weight of the risky assets.
        let history = {
            let mut held = self.clone();
            held.held_cash.clear();
            let risky = self.risky_assets();
            let equal: BTreeMap<String, f64> = risky
                .iter()
                .map(|a| ((*a).to_owned(), 1.0 / risky.len() as f64))
                .collect();
            let history = crate::backtest::BacktestOptions {
                strategies: vec![crate::backtest::Strategy::Weights {
                    name: Some("The weights".to_owned()),
                    weights: weights.clone(),
                }],
                benchmark: Some(crate::backtest::Benchmark::Weights {
                    name: Some("Equal weight".to_owned()),
                    weights: equal,
                }),
                ..crate::backtest::BacktestOptions::default()
            };
            held.backtest_with(&history)
                .ok()
                .and_then(|b| Some((b.report.strategy.tested, b.report.benchmark?.tested)))
        };
        let (Some(&start), Some(&end)) = (self.dates.first(), self.dates.last()) else {
            let mut report = ValidationReport::new();
            report.error("dates", "there are none");
            return Err(Error::Validation(report));
        };
        let by_label: BTreeMap<String, f64> = labels.iter().cloned().zip(w).collect();
        let report =
            finalytics_reports::optimisation_report(finalytics_reports::OptimisationInputs {
                method: "Given weights".to_owned(),
                description: "the weights as given, valued on the assets' estimates.".to_owned(),
                status: "given".to_owned(),
                solver: String::new(),
                iterations: 0,
                labels: &labels,
                weights: Some(&by_label),
                expected_returns: &expected,
                mean_returns: options
                    .expected_returns
                    .is_empty()
                    .then_some(self.mean_method),
                covariance: &covariance,
                risk_free_asset: rf_asset.map(str::to_owned),
                cash,
                given: true,
                risk_free_rate: rf,
                periods_per_year: self.periods_per_year,
                covariance_method: self.covariance_method,
                limits: if options.frontier {
                    crate::backtest::limits(options, false)
                } else {
                    Vec::new()
                },
                start,
                end,
                frontier: points,
                history,
                shortfall: None,
                semivariance: None,
                drawdown_at_risk: None,
                groups: options
                    .groups
                    .iter()
                    .map(|(name, g)| (name.clone(), self.group_assets(name, g), g.min, g.max))
                    .collect(),
                resampling: None,
                current_weights: None,
                benchmark_weights: None,
            })?;
        Ok(GivenWeights { report, results })
    }
}

/// Given weights valued on the assets' estimates, built step by step; [`Evaluate::build`]
/// values them. The limits apply to the efficient frontier, which the report draws when asked.
#[derive(Debug, Clone)]
#[must_use = "valuing weights does nothing until `build` runs it"]
pub struct Evaluate<'a> {
    assets: &'a Assets,
    weights: BTreeMap<String, f64>,
    options: OptimiseOptions,
    name: Option<String>,
}

impl Evaluate<'_> {
    /// An annual risk-free rate for the Sharpe ratio, in place of the assets' own.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.options.risk_free_rate = Some(rate);
        self
    }

    /// An asset's annual expected return in place of its estimate, such as a view.
    pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
        self.options
            .expected_returns
            .insert(asset.to_owned(), value);
        self
    }

    /// Draws the efficient frontier in the report, under the limits given, so that the weights
    /// can be seen against it. It takes 30 optimisations.
    pub fn with_frontier(mut self) -> Self {
        self.options.frontier = true;
        self
    }

    /// The frontier's largest weight of any asset, such as 0.4 for 40%.
    pub fn max_weight(mut self, max: f64) -> Self {
        self.options.max_weight = Some(max);
        self
    }

    /// The frontier's limits on every asset's weight.
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.options.bounds = Some(Bound { min, max });
        self
    }

    /// The frontier's limits on one asset's weight.
    pub fn asset_bounds(mut self, asset: &str, min: f64, max: f64) -> Self {
        self.options
            .asset_bounds
            .insert(asset.to_owned(), Bound { min, max });
        self
    }

    /// The frontier's limits on the total weight of the `assets` of group `name`.
    pub fn group<S: AsRef<str>>(mut self, name: &str, assets: &[S], min: f64, max: f64) -> Self {
        self.options.groups.insert(
            name.to_owned(),
            AssetGroup {
                assets: assets.iter().map(|a| a.as_ref().to_owned()).collect(),
                classification: None,
                min,
                max,
            },
        );
        self
    }

    /// The frontier's limits on the total weight of the assets classed `class` under
    /// `scheme`, such as `Equity` under `asset_class`.
    pub fn class_limit(mut self, scheme: &str, class: &str, min: f64, max: f64) -> Self {
        self.options.groups.insert(
            class.to_owned(),
            AssetGroup {
                assets: Vec::new(),
                classification: Some(scheme.to_owned()),
                min,
                max,
            },
        );
        self
    }

    /// The frontier's smallest effective number of assets, `1 / sum w^2`, such as 5.
    pub fn min_effective_assets(mut self, k: f64) -> Self {
        self.options.min_effective_assets = Some(k);
        self
    }

    /// Cash on the frontier between `min` and `max` of the portfolio, as for optimisation.
    pub fn cash(mut self, min: f64, max: f64) -> Self {
        self.options.cash = Some(Bound { min, max });
        self
    }

    /// The report's title, such as `"Current portfolio"`: "Given weights" by default.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// The options built so far.
    pub fn options(&self) -> &OptimiseOptions {
        &self.options
    }

    /// Values the weights: the figures in `results`, with the report in `report`.
    pub fn build(&self) -> Result<GivenWeights> {
        let mut valued = self.assets.evaluate_with(&self.weights, &self.options)?;
        valued.report.name.clone_from(&self.name);
        Ok(valued)
    }
}
