//! Assets: the prices or returns of things one might hold, and what can be learnt from them.

use std::collections::BTreeMap;

mod evaluate;
pub use evaluate::{Evaluate, Evaluation, GivenWeights};

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Decimal, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, price_columns_from_csv};
use finalytics_ledger::HoldingKey;
use finalytics_opt::{
    AssetBound, Bound, Constraints, GroupLimit, Linkage, Objective, Problem, ResampledSolution,
    Resampling, ReturnUncertainty, Solution, SolveStatus, TrackingErrorLimit, TradeLimits,
    Turnover,
};
use finalytics_reports::OptimisationReport;
use finalytics_risk::{
    CovarianceEstimate, CovarianceMethod, Matrix, MeanMethod, PriceTable, ReturnsTable, Statistics,
    StatisticsOptions, TailMethod, TailRisk, covariance, periods_per_year, statistics, tail_risk,
};

/// The prices or returns of several assets over the same periods, with the settings that turn
/// them into annual figures: the number of periods in a year (detected from the dates), the
/// annual risk-free rate (zero by default), the covariance estimator (Ledoit–Wolf by default)
/// and the estimator of mean returns (the historical mean by default).
///
/// ```
/// use finalytics::{Assets, Method, datasets};
///
/// let assets = Assets::builder()
///     .prices(datasets::asset_class_prices())
///     .risk_free_rate(0.02)
///     .build()?;
/// let result = assets
///     .optimise(Method::MaximumSharpe)
///     .max_weight(0.4)
///     .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
///     .build()?;
/// let weights = result.results.weights.unwrap_or_default();
/// assert!(weights.values().all(|w| *w <= 0.4 + 1e-6));
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assets {
    /// The date of the prices the first returns are measured from, when the assets came from
    /// prices.
    origin: Option<Date>,
    dates: Vec<Date>,
    returns: ReturnsTable,
    periods_per_year: f64,
    risk_free_rate: f64,
    risk_free_asset: Option<usize>,
    covariance_method: CovarianceMethod,
    mean_method: MeanMethod,
    warnings: ValidationReport,
    /// Each classification scheme's class of each asset classified by it, such as
    /// `asset_class` → `US Equity` → `Equity`.
    classes: BTreeMap<String, BTreeMap<String, String>>,
    /// Cash in other currencies than the risk-free asset's, such as a portfolio's euro cash:
    /// valued and estimated with the other assets, but not chosen by optimisation.
    held_cash: Vec<usize>,
}

/// One asset's statistics, with its value at risk and expected shortfall at 95% (historical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AssetStatistics {
    /// The asset.
    pub asset: String,
    /// Its statistics, annualised with the assets' periods per year and risk-free rate.
    pub statistics: Statistics,
    /// Its value at risk and expected shortfall over one period at 95%, from history.
    pub tail_risk: TailRisk,
}

/// An optimisation method, by name. The parameters some methods need are given beside it, in
/// [`OptimiseOptions`] or with the builder's methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Method {
    /// The smallest variance.
    MinimumVariance,
    /// The largest expected return less `risk_aversion / 2` times the variance.
    MeanVariance,
    /// Mean–variance against the worst expected returns within `uncertainty`.
    RobustMeanVariance,
    /// The smallest variance with an expected return of at least `target`.
    TargetReturn,
    /// The largest expected return with a volatility of at most `target`.
    TargetVolatility,
    /// The largest Sharpe ratio.
    #[default]
    MaximumSharpe,
    /// The largest diversification ratio.
    MaximumDiversification,
    /// The smallest conditional value at risk of the periods' returns, at `confidence`.
    MinimumCvar,
    /// The smallest semivariance of the periods' returns below zero.
    MinimumSemivariance,
    /// The smallest conditional drawdown at risk of the cumulative returns, at `confidence`.
    MinimumCdar,
    /// Every asset with the same weight: the naive portfolio that estimated ones must beat.
    EqualWeight,
    /// Weights in proportion to the inverse of each asset's volatility.
    InverseVolatility,
    /// Every asset with the same share of the portfolio's risk.
    RiskParity,
    /// Each asset's share of the portfolio's risk equal to its budget, given with `budgets`
    /// (equal shares without them, as risk parity).
    RiskBudgeting,
    /// Hierarchical risk parity.
    HierarchicalRiskParity,
}

/// The points on the efficient frontier that an optimisation report draws.
const FRONTIER_POINTS: usize = 30;

/// The asset that holds cash at the risk-free rate when the assets have no risk-free asset.
pub(crate) const CASH: &str = "Cash";

/// An optimisation: the optimisation report on the weights chosen, and the raw results.
/// Printing it prints the report, and showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Optimisation {
    /// The optimisation report: the weights, what they are expected to deliver, where their
    /// risk comes from, where they sit against each asset (and on the efficient frontier, if
    /// asked for) and the history they were fitted to.
    pub report: OptimisationReport,
    /// The raw results: the weights by asset, their expected return, volatility and Sharpe
    /// ratio, and the solver's solution.
    pub results: OptimisationResult,
}

impl std::fmt::Display for Optimisation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl Method {
    /// What the method does, in a sentence, for reports.
    pub fn description(self) -> &'static str {
        match self {
            Method::MinimumVariance => "the weights with the smallest variance.",
            Method::MeanVariance => {
                "the largest expected return less half the risk aversion times the variance."
            }
            Method::RobustMeanVariance => {
                "mean–variance against the worst expected returns within the uncertainty given."
            }
            Method::TargetReturn => {
                "the smallest variance with at least the target expected return."
            }
            Method::TargetVolatility => {
                "the largest expected return with at most the target volatility."
            }
            Method::MaximumSharpe => {
                "the largest expected return in excess of the risk-free rate per unit of \
                 volatility, solved as a quadratic programme after the Charnes–Cooper \
                 transformation."
            }
            Method::MaximumDiversification => {
                "the largest ratio of the weighted average of the assets' volatilities to the \
                 portfolio's."
            }
            Method::MinimumCvar => {
                "the smallest average loss in the worst periods, at the confidence given, \
                 from the historical returns as scenarios."
            }
            Method::MinimumSemivariance => {
                "the smallest mean square of the returns below zero, from the historical returns \
                 as scenarios."
            }
            Method::MinimumCdar => {
                "the smallest average of the worst drawdowns, at the confidence given, of the \
                 historical returns added up period by period."
            }
            Method::EqualWeight => "every asset with the same weight.",
            Method::InverseVolatility => {
                "weights in proportion to the inverse of each asset's volatility."
            }
            Method::RiskParity => "every asset with the same share of the portfolio's risk.",
            Method::RiskBudgeting => {
                "each asset's share of the portfolio's risk equal to its budget."
            }
            Method::HierarchicalRiskParity => {
                "clusters of assets by correlation, with weights split between clusters by \
                 their risk."
            }
        }
    }

    /// The method's name in words, such as `"Maximum Sharpe"`, for reports.
    pub fn label(self) -> &'static str {
        match self {
            Method::MinimumVariance => "Minimum variance",
            Method::MeanVariance => "Mean–variance",
            Method::RobustMeanVariance => "Robust mean–variance",
            Method::TargetReturn => "Target return",
            Method::TargetVolatility => "Target volatility",
            Method::MaximumSharpe => "Maximum Sharpe",
            Method::MaximumDiversification => "Maximum diversification",
            Method::MinimumCvar => "Minimum CVaR",
            Method::MinimumSemivariance => "Minimum semivariance",
            Method::MinimumCdar => "Minimum CDaR",
            Method::EqualWeight => "Equal weight",
            Method::InverseVolatility => "Inverse volatility",
            Method::RiskParity => "Risk parity",
            Method::RiskBudgeting => "Risk budgeting",
            Method::HierarchicalRiskParity => "Hierarchical risk parity",
        }
    }

    /// The method's name within a sentence: "maximum Sharpe", "minimum CVaR".
    pub(crate) fn in_sentence(self) -> String {
        let label = self.label();
        let mut chars = label.chars();
        match chars.next() {
            Some(first) => first.to_lowercase().chain(chars).collect(),
            None => String::new(),
        }
    }

    /// Whether the method is measured on the periods' returns as scenarios: minimum CVaR,
    /// semivariance and CDaR.
    pub(crate) fn uses_scenarios(self) -> bool {
        matches!(
            self,
            Method::MinimumCvar | Method::MinimumSemivariance | Method::MinimumCdar
        )
    }

    /// Whether the method's weights can be resampled: every method that uses estimates, other
    /// than those measured on the periods' returns.
    pub(crate) fn resamples(self) -> bool {
        !(self == Method::EqualWeight || self.uses_scenarios())
    }

    /// Whether the method takes limits on the weights. Equal weight, inverse volatility, risk
    /// parity, risk budgeting and hierarchical risk parity give a fully invested, long-only
    /// portfolio and take none.
    pub fn takes_limits(self) -> bool {
        !matches!(
            self,
            Method::EqualWeight
                | Method::InverseVolatility
                | Method::RiskParity
                | Method::RiskBudgeting
                | Method::HierarchicalRiskParity
        )
    }
}

/// A group of assets whose total weight is limited: the assets listed, or those classed as the
/// group's name under a classification, such as the group "Equity" under `asset_class`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssetGroup {
    /// The assets in the group; or give `classification`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<String>,
    /// A classification scheme of the assets, such as `asset_class`: the group is the assets
    /// classed as its name under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// The smallest total weight: 0 by default.
    #[serde(default)]
    pub min: f64,
    /// The largest total weight: 1 by default.
    #[serde(default = "one")]
    pub max: f64,
}

fn one() -> f64 {
    1.0
}

/// The cash to keep as it is: every currency's (`true`), or that of the currencies named, such
/// as `["USD"]`. Rebalancing keeps it as it is, the targets sharing the rest of the value;
/// optimisation holds it at its current weight and optimises the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum KeepCash {
    /// Every currency's cash, when true; none when false.
    All(bool),
    /// The cash of these currencies, by ISO 4217 code, such as `["USD"]`.
    Currencies(Vec<String>),
}

impl KeepCash {
    /// Whether it keeps any cash: true, or a list that names a currency.
    pub fn keeps_any(&self) -> bool {
        match self {
            KeepCash::All(all) => *all,
            KeepCash::Currencies(list) => !list.is_empty(),
        }
    }
}

/// Writes `keep` as `null` when it keeps no cash, as `false` and an empty list do, so that
/// options that mean the same are written, and digested, the same in every language.
fn keep_cash_written<S: serde::Serializer>(
    keep: &Option<KeepCash>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    keep.as_ref()
        .filter(|k| k.keeps_any())
        .serialize(serializer)
}

/// The method, its parameters and the limits on the weights. Everything is optional: by default
/// the portfolio with the largest Sharpe ratio, fully invested and long only.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct OptimiseOptions {
    /// The method: the largest Sharpe ratio by default.
    pub method: Method,
    /// The annual risk-free rate for the Sharpe ratio; by default the assets' own.
    pub risk_free_rate: Option<f64>,
    /// The annual expected return (`target_return`) or volatility (`target_volatility`) to aim
    /// for, or the smallest annual expected return for minimum CVaR, semivariance and CDaR.
    pub target: Option<f64>,
    /// The weight on variance, for mean–variance objectives.
    pub risk_aversion: Option<f64>,
    /// The uncertainty of the expected returns, for robust mean–variance.
    pub uncertainty: Option<ReturnUncertainty>,
    /// The confidence of the conditional value at risk or drawdown at risk: 0.95 by default.
    pub confidence: Option<f64>,
    /// The risk budgets, by asset, for risk budgeting; equal without them.
    pub budgets: Option<BTreeMap<String, f64>>,
    /// How hierarchical risk parity joins clusters: single linkage by default.
    pub linkage: Option<Linkage>,
    /// The smallest and largest weight of every asset: 0 and 1 by default.
    pub bounds: Option<Bound>,
    /// The largest weight of any asset, as `bounds` of 0 and `max_weight`; give one or the
    /// other.
    pub max_weight: Option<f64>,
    /// Bounds for particular assets, which take the place of `bounds` for them.
    pub asset_bounds: BTreeMap<String, Bound>,
    /// Limits on the total weight of groups of assets, by group name.
    pub groups: BTreeMap<String, AssetGroup>,
    /// The smallest effective number of assets, `1 / sum w^2`, such as 5: the weights are
    /// spread over at least as many assets as 5 equal weights would be.
    pub min_effective_assets: Option<f64>,
    /// Resampled optimisation (Michaud, 1998): the weights are the average of the optimal
    /// weights for this many histories, such as 200, simulated from the estimates with as many
    /// returns as the assets have, from random numbers with a fixed seed. For the methods that
    /// use estimates, other than those measured on the periods' returns.
    pub resample: Option<usize>,
    /// Annual expected returns by asset, in place of the estimates.
    pub expected_returns: BTreeMap<String, f64>,
    /// The weights held now, by asset, adding up to one with the cash: any of the assets, the
    /// risk-free asset and cash in other currencies included, or `Cash` at the risk-free rate
    /// where there is no risk-free asset. They are the start of the trades the report lists, and
    /// of the turnover and trade limits; optimisation invests the cash it does not choose.
    pub current_weights: BTreeMap<String, f64>,
    /// The largest one-way turnover from the current weights, `sum |w - current| / 2`, such as
    /// 0.2 for 20% of the portfolio bought (and as much sold).
    pub max_turnover: Option<f64>,
    /// The smallest trade, as a weight, such as 0.01: each asset is traded by at least this
    /// much from its current weight, or not at all.
    pub min_trade: Option<f64>,
    /// The largest number of assets traded from the current weights.
    pub max_trades: Option<usize>,
    /// A benchmark's weights by asset, adding up to one with the cash, as `current_weights`
    /// may hold it: the report gives the weights' ex-ante tracking error against it, counting the
    /// risk of any cash in other currencies.
    pub benchmark_weights: BTreeMap<String, f64>,
    /// The largest ex-ante tracking error a year against `benchmark_weights`, such as 0.03.
    pub max_tracking_error: Option<f64>,
    /// The cash held now to keep at its current weight, as rebalancing keeps it: every
    /// currency's (`true`) or that of the currencies named (`["USD"]`), at its weight in
    /// `current_weights`. The rest of the portfolio is optimised, and the
    /// limits are measured on the whole of it, so that rebalancing to the weights with the same
    /// cash kept trades what the limits allow. The cash of a currency is the asset named for it,
    /// such as `EUR cash`; `true` also keeps a risk-free asset that is not, such as T-bills, and
    /// `Cash` at the risk-free rate. It needs `current_weights`, and the cash it keeps takes no
    /// `cash` limits.
    #[serde(serialize_with = "keep_cash_written")]
    pub keep_cash: Option<KeepCash>,
    /// The smallest and largest weight of cash, such as 0 and 0.2: with it, optimisation may
    /// hold cash beside the risky assets, riskless, and the frontier includes it. The cash is
    /// the risk-free asset, earning its return, or, without one, an asset named `Cash` earning
    /// the risk-free rate. Without it, the weights are fully invested in the risky assets.
    pub cash: Option<Bound>,
    /// Whether the optimisation report draws the efficient frontier under the same limits.
    /// Off by default: the frontier takes 30 more optimisations, so it adds time that grows with
    /// the number of assets, to seconds for hundreds of assets. Only an optimisation reads it;
    /// backtests leave it out.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "schema", schemars(extend("default" = false)))]
    pub frontier: bool,
}

/// A portfolio on the efficient frontier of [`Assets`]: its annual expected return and
/// volatility, and its weights by asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FrontierPortfolio {
    /// The annual expected return.
    pub expected_return: f64,
    /// The annual volatility.
    pub volatility: f64,
    /// The weights, by asset.
    pub weights: BTreeMap<String, f64>,
}

/// The result of an optimisation over assets: the weights by asset, with their expected return,
/// volatility and Sharpe ratio, and the solver's full solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationResult {
    /// How the optimisation ended.
    pub status: SolveStatus,
    /// The weights by asset, absent when there is no solution.
    pub weights: Option<BTreeMap<String, f64>>,
    /// The annual expected return of the weights.
    pub expected_return: Option<f64>,
    /// The annual volatility of the weights.
    pub volatility: Option<f64>,
    /// `(expected_return - risk_free_rate) / volatility`.
    pub sharpe_ratio: Option<f64>,
    /// The annual risk-free rate used.
    pub risk_free_rate: f64,
    /// The solver's solution, with the objective, its value and the work done. With
    /// resampling, it is the solution for the estimates themselves, and `weights` are the
    /// average in `resampled`.
    pub solution: Solution,
    /// For resampled optimisation, the average weights, how much each varied across the
    /// simulated histories, and how many were solved.
    #[serde(default)]
    pub resampled: Option<ResampledSolution>,
}

/// The weights as text: each asset's weight, the largest first, with the expected return,
/// volatility and Sharpe ratio.
impl std::fmt::Display for OptimisationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pct = |v: Option<f64>| {
            v.filter(|v| v.is_finite()).map_or_else(
                || "–".to_owned(),
                |v| format!("{:.1}%", v * 100.0).replace('-', "−"),
            )
        };
        let method = serde_json::to_value(&self.solution.objective)
            .ok()
            .and_then(|v| serde_json::from_value::<Method>(v["type"].clone()).ok())
            .map_or("Optimisation", Method::label);
        let title = format!("OptimisationResult · {method} · {}", self.status);
        let mut figures = vec![
            ("Expected return".to_owned(), pct(self.expected_return)),
            ("volatility".to_owned(), pct(self.volatility)),
            (
                "Sharpe ratio".to_owned(),
                self.sharpe_ratio
                    .map_or_else(|| "–".to_owned(), |v| format!("{v:.2}").replace('-', "−")),
            ),
        ];
        figures.push(("risk-free".to_owned(), pct(Some(self.risk_free_rate))));
        let empty = BTreeMap::new();
        f.write_str(&finalytics_reports::weights_text(
            &title,
            self.weights.as_ref().unwrap_or(&empty),
            &figures,
        ))
    }
}

impl OptimisationResult {
    /// The weights as holdings, for [`Portfolio::rebalance`](crate::Portfolio::rebalance): each
    /// asset as the instrument of the same name. Empty when there is no solution.
    pub fn holdings(&self) -> BTreeMap<HoldingKey, f64> {
        self.weights
            .iter()
            .flatten()
            .filter_map(|(asset, w)| asset.parse().ok().map(|key| (key, *w)))
            .collect()
    }
}

impl Assets {
    /// A builder for assets from their prices or returns, with a risk-free asset or rate.
    pub fn builder() -> AssetsBuilder {
        AssetsBuilder::default()
    }

    /// Assets from their prices: the returns between consecutive dates on which every asset has
    /// a price. Prices should include income, as adjusted closes do. The number of periods in a
    /// year is detected from the dates.
    pub fn from_prices(prices: &PriceTable) -> Result<Assets> {
        let found = prices.returns()?;
        let periods = periods_per_year(prices.dates());
        // The latest date before the first return on which every asset has a price.
        let origin = found.dates.first().and_then(|first| {
            prices
                .dates()
                .iter()
                .enumerate()
                .take_while(|(_, d)| *d < first)
                .filter(|(i, _)| prices.columns().iter().all(|c| c[*i].is_some()))
                .map(|(_, d)| *d)
                .last()
        });
        Ok(Assets {
            origin,
            dates: found.dates,
            returns: found.table,
            periods_per_year: periods,
            risk_free_rate: 0.0,
            risk_free_asset: None,
            covariance_method: CovarianceMethod::LedoitWolf,
            mean_method: MeanMethod::Historical,
            warnings: found.warnings,
            classes: BTreeMap::new(),
            held_cash: Vec::new(),
        })
    }

    /// Assets from their returns: `dates` are the dates each return ends on, one per row of the
    /// table. The number of periods in a year is detected from the dates.
    pub fn from_returns(dates: Vec<Date>, returns: ReturnsTable) -> Result<Assets> {
        let mut report = ValidationReport::new();
        if dates.len() != returns.observations() {
            report.error(
                "dates",
                format!(
                    "{} dates for {} returns",
                    dates.len(),
                    returns.observations()
                ),
            );
        }
        for (i, pair) in dates.windows(2).enumerate() {
            if pair[1] <= pair[0] {
                report.error(
                    format!("dates[{}]", i + 1),
                    format!("{} does not follow {}", pair[1], pair[0]),
                );
            }
        }
        report.into_result(())?;
        let periods = periods_per_year(&dates);
        Ok(Assets {
            origin: None,
            dates,
            returns,
            periods_per_year: periods,
            risk_free_rate: 0.0,
            risk_free_asset: None,
            covariance_method: CovarianceMethod::LedoitWolf,
            mean_method: MeanMethod::Historical,
            warnings: ValidationReport::new(),
            classes: BTreeMap::new(),
            held_cash: Vec::new(),
        })
    }

    /// Assets from prices in CSV: a `date` column and one column of prices per asset, or the
    /// long layout of `date`, `instrument` and `price` columns. The assets keep the order of the
    /// file.
    pub fn from_csv(text: &str) -> Result<Assets> {
        let series = price_columns_from_csv(text, CsvOptions::default())?;
        let mut dates: Vec<Date> = series
            .iter()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .collect();
        dates.sort_unstable();
        dates.dedup();
        let labels: Vec<String> = series.iter().map(|(id, _)| id.to_string()).collect();
        let columns: Vec<Vec<Option<f64>>> = series
            .iter()
            .map(|(_, s)| {
                dates
                    .iter()
                    .map(|d| s.get(*d).map(|p| to_f64(*p)))
                    .collect()
            })
            .collect();
        Assets::from_prices(&PriceTable::new(dates, labels, columns)?)
    }

    /// The same assets with an annual risk-free rate, for Sharpe ratios.
    pub fn with_risk_free_rate(mut self, rate: f64) -> Assets {
        self.risk_free_rate = rate;
        self.risk_free_asset = None;
        self
    }

    /// The same assets with one of them, such as T-bills, as the risk-free asset: the
    /// risk-free rate is its mean return, annualised as expected returns are, and optimisation
    /// leaves it out, so that it finds the best portfolio of the risky assets. Its statistics,
    /// covariance and correlation are still given.
    pub fn with_risk_free_asset(mut self, asset: &str) -> Result<Assets> {
        let Some(i) = self.labels().iter().position(|l| l == asset) else {
            let mut report = ValidationReport::new();
            report.error("risk_free_asset", format!("unknown asset {asset}"));
            return Err(Error::Validation(report));
        };
        self.risk_free_rate = self.historical_means()[i];
        self.risk_free_asset = Some(i);
        Ok(self)
    }

    /// The same assets with the number of periods in a year given rather than detected: 252 for
    /// daily returns, 52 for weekly, 12 for monthly.
    pub fn with_periods_per_year(mut self, periods: f64) -> Assets {
        self.periods_per_year = periods;
        self
    }

    /// The same assets with another covariance estimator.
    pub fn with_covariance(mut self, method: CovarianceMethod) -> Assets {
        self.covariance_method = method;
        self
    }

    /// The covariance estimator.
    pub fn covariance_method(&self) -> CovarianceMethod {
        self.covariance_method
    }

    /// The same assets with another estimator of mean returns, checked on the risky assets'
    /// returns: James–Stein needs more returns than risky assets.
    pub fn with_mean_returns(mut self, method: MeanMethod) -> Result<Assets> {
        self.mean_method = method;
        let mut report = ValidationReport::new();
        if let Err(e) = self.risky_means(&self.risky_returns()?) {
            rehome(e, "mean_returns", &mut report)?;
        }
        report.into_result(self)
    }

    /// The estimator of mean returns.
    pub fn mean_method(&self) -> MeanMethod {
        self.mean_method
    }

    /// The date of the prices the first returns are measured from, when the assets came from
    /// prices: the start of the first period.
    pub fn origin(&self) -> Option<Date> {
        self.origin
    }

    /// The assets' labels.
    pub fn labels(&self) -> &[String] {
        self.returns.labels()
    }

    /// The date each return ends on.
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }

    /// The returns, one column per asset.
    pub fn returns(&self) -> &ReturnsTable {
        &self.returns
    }

    /// The number of periods in a year.
    pub fn periods_per_year(&self) -> f64 {
        self.periods_per_year
    }

    /// The annual risk-free rate.
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// The risk-free asset, if one of the assets is.
    pub fn risk_free_asset(&self) -> Option<&str> {
        self.risk_free_asset.map(|i| self.labels()[i].as_str())
    }

    /// The assets optimisation chooses among: all but the cash, that is, the risk-free asset and
    /// any cash in other currencies.
    pub fn risky_assets(&self) -> Vec<&str> {
        self.labels()
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.is_cash(*i))
            .map(|(_, l)| l.as_str())
            .collect()
    }

    /// The cash among the assets, which optimisation does not choose: the risk-free asset, if
    /// any, then cash in other currencies, such as a portfolio's euro cash.
    pub fn cash_assets(&self) -> Vec<&str> {
        self.risk_free_asset
            .iter()
            .chain(&self.held_cash)
            .map(|&i| self.labels()[i].as_str())
            .collect()
    }

    fn is_cash(&self, i: usize) -> bool {
        Some(i) == self.risk_free_asset || self.held_cash.contains(&i)
    }

    /// Whether `Cash` stands for cash at the risk-free rate: the assets have no risk-free asset
    /// and none of them is named `Cash`.
    pub(crate) fn cash_at_rate(&self) -> bool {
        self.risk_free_asset.is_none() && !self.labels().iter().any(|l| l == CASH)
    }

    /// Whether weights by asset may name `label`: every one of the assets, cash included, and
    /// `Cash` where it stands for cash at the risk-free rate.
    pub(crate) fn takes_label(&self, label: &str) -> bool {
        self.labels().iter().any(|l| l == label) || (label == CASH && self.cash_at_rate())
    }

    /// Whether `label` is cash: the risk-free asset, cash in another currency, or `Cash` at the
    /// risk-free rate.
    pub(crate) fn is_cash_label(&self, label: &str) -> bool {
        match self.labels().iter().position(|l| l == label) {
            Some(i) => self.is_cash(i),
            None => label == CASH && self.cash_at_rate(),
        }
    }

    /// Checks weights by asset given at `path`, such as `options.current_weights`, by the rule
    /// every input of weights follows: each names one of the assets, cash included, or `Cash`
    /// where it stands for cash at the risk-free rate, and is a finite number (and, unless
    /// `signed`, zero or more), and together they add up to one, checked whenever the numbers
    /// are. Returns whether the weights are all known and finite.
    pub(crate) fn check_weights(
        &self,
        weights: &BTreeMap<String, f64>,
        path: &str,
        signed: bool,
        report: &mut ValidationReport,
    ) -> bool {
        let mut known = true;
        let mut numbers = true;
        for (asset, w) in weights {
            let at = format!("{path}.{asset}");
            if !self.takes_label(asset) {
                report.error(at, unknown_asset(asset));
                known = false;
            } else if signed && !w.is_finite() {
                report.error(at, format!("must be a finite number, not {w}"));
                numbers = false;
            } else if !signed && !(w.is_finite() && *w >= 0.0) {
                report.error(at, format!("must be a weight of zero or more, not {w}"));
                numbers = false;
            }
        }
        let total: f64 = weights.values().sum();
        if numbers && total.is_finite() && !weights.is_empty() && (total - 1.0).abs() > 1e-6 {
            report.error(path, self.add_up(total));
        }
        known && numbers
    }

    /// The problem with weights that add up to `total`, with where their cash goes.
    fn add_up(&self, total: f64) -> String {
        match self.risk_free_asset() {
            Some(cash) => {
                format!("add up to {total}, not one; give {cash}, the risk-free asset, any cash")
            }
            None if self.cash_at_rate() => {
                format!("add up to {total}, not one; give {CASH} any cash")
            }
            None => format!("add up to {total}, not one"),
        }
    }

    /// Why `asset` cannot be limited or budgeted as the risky assets are: it is cash, which
    /// optimisation holds in its own way, or it is not one of the assets.
    pub(crate) fn not_chosen(&self, asset: &str) -> String {
        if self.risk_free_asset() == Some(asset) {
            format!(
                "{asset} is the risk-free asset, which optimisation holds only as cash: limit it \
                 with cash"
            )
        } else if asset == CASH && self.cash_at_rate() {
            format!(
                "{CASH} is cash at the risk-free rate, which optimisation holds only as cash: \
                 limit it with cash"
            )
        } else if self.is_cash_label(asset) {
            format!("{asset} is cash in another currency, which optimisation does not choose")
        } else {
            unknown_asset(asset)
        }
    }

    /// The asset that holds the cash `options` allow, if they allow any: the risk-free asset,
    /// or `Cash` at the risk-free rate.
    fn cash_choice(&self, options: &OptimiseOptions) -> Option<String> {
        options.cash?;
        Some(self.risk_free_asset().unwrap_or(CASH).to_owned())
    }

    /// Weights `w` of `labels` by label: every asset's, and the cash's where the weights hold
    /// some, so that cash a problem holds at zero only to measure a benchmark is left out.
    fn by_label(&self, labels: &[String], w: &[f64]) -> BTreeMap<String, f64> {
        labels
            .iter()
            .cloned()
            .zip(w.iter().copied())
            .filter(|(l, w)| *w != 0.0 || !self.held_cash.iter().any(|&i| self.labels()[i] == *l))
            .collect()
    }

    /// Whether the frontier uses the option `name` of `options`: the limits on weights and
    /// trades, the expected returns, the current and benchmark weights, and the risk-free rate
    /// where `Cash` earns it.
    pub(crate) fn frontier_uses(&self, options: &OptimiseOptions, name: &str) -> bool {
        crate::methods::is_limit(name)
            || matches!(
                name,
                "expected_returns" | "current_weights" | "benchmark_weights"
            )
            || (name == "risk_free_rate" && self.cash_at_rate() && options.cash.is_some())
    }

    /// The weight of the cash held now that optimisation invests: every cash in
    /// `current_weights` but the cash `options` let it choose and the cash they keep.
    fn invested_cash(&self, options: &OptimiseOptions) -> f64 {
        let chosen = self.cash_choice(options);
        options
            .current_weights
            .iter()
            .filter(|(l, _)| {
                self.is_cash_label(l)
                    && chosen.as_deref() != Some(l.as_str())
                    && !keeps(options.keep_cash.as_ref(), l)
            })
            .map(|(_, w)| *w)
            .sum()
    }

    /// The cash `options` keep at its current weight: each cash of `current_weights` that
    /// `keep_cash` names, with its weight now.
    pub(crate) fn kept_cash(&self, options: &OptimiseOptions) -> Vec<(String, f64)> {
        options
            .current_weights
            .iter()
            .filter(|(l, w)| {
                **w > 0.0 && self.is_cash_label(l) && keeps(options.keep_cash.as_ref(), l)
            })
            .map(|(l, w)| (l.clone(), *w))
            .collect()
    }

    /// The same assets with `labels` held as cash in other currencies: estimated and valued with
    /// the others, but not chosen by optimisation.
    pub(crate) fn with_held_cash(mut self, labels: &[String]) -> Assets {
        self.held_cash = labels
            .iter()
            .filter_map(|l| self.labels().iter().position(|x| x == l))
            .collect();
        self
    }

    /// The returns of the assets `keep` picks, in order.
    fn returns_of(&self, keep: impl Fn(usize) -> bool) -> Result<ReturnsTable> {
        let picked: Vec<usize> = (0..self.labels().len()).filter(|i| keep(*i)).collect();
        ReturnsTable::new(
            picked.iter().map(|&i| self.labels()[i].clone()).collect(),
            picked
                .iter()
                .map(|&i| self.returns.columns()[i].clone())
                .collect(),
        )
    }

    /// The returns of the risky assets, which optimisation chooses among.
    pub(crate) fn risky_returns(&self) -> Result<ReturnsTable> {
        if self.risk_free_asset.is_none() && self.held_cash.is_empty() {
            return Ok(self.returns.clone());
        }
        self.returns_of(|i| !self.is_cash(i))
    }

    /// The returns the estimates are made from: every asset but the risk-free asset, whose risk
    /// is none, so that the risky assets and any cash in other currencies share one estimate.
    pub(crate) fn estimation_returns(&self) -> Result<ReturnsTable> {
        match self.risk_free_asset {
            None => Ok(self.returns.clone()),
            Some(rf) => self.returns_of(|i| i != rf),
        }
    }

    /// The annualised covariance of `labels`, from the estimate of every asset but the
    /// risk-free asset, whose rows and columns are zero: cash is riskless.
    fn covariance_of(&self, labels: &[String]) -> Result<Matrix> {
        let estimated = self.estimation_returns()?;
        let all = covariance(&estimated, self.covariance_method)?
            .matrix
            .scaled(self.periods_per_year);
        let place: Vec<Option<usize>> = labels
            .iter()
            .map(|l| estimated.labels().iter().position(|x| x == l))
            .collect();
        let n = labels.len();
        let mut values = vec![0.0; n * n];
        for (i, a) in place.iter().enumerate() {
            for (j, b) in place.iter().enumerate() {
                if let (Some(a), Some(b)) = (a, b) {
                    values[i * n + j] = all.get(*a, *b);
                }
            }
        }
        Matrix::new(labels.to_vec(), values)
    }

    /// Each label's annual expected return by the assets' estimator: the risk-free asset's is
    /// its historical mean, and the others' are estimated together.
    fn expected_of(&self, labels: &[String]) -> Result<Vec<f64>> {
        let estimated = self.estimation_returns()?;
        let means = self.risky_means(&estimated)?;
        let historical = self.historical_means();
        Ok(labels
            .iter()
            .map(|l| match estimated.labels().iter().position(|x| x == l) {
                Some(k) => means[k],
                None => self
                    .labels()
                    .iter()
                    .position(|x| x == l)
                    .map_or(0.0, |k| historical[k]),
            })
            .collect())
    }

    /// Whether the risk-free asset's returns are the same in every period, as idle cash's are:
    /// it is then left out of the covariance estimate, so that shrinkage does not lend it risk.
    fn riskless_cash(&self) -> bool {
        self.risk_free_asset.is_some_and(|i| {
            let c = &self.returns.columns()[i];
            c.iter().all(|r| *r == c[0])
        })
    }

    /// The return of cash in each period: the risk-free asset's, or the annual risk-free rate
    /// converted to a rate per period by compounding, `(1 + rate)^(1 / periods) - 1`, as the
    /// statistics convert it.
    pub(crate) fn cash_returns(&self) -> Vec<f64> {
        match self.risk_free_asset {
            Some(i) => self.returns.columns()[i].clone(),
            None => {
                let per_period =
                    (1.0 + self.risk_free_rate).powf(1.0 / self.periods_per_year) - 1.0;
                vec![per_period; self.dates.len()]
            }
        }
    }

    /// The same assets over the periods `rows` only, with the same settings: the periods per
    /// year, the estimator, and the risk-free asset (whose mean over `rows` becomes the rate) or
    /// rate.
    pub(crate) fn window(&self, rows: std::ops::Range<usize>) -> Result<Assets> {
        let returns = ReturnsTable::new(
            self.labels().to_vec(),
            self.returns
                .columns()
                .iter()
                .map(|c| c[rows.clone()].to_vec())
                .collect(),
        )?;
        let origin = match rows.start {
            0 => self.origin,
            i => Some(self.dates[i - 1]),
        };
        let mut window = Assets {
            origin,
            dates: self.dates[rows].to_vec(),
            returns,
            periods_per_year: self.periods_per_year,
            risk_free_rate: self.risk_free_rate,
            risk_free_asset: self.risk_free_asset,
            covariance_method: self.covariance_method,
            mean_method: self.mean_method,
            warnings: ValidationReport::new(),
            classes: self.classes.clone(),
            held_cash: self.held_cash.clone(),
        };
        if let Some(i) = self.risk_free_asset {
            window.risk_free_rate = window.historical_means()[i];
        }
        Ok(window)
    }

    /// Problems met reading the prices that did not stop the assets being built, such as dates
    /// left out because an asset had no price.
    pub fn warnings(&self) -> &ValidationReport {
        &self.warnings
    }

    /// Each classification scheme's class of each asset it classifies, such as `asset_class` →
    /// `US Equity` → `Equity`, for group limits by class.
    pub fn classes(&self) -> &BTreeMap<String, BTreeMap<String, String>> {
        &self.classes
    }

    /// The same assets classified by `scheme`, such as `asset_class`, with each asset's class
    /// in `classes`, such as `[("US Equity", "Equity"), ("Bonds", "Fixed income")]`: group
    /// limits can then name a class rather than list its assets.
    pub fn with_classes<S: Into<String>, C: Into<String>>(
        mut self,
        scheme: &str,
        classes: impl IntoIterator<Item = (S, C)>,
    ) -> Result<Assets> {
        let mut report = ValidationReport::new();
        let mut found = BTreeMap::new();
        for (asset, class) in classes {
            let asset = asset.into();
            if self.labels().contains(&asset) {
                found.insert(asset, class.into());
            } else {
                report.error(
                    format!("classes.{scheme}.{asset}"),
                    format!("unknown asset {asset}"),
                );
            }
        }
        report.into_result(())?;
        self.classes.insert(scheme.to_owned(), found);
        Ok(self)
    }

    /// The assets of group `name`, as `group` gives them: listed, or those of the risky assets
    /// classed `name` under its classification.
    pub(crate) fn group_assets(&self, name: &str, group: &AssetGroup) -> Vec<String> {
        match &group.classification {
            None => group.assets.clone(),
            Some(scheme) => self
                .risky_assets()
                .into_iter()
                .filter(|a| {
                    self.classes
                        .get(scheme)
                        .and_then(|c| c.get(*a))
                        .is_some_and(|c| c == name)
                })
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Each asset's statistics, in the order of the assets.
    pub fn statistics(&self) -> Result<Vec<AssetStatistics>> {
        let mut options = StatisticsOptions::default();
        options.periods_per_year = self.periods_per_year;
        options.risk_free_rate = self.risk_free_rate;
        options.target_return = self.risk_free_rate;
        self.labels()
            .iter()
            .zip(self.returns.columns())
            .map(|(label, column)| {
                Ok(AssetStatistics {
                    asset: label.clone(),
                    statistics: statistics(column, &options)?,
                    tail_risk: tail_risk(column, 0.95, TailMethod::Historical)?,
                })
            })
            .collect()
    }

    /// The covariance of the assets' returns, annualised, by the assets' estimator. A risk-free
    /// asset whose returns are the same in every period, such as a portfolio's base-currency cash,
    /// is riskless: it is left out of the estimate, and its row and column are zero.
    pub fn covariance(&self) -> Result<CovarianceEstimate> {
        if self.riskless_cash() {
            let mut estimate = covariance(&self.estimation_returns()?, self.covariance_method)?;
            estimate.matrix = self.covariance_of(self.labels())?;
            return Ok(estimate);
        }
        let mut estimate = covariance(&self.returns, self.covariance_method)?;
        estimate.matrix = estimate.matrix.scaled(self.periods_per_year);
        Ok(estimate)
    }

    /// The correlation of the assets' returns, from the covariance estimate.
    pub fn correlation(&self) -> Result<Matrix> {
        self.covariance()?.matrix.correlation()
    }

    /// Each asset's expected return, annualised, by the assets' estimator of mean returns: the
    /// risky assets' are estimated together, and the risk-free asset's is its historical mean,
    /// the risk-free rate. These are the expected returns optimisation uses unless others are
    /// given.
    pub fn mean_returns(&self) -> Vec<f64> {
        // The estimator was checked on these returns when it was chosen.
        self.expected_of(self.labels())
            .unwrap_or_else(|_| self.historical_means())
    }

    /// Each asset's historical mean return, annualised.
    fn historical_means(&self) -> Vec<f64> {
        self.returns
            .columns()
            .iter()
            .map(|c| c.iter().sum::<f64>() / c.len() as f64 * self.periods_per_year)
            .collect()
    }

    /// The mean returns of `returns`, annualised, by the assets' estimator.
    fn risky_means(&self, returns: &ReturnsTable) -> Result<Vec<f64>> {
        if self.mean_method == MeanMethod::Historical {
            return Ok(returns
                .columns()
                .iter()
                .map(|c| c.iter().sum::<f64>() / c.len() as f64 * self.periods_per_year)
                .collect());
        }
        Ok(finalytics_risk::mean_returns(returns, self.mean_method)?
            .means
            .iter()
            .map(|m| m * self.periods_per_year)
            .collect())
    }

    /// An optimisation of the risky assets by `method`, with the limits and parameters the
    /// builder adds; [`Optimise::build`] runs it.
    ///
    /// ```
    /// use finalytics::{Assets, Method, datasets};
    ///
    /// let assets = Assets::builder()
    ///     .prices(datasets::asset_class_prices())
    ///     .risk_free_asset("T-Bills")
    ///     .build()?;
    /// let result = assets.optimise(Method::MaximumSharpe).max_weight(0.4).build()?;
    /// assert!(result.results.weights.unwrap().values().all(|w| *w <= 0.4 + 1e-6));
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn optimise(&self, method: Method) -> Optimise<'_> {
        Optimise {
            assets: self,
            options: OptimiseOptions {
                method,
                ..OptimiseOptions::default()
            },
            name: None,
        }
    }

    /// Optimises a portfolio of the risky assets as `options` describe: the annualised
    /// covariance and mean returns by the assets' estimators (Ledoit–Wolf and the historical
    /// mean unless others were chosen), expected returns given in `options` in place of the
    /// estimates, and the method and limits of `options`. The result holds the optimisation
    /// report and the raw results. [`Assets::optimise`] builds the same options step by step.
    pub fn optimise_with(&self, options: &OptimiseOptions) -> Result<Optimisation> {
        self.check_options(options, true).into_result(())?;
        let results = self.solve(options)?;
        let report = self.optimisation_report(options, &results)?;
        Ok(Optimisation { report, results })
    }

    /// The optimisation report on `results`, the solution of `options`.
    fn optimisation_report(
        &self,
        options: &OptimiseOptions,
        results: &OptimisationResult,
    ) -> Result<OptimisationReport> {
        let (problem, labels) = self.problem(options)?;
        let n = labels.len();
        let covariance = match problem.covariance.clone() {
            Some(c) => c,
            None => Matrix::new(labels.clone(), vec![0.0; n * n])?,
        };
        let (Some(&start), Some(&end)) = (self.dates.first(), self.dates.last()) else {
            let mut report = ValidationReport::new();
            report.error("dates", "there are none");
            return Err(Error::Validation(report));
        };
        let expected = problem
            .expected_returns
            .clone()
            .unwrap_or_else(|| vec![0.0; n]);
        let frontier = if options.frontier {
            problem
                .frontier(FRONTIER_POINTS)
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
        let history = results.weights.as_ref().and_then(|weights| {
            let cash = self.cash_choice(options);
            let invested: Vec<&String> = labels
                .iter()
                .filter(|l| cash.as_ref() != Some(*l))
                .collect();
            let equal: BTreeMap<String, f64> = invested
                .iter()
                .map(|l| ((*l).clone(), 1.0 / invested.len() as f64))
                .collect();
            let mut held = weights.clone();
            let total: f64 = held.values().sum();
            if let Some(cash) = self.risk_free_asset().filter(|_| total < 1.0 - 1e-9) {
                *held.entry(cash.to_owned()).or_default() += 1.0 - total;
            }
            let history = crate::backtest::BacktestOptions {
                strategies: vec![crate::backtest::Strategy::Weights {
                    name: Some("The weights".to_owned()),
                    weights: held,
                }],
                benchmark: Some(crate::backtest::Benchmark::Weights {
                    name: Some("Equal weight".to_owned()),
                    weights: equal,
                }),
                ..crate::backtest::BacktestOptions::default()
            };
            let report = self.backtest_with(&history).ok()?.report;
            Some((report.strategy.tested, report.benchmark?.tested))
        });
        // For minimum CVaR, the share of risk is of the expected shortfall the method minimised,
        // on the same returns: the problem's scenarios, one column per label, cash included.
        let shortfall = match (&results.weights, options.method) {
            (Some(weights), Method::MinimumCvar) => {
                let returns = match &problem.scenarios {
                    Some(scenarios) => scenarios.clone(),
                    None => self.risky_returns()?,
                };
                let w: Vec<f64> = labels
                    .iter()
                    .map(|l| weights.get(l).copied().unwrap_or(0.0))
                    .collect();
                Some(finalytics_risk::shortfall_decomposition(
                    &w,
                    &returns,
                    options.confidence.unwrap_or(0.95),
                )?)
            }
            _ => None,
        };
        // For minimum semivariance and CDaR, the optimum the method reached.
        let optimum = results
            .weights
            .as_ref()
            .and(results.solution.objective_value);
        let semivariance = optimum.filter(|_| options.method == Method::MinimumSemivariance);
        let drawdown_at_risk = optimum
            .filter(|_| options.method == Method::MinimumCdar)
            .map(|v| (options.confidence.unwrap_or(0.95), v));
        let solver = serde_json::to_value(results.solution.method)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        finalytics_reports::optimisation_report(finalytics_reports::OptimisationInputs {
            method: options.method.label().to_owned(),
            description: options.method.description().to_owned(),
            status: results.status.to_string(),
            solver,
            iterations: results.solution.iterations,
            labels: &labels,
            weights: results.weights.as_ref(),
            expected_returns: &expected,
            mean_returns: options
                .expected_returns
                .is_empty()
                .then_some(self.mean_method),
            covariance: &covariance,
            risk_free_asset: self.risk_free_asset().map(str::to_owned),
            cash: labels
                .iter()
                .filter(|l| self.is_cash_label(l))
                .cloned()
                .collect(),
            given: false,
            risk_free_rate: results.risk_free_rate,
            periods_per_year: self.periods_per_year,
            covariance_method: self.covariance_method,
            limits: crate::backtest::limits(options, false),
            start,
            end,
            frontier,
            history,
            shortfall,
            semivariance,
            drawdown_at_risk,
            groups: options
                .groups
                .iter()
                .map(|(name, g)| (name.clone(), self.group_assets(name, g), g.min, g.max))
                .collect(),
            resampling: results.resampled.as_ref().map(|r| {
                finalytics_reports::ResamplingSummary::new(
                    options.resample.unwrap_or_default(),
                    self.returns.observations(),
                    r.solved,
                )
            }),
            current_weights: (!options.current_weights.is_empty())
                .then_some(&options.current_weights),
            benchmark_weights: (!options.benchmark_weights.is_empty())
                .then_some(&options.benchmark_weights),
            options: serde_json::to_string(options).unwrap_or_default(),
        })
    }

    /// Solves the optimisation `options` describe, without a report: the weights a backtest
    /// fits. The options are checked first.
    pub(crate) fn solve(&self, options: &OptimiseOptions) -> Result<OptimisationResult> {
        self.check_options(options, false).into_result(())?;
        self.solve_checked(options)
    }

    /// Solves the optimisation `options` describe, whose options were checked before: a
    /// backtest's refit, whose current weights are those the run holds rather than the user's.
    pub(crate) fn refit(&self, options: &OptimiseOptions) -> Result<OptimisationResult> {
        self.solve_checked(options)
    }

    fn solve_checked(&self, options: &OptimiseOptions) -> Result<OptimisationResult> {
        let rf = options.risk_free_rate.unwrap_or(self.risk_free_rate);
        let (mut problem, labels) = self.problem_unchecked(options)?;
        // The floor on expected return of the downside methods; the frontier and the report's
        // estimates do not have it.
        if options.method.uses_scenarios() {
            problem.constraints.min_return = options.target;
        }
        let objective = objective(options, rf, &labels)?;
        let mut solution = problem
            .solve(&objective)
            .map_err(|e| user_paths(e, options))?;
        // Resampling once the estimates themselves have a solution: without one, the status
        // says why.
        let mut resampled = match options.resample.filter(|_| options.method.resamples()) {
            Some(samples) if solution.status.has_solution() => {
                let resampling = Resampling::new(samples, self.returns.observations(), 0)
                    .with_periods_per_year(self.periods_per_year);
                Some(
                    problem
                        .resampled(&objective, &resampling)
                        .map_err(|e| user_paths(e, options))?,
                )
            }
            _ => None,
        };
        // The weights as the engine gives them: exact where the solver is only close.
        let current = (!options.current_weights.is_empty()).then(|| {
            labels
                .iter()
                .map(|l| options.current_weights.get(l).copied().unwrap_or(0.0))
                .collect::<Vec<f64>>()
        });
        if let Some(w) = solution.weights.as_mut() {
            clean_weights(w, &problem, current.as_deref());
            let (expected, volatility) = figures(&problem, w);
            solution.expected_return = expected;
            solution.volatility = volatility;
        }
        if let Some(r) = resampled.as_mut()
            && let Some(w) = r.weights.as_mut()
        {
            clean_weights(w, &problem, current.as_deref());
            let (expected, volatility) = figures(&problem, w);
            r.expected_return = expected;
            r.volatility = volatility;
        }
        let (status, chosen, expected_return, volatility) = match &resampled {
            Some(r) => (
                if r.weights.is_some() {
                    solution.status
                } else {
                    SolveStatus::NumericalDifficulty
                },
                r.weights.as_ref(),
                r.expected_return,
                r.volatility,
            ),
            None => (
                solution.status,
                solution.weights.as_ref(),
                solution.expected_return,
                solution.volatility,
            ),
        };
        let weights = chosen.map(|w| self.by_label(&labels, w));
        let sharpe_ratio = match (expected_return, volatility) {
            (Some(r), Some(v)) if v > 0.0 => Some((r - rf) / v),
            _ => None,
        };
        Ok(OptimisationResult {
            status,
            weights,
            expected_return,
            volatility,
            sharpe_ratio,
            risk_free_rate: rf,
            solution,
            resampled,
        })
    }

    /// The efficient frontier: `points` portfolios from the one with the least variance to the
    /// one with the largest expected return, under the limits of `options`, each with its
    /// weights by asset. The frontier takes the limits, the expected returns, and the current
    /// and benchmark weights the limits start from; its method is not used, and it refuses a
    /// method's parameters, which it does not use either.
    pub fn frontier(
        &self,
        points: usize,
        options: &OptimiseOptions,
    ) -> Result<Vec<FrontierPortfolio>> {
        let mut report = ValidationReport::new();
        for name in crate::methods::given(options) {
            if !self.frontier_uses(options, name) {
                report.error(
                    format!("options.{name}"),
                    format!("the frontier does not use {name}"),
                );
            }
        }
        if options.frontier {
            report.error("options.frontier", "the frontier does not use frontier");
        }
        // Each point is the least variance for its expected return, whatever the method.
        let mut options = options.clone();
        options.method = Method::MinimumVariance;
        report.merge("", self.check_options(&options, false));
        report.into_result(())?;
        let options = &options;
        let (problem, labels) = self.problem_unchecked(options)?;
        let current = (!options.current_weights.is_empty()).then(|| {
            labels
                .iter()
                .map(|l| options.current_weights.get(l).copied().unwrap_or(0.0))
                .collect::<Vec<f64>>()
        });
        Ok(problem
            .frontier(points)?
            .into_iter()
            .map(|p| {
                let mut w = p.weights;
                clean_weights(&mut w, &problem, current.as_deref());
                let (expected, volatility) = figures(&problem, &w);
                FrontierPortfolio {
                    expected_return: expected.unwrap_or(p.expected_return),
                    volatility: volatility.unwrap_or(p.volatility),
                    weights: self.by_label(&labels, &w),
                }
            })
            .collect())
    }

    /// Every problem with `options` on these assets, at the paths of the options
    /// (`options.<name>`), gathered together: unknown assets, numbers out of range, limits no
    /// fully invested portfolio can meet, and the parameters the method needs. With `single`,
    /// when one method is asked for, a parameter the method does not use is a problem too; a
    /// comparison applies each to the methods that take it.
    #[doc(hidden)]
    pub fn check_options(&self, options: &OptimiseOptions, single: bool) -> ValidationReport {
        let mut report = ValidationReport::new();
        let risky = self.risky_assets();
        let n = risky.len();
        let method = options.method;
        let name = method.in_sentence();
        let unknown = |asset: &str| self.not_chosen(asset);
        let known = |asset: &str| risky.contains(&asset);
        let finite = |report: &mut ValidationReport, value: Option<f64>, path: &str| match value {
            Some(v) if !v.is_finite() => {
                report.error(path, format!("must be a finite number, not {v}"));
                false
            }
            _ => true,
        };
        // A weight's limits: from 0 to 1, the smaller first.
        let share = |report: &mut ValidationReport, min: f64, max: f64, path: &str| {
            if !(min.is_finite() && max.is_finite()) {
                report.error(path, "the limits must be finite numbers");
            } else if min > max {
                report.error(
                    path,
                    format!("the smallest weight {min} is above the largest {max}"),
                );
            } else if min < 0.0 || max > 1.0 {
                report.error(
                    path,
                    "weights are shares of a long-only portfolio, from 0 to 1; for short \
                     positions or leverage, use finalytics.opt.Problem",
                );
            }
        };

        // Limits on weights.
        if options.bounds.is_some() && options.max_weight.is_some() {
            report.error("options.max_weight", "give bounds or max_weight, not both");
        }
        let (default, limit_path) = match (options.max_weight, options.bounds) {
            (Some(max), _) => {
                if !(max.is_finite() && max > 0.0 && max <= 1.0) {
                    report.error(
                        "options.max_weight",
                        format!("must be a share above 0 and at most 1, such as 0.4, not {max}"),
                    );
                }
                (Bound { min: 0.0, max }, "options.max_weight")
            }
            (None, Some(b)) => {
                share(&mut report, b.min, b.max, "options.bounds");
                (b, "options.bounds")
            }
            (None, None) => (Bound { min: 0.0, max: 1.0 }, "options.bounds"),
        };
        let mut lowest = 0.0;
        let mut highest = 0.0;
        for asset in &risky {
            let b = options.asset_bounds.get(*asset).copied().unwrap_or(default);
            lowest += b.min;
            highest += b.max;
        }
        if let Some(b) = options.cash {
            lowest += b.min;
            highest += b.max;
        }
        let kept: f64 = self.kept_cash(options).iter().map(|(_, w)| w).sum();
        lowest += kept;
        highest += kept;
        for (asset, b) in &options.asset_bounds {
            let path = format!("options.asset_bounds.{asset}");
            if known(asset) {
                share(&mut report, b.min, b.max, &path);
            } else {
                report.error(path, unknown(asset));
            }
        }
        if report.is_empty() && method.takes_limits() && n > 0 {
            let path = if options.asset_bounds.is_empty() {
                limit_path
            } else {
                "options.asset_bounds"
            };
            if highest < 1.0 - 1e-9 {
                report.error(
                    path,
                    format!(
                        "the {n} assets{} can hold at most {:.4} of the portfolio together, so \
                         it cannot be fully invested; with {n} assets the largest weight must be \
                         at least {:.4}",
                        if options.cash.is_some() || kept > 0.0 {
                            " and cash"
                        } else {
                            ""
                        },
                        highest,
                        (1.0 - kept) / n as f64
                    ),
                );
            } else if lowest > 1.0 + 1e-9 {
                report.error(
                    path,
                    format!(
                        "the {n} assets must hold at least {lowest:.4} of the portfolio together, \
                         more than all of it"
                    ),
                );
            }
        }
        for (group, g) in &options.groups {
            let path = format!("options.groups.{group}");
            share(&mut report, g.min, g.max, &path);
            for asset in &g.assets {
                if !known(asset) {
                    report.error(&path, unknown(asset));
                }
            }
            match &g.classification {
                Some(_) if !g.assets.is_empty() => report.error(
                    &path,
                    "give the group's assets or a classification, not both",
                ),
                Some(scheme) => match self.classes.get(scheme) {
                    None => {
                        let names: Vec<&str> = self.classes.keys().map(String::as_str).collect();
                        report.error(
                            format!("{path}.classification"),
                            if names.is_empty() {
                                format!(
                                    "no asset is classified by {scheme}: give the assets' classes"
                                )
                            } else {
                                format!(
                                    "no asset is classified by {scheme}; the classifications are \
                                     {}",
                                    names.join(", ")
                                )
                            },
                        );
                    }
                    Some(classes) => {
                        if self.group_assets(group, g).is_empty() {
                            let mut names: Vec<&str> = risky
                                .iter()
                                .filter_map(|a| classes.get(*a).map(String::as_str))
                                .collect();
                            names.sort_unstable();
                            names.dedup();
                            report.error(
                                &path,
                                format!(
                                    "no asset is classed {group} under {scheme}; the classes are {}",
                                    names.join(", ")
                                ),
                            );
                        }
                    }
                },
                None if g.assets.is_empty() => report.error(
                    &path,
                    "give the group's assets, or a classification whose class is its name",
                ),
                None => {}
            }
        }
        if single {
            let info = method.info();
            for limit in crate::methods::given(options)
                .into_iter()
                .filter(|l| crate::methods::is_limit(l) && !info.takes(l))
            {
                report.error(
                    format!("options.{limit}"),
                    crate::methods::refused_limit(&[(name.clone(), method)], limit),
                );
            }
        }

        if let Some(k) = options.min_effective_assets {
            let path = "options.min_effective_assets";
            if !(k.is_finite() && k >= 1.0) {
                report.error(path, format!("must be 1 or more, such as 5, not {k}"));
            } else if k > n as f64 {
                report.error(
                    path,
                    format!("must be at most the number of assets, {n}, not {k}"),
                );
            } else if report.is_empty() && method.takes_limits() {
                let bounds: Vec<Bound> = risky
                    .iter()
                    .map(|a| options.asset_bounds.get(*a).copied().unwrap_or(default))
                    .collect();
                let most = most_effective_assets(&bounds);
                if k > most * (1.0 + 1e-9) {
                    report.error(
                        path,
                        format!(
                            "the limits on weights allow at most {most:.2} effective assets, \
                             fewer than {k}"
                        ),
                    );
                }
            }
        }

        if let Some(b) = options.cash {
            share(&mut report, b.min, b.max, "options.cash");
            if self.risk_free_asset.is_none() && self.labels().iter().any(|l| l == CASH) {
                report.error(
                    "options.cash",
                    format!(
                        "{CASH} is one of the assets: make it the risk-free asset to hold it as \
                         cash"
                    ),
                );
            }
        }
        // Weights held now and a benchmark's: shares of the portfolio, by the rule every input
        // of weights follows, adding up to one with any cash.
        self.check_weights(
            &options.current_weights,
            "options.current_weights",
            false,
            &mut report,
        );
        self.check_weights(
            &options.benchmark_weights,
            "options.benchmark_weights",
            false,
            &mut report,
        );
        let trading = [
            ("max_turnover", options.max_turnover.is_some()),
            ("min_trade", options.min_trade.is_some()),
            ("max_trades", options.max_trades.is_some()),
        ];
        for (field, _) in trading.iter().filter(|(_, given)| *given) {
            if options.current_weights.is_empty() {
                report.error(
                    format!("options.{field}"),
                    "limits trades from the weights held now: give current_weights too",
                );
            }
        }
        if let Some(keep) = options.keep_cash.as_ref().filter(|k| k.keeps_any()) {
            if options.current_weights.is_empty() {
                report.error(
                    "options.keep_cash",
                    "keeps the cash held now at its weight: give current_weights too",
                );
            }
            if let KeepCash::Currencies(list) = keep {
                for (k, code) in list.iter().enumerate() {
                    if code.parse::<Currency>().is_err() {
                        report.error(
                            format!("options.keep_cash[{k}]"),
                            format!("{code:?} is not a currency code, such as USD"),
                        );
                    }
                }
            }
            if options.cash.is_some()
                && let Some(chosen) = self.cash_choice(options)
                && keeps(Some(keep), &chosen)
            {
                report.error(
                    "options.cash",
                    format!(
                        "keep_cash keeps {chosen} at its current weight, so it takes no limits: \
                         give cash or keep_cash for it, not both"
                    ),
                );
            }
        }
        if let Some(t) = options.max_turnover {
            // Optimisation invests the cash held that it does not choose.
            let cash = self.invested_cash(options);
            if !(t.is_finite() && t > 0.0 && t <= 1.0) {
                report.error(
                    "options.max_turnover",
                    format!("must be a share above 0 and at most 1, such as 0.2, not {t}"),
                );
            } else if t < cash / 2.0 - 1e-12 {
                report.error(
                    "options.max_turnover",
                    format!(
                        "investing the {} held in cash turns over {} on its own, more than {}",
                        crate::backtest::percent_text(cash),
                        crate::backtest::percent_text(cash / 2.0),
                        crate::backtest::percent_text(t)
                    ),
                );
            }
        }
        if let Some(m) = options.min_trade
            && !(m.is_finite() && m > 0.0 && m <= 1.0)
        {
            report.error(
                "options.min_trade",
                format!("must be a weight above 0 and at most 1, such as 0.01, not {m}"),
            );
        }
        if let Some(te) = options.max_tracking_error {
            if options.benchmark_weights.is_empty() {
                report.error(
                    "options.max_tracking_error",
                    "limits the tracking error against a benchmark: give benchmark_weights too",
                );
            }
            if !(te.is_finite() && te > 0.0) {
                report.error(
                    "options.max_tracking_error",
                    format!("must be an annual tracking error above 0, such as 0.03, not {te}"),
                );
            }
        }

        // Numbers.
        finite(
            &mut report,
            options.risk_free_rate,
            "options.risk_free_rate",
        );
        for (asset, r) in &options.expected_returns {
            let path = format!("options.expected_returns.{asset}");
            if self.risk_free_asset() == Some(asset.as_str())
                || (asset == CASH && self.cash_at_rate())
            {
                report.error(
                    path,
                    format!("{asset} earns the risk-free rate: give risk_free_rate for another"),
                );
            } else if !self.takes_label(asset) {
                report.error(path, unknown_asset(asset));
            } else {
                finite(&mut report, Some(*r), &path);
            }
        }

        // The method's own parameters, as its description gives them: those it needs, and, for
        // one method, those it does not use. A comparison applies each to the methods that
        // take it, and refuses one that none takes (`Assets::simulate`).
        let info = method.info();
        let given = crate::methods::given(options);
        for parameter in given.iter().filter(|p| !crate::methods::is_limit(p)) {
            if !info.takes(parameter) && single {
                let path = format!("options.{parameter}");
                if method == Method::RiskParity && *parameter == "budgets" {
                    report.error(
                        path,
                        "risk parity gives every asset the same budget; use risk budgeting for others",
                    );
                } else {
                    report.error(path, format!("{name} does not use {parameter}"));
                }
            }
        }
        for parameter in info.parameters.iter().filter(|p| p.required) {
            if !given.contains(&parameter.name.as_str()) {
                report.error(
                    format!("options.{}", parameter.name),
                    format!("{name} needs {}", parameter.name),
                );
            }
        }
        if let Some(target) = options.target {
            finite(&mut report, Some(target), "options.target");
            if method == Method::TargetVolatility && target <= 0.0 {
                report.error(
                    "options.target",
                    "a volatility to aim for must be above zero",
                );
            }
        }
        if let Some(a) = options.risk_aversion
            && finite(&mut report, Some(a), "options.risk_aversion")
            && a <= 0.0
        {
            report.error(
                "options.risk_aversion",
                format!("must be greater than zero, such as 3, not {a}"),
            );
        }
        if let Some(c) = options.confidence
            && !(c > 0.0 && c < 1.0)
        {
            report.error(
                "options.confidence",
                format!("must be between 0 and 1, such as 0.95, not {c}"),
            );
        }
        if let Some(samples) = options.resample
            && samples < 2
        {
            report.error(
                "options.resample",
                format!("must be at least 2 histories, such as 200, not {samples}"),
            );
        }
        if let Some(budgets) = options.budgets.as_ref().filter(|_| info.takes("budgets")) {
            for (asset, b) in budgets {
                let path = format!("options.budgets.{asset}");
                if !known(asset) {
                    report.error(path, unknown(asset));
                } else if !(b.is_finite() && *b >= 0.0) {
                    report.error(path, format!("must be a budget of zero or more, not {b}"));
                }
            }
            for asset in risky.iter().filter(|a| !budgets.contains_key(**a)) {
                report.error("options.budgets", format!("no budget for {asset}"));
            }
        }
        report
    }

    /// The problem `options` describe, over the risky assets, with their labels. The options
    /// are checked first, by [`Assets::check_options`].
    fn problem(&self, options: &OptimiseOptions) -> Result<(Problem, Vec<String>)> {
        self.check_options(options, false).into_result(())?;
        self.problem_unchecked(options)
    }

    /// The problem `options` describe, whose options were checked before.
    fn problem_unchecked(&self, options: &OptimiseOptions) -> Result<(Problem, Vec<String>)> {
        let returns = self.risky_returns()?;
        let mut labels = returns.labels().to_vec();
        // With limits on cash, the cash is one of the choices, riskless: the risk-free asset,
        // earning its own mean return, or `Cash`, earning the risk-free rate.
        let cash = self.cash_choice(options).zip(options.cash);
        if let Some((rf, _)) = &cash {
            labels.push(rf.clone());
        }
        // The cash kept, held at its current weight, as one of the assets.
        let kept = self.kept_cash(options);
        labels.extend(kept.iter().map(|(l, _)| l.clone()));
        // Cash in another currency that a benchmark holds, which optimisation does not choose:
        // held at zero, so that the tracking error counts the benchmark's risk in it.
        let watched: Vec<String> = options
            .benchmark_weights
            .iter()
            .filter(|(l, w)| {
                **w != 0.0
                    && self.held_cash.iter().any(|&i| self.labels()[i] == **l)
                    && !labels.contains(l)
            })
            .map(|(l, _)| l.clone())
            .collect();
        labels.extend(watched.iter().cloned());
        let rate = options.risk_free_rate.unwrap_or(self.risk_free_rate);
        let mut expected = self.expected_of(&labels)?;
        for (label, e) in labels.iter().zip(expected.iter_mut()) {
            if label == CASH && self.cash_at_rate() {
                *e = rate;
            }
        }
        for (asset, r) in &options.expected_returns {
            if let Some(i) = labels.iter().position(|l| l == asset) {
                expected[i] = *r;
            }
        }
        let mut constraints = Constraints::default();
        if let Some(bounds) = options.bounds {
            constraints.bounds = bounds;
        }
        if let Some(max) = options.max_weight {
            constraints.bounds = Bound { min: 0.0, max };
        }
        for (asset, bound) in &options.asset_bounds {
            constraints.asset_bounds.push(AssetBound {
                asset: asset.clone(),
                min: bound.min,
                max: bound.max,
            });
        }
        // Last, so that a problem with it is told apart from the assets' own bounds.
        if let Some((rf, bound)) = &cash {
            constraints.asset_bounds.push(AssetBound {
                asset: rf.clone(),
                min: bound.min,
                max: bound.max,
            });
        }
        for (label, weight) in &kept {
            constraints.asset_bounds.push(AssetBound {
                asset: label.clone(),
                min: *weight,
                max: *weight,
            });
        }
        for label in &watched {
            constraints.asset_bounds.push(AssetBound {
                asset: label.clone(),
                min: 0.0,
                max: 0.0,
            });
        }
        for (name, group) in &options.groups {
            constraints.groups.push(GroupLimit {
                name: name.clone(),
                assets: self.group_assets(name, group),
                min: group.min,
                max: group.max,
            });
        }
        let by_label = |weights: &BTreeMap<String, f64>| -> Vec<f64> {
            labels
                .iter()
                .map(|l| weights.get(l).copied().unwrap_or(0.0))
                .collect()
        };
        if options.max_turnover.is_some()
            || options.min_trade.is_some()
            || options.max_trades.is_some()
        {
            // The turnover counts the cash too. Optimisation invests the cash it does not
            // choose, so a change of that cash's whole weight is part of every portfolio's
            // turnover, and the choices' changes have what the limit leaves; cash that is
            // one of the choices is counted with the others.
            let current: Vec<f64> = labels
                .iter()
                .map(|l| {
                    if watched.contains(l) {
                        0.0
                    } else {
                        options.current_weights.get(l).copied().unwrap_or(0.0)
                    }
                })
                .collect();
            let mut turnover = Turnover::new(current);
            let invested = self.invested_cash(options);
            turnover.limit = options.max_turnover.map(|t| (t - invested / 2.0).max(0.0));
            constraints.turnover = Some(turnover);
        }
        if options.min_trade.is_some() || options.max_trades.is_some() {
            let mut trades = TradeLimits::default();
            trades.minimum = options.min_trade;
            trades.maximum_count = options.max_trades;
            constraints.trades = Some(trades);
        }
        if let Some(limit) = options.max_tracking_error {
            constraints.tracking_error = Some(TrackingErrorLimit {
                benchmark: by_label(&options.benchmark_weights),
                limit,
            });
        }
        let matrix = self.covariance_of(&labels)?;
        constraints.min_effective_assets = options.min_effective_assets;
        let mut problem = Problem::from_covariance(matrix)
            .with_expected_returns(expected)
            .with_constraints(constraints);
        if options.method.uses_scenarios() {
            // Each label's returns period by period: an asset's own, cash in another currency's
            // and the risk-free asset's included, and for `Cash` the risk-free rate in each
            // period, as a backtest's cash earns it.
            let per_period = (1.0 + rate).powf(1.0 / self.periods_per_year) - 1.0;
            let columns = labels
                .iter()
                .map(|l| match self.labels().iter().position(|x| x == l) {
                    Some(i) => self.returns.columns()[i].clone(),
                    None => vec![per_period; self.dates.len()],
                })
                .collect();
            problem = problem.with_scenarios(ReturnsTable::new(labels.clone(), columns)?);
        }
        Ok((problem, labels))
    }
}

/// Whether `keep` keeps the cash `label`: every cash for `true`, and for a list of currencies
/// the cash named for one of them, such as `EUR cash`.
pub(crate) fn keeps(keep: Option<&KeepCash>, label: &str) -> bool {
    match keep {
        Some(KeepCash::All(all)) => *all,
        Some(KeepCash::Currencies(list)) => match label.parse::<HoldingKey>() {
            Ok(HoldingKey::Cash(currency)) => list
                .iter()
                .any(|code| code.parse::<Currency>().ok() == Some(currency)),
            _ => false,
        },
        None => false,
    }
}

/// The one message for a label that is not one of the assets, in every input that takes
/// weights or values by asset.
pub(crate) fn unknown_asset(asset: &str) -> String {
    format!("unknown asset {asset}")
}

/// `error` with the optimiser's paths (`constraints.bounds`, `objective.confidence`) turned into
/// those of the options that set them (`options.max_weight`, `options.confidence`).
fn user_paths(error: Error, options: &OptimiseOptions) -> Error {
    let Error::Validation(report) = error else {
        return error;
    };
    let nth = |keys: Vec<&String>, rest: &str| -> Option<String> {
        let (k, _) = rest.strip_prefix('[')?.split_once(']')?;
        keys.get(k.parse::<usize>().ok()?).map(|key| (*key).clone())
    };
    let mut mapped = ValidationReport::new();
    for issue in report.issues() {
        let path = &issue.path;
        let user = if path == "constraints.bounds" {
            if options.max_weight.is_some() {
                "options.max_weight".to_owned()
            } else {
                "options.bounds".to_owned()
            }
        } else if let Some(rest) = path.strip_prefix("constraints.asset_bounds") {
            nth(options.asset_bounds.keys().collect(), rest).map_or_else(
                || {
                    if options.cash.is_some() {
                        "options.cash".to_owned()
                    } else {
                        "options.asset_bounds".to_owned()
                    }
                },
                |a| format!("options.asset_bounds.{a}"),
            )
        } else if let Some(rest) = path.strip_prefix("constraints.groups") {
            nth(options.groups.keys().collect(), rest).map_or_else(
                || "options.groups".to_owned(),
                |g| format!("options.groups.{g}"),
            )
        } else if path == "constraints.min_effective_assets" {
            "options.min_effective_assets".to_owned()
        } else if path.starts_with("constraints.turnover") {
            "options.max_turnover".to_owned()
        } else if path == "constraints.trades.minimum" {
            "options.min_trade".to_owned()
        } else if path.starts_with("constraints.trades") {
            if options.min_trade.is_some() {
                "options.min_trade".to_owned()
            } else {
                "options.max_trades".to_owned()
            }
        } else if path.starts_with("constraints.tracking_error") {
            "options.max_tracking_error".to_owned()
        } else if path == "constraints.min_return" {
            "options.target".to_owned()
        } else if path.starts_with("resampling") {
            "options.resample".to_owned()
        } else if let Some(rest) = path.strip_prefix("objective.") {
            format!("options.{rest}")
        } else if path == "constraints" || path == "objective" {
            "options".to_owned()
        } else {
            path.clone()
        };
        let mut one = ValidationReport::new();
        match issue.severity {
            finalytics_core::Severity::Error => one.error(user, issue.message.clone()),
            _ => one.warning(user, issue.message.clone()),
        }
        mapped.merge("", one);
    }
    Error::Validation(mapped)
}

/// The objective `options` name, with its parameters.
fn objective(
    options: &OptimiseOptions,
    risk_free_rate: f64,
    labels: &[String],
) -> Result<Objective> {
    let mut report = ValidationReport::new();
    let mut need = |value: Option<f64>, name: &str| -> f64 {
        value.unwrap_or_else(|| {
            report.error(
                format!("options.{name}"),
                format!("{} needs {name}", options.method.in_sentence()),
            );
            0.0
        })
    };
    let objective = match options.method {
        Method::MinimumVariance => Objective::MinimumVariance,
        Method::MeanVariance => Objective::MeanVariance {
            risk_aversion: need(options.risk_aversion, "risk_aversion"),
        },
        Method::RobustMeanVariance => {
            let risk_aversion = need(options.risk_aversion, "risk_aversion");
            match options.uncertainty.clone() {
                Some(uncertainty) => Objective::RobustMeanVariance {
                    risk_aversion,
                    uncertainty,
                },
                None => {
                    report.error(
                        "options.uncertainty",
                        "robust_mean_variance needs uncertainty",
                    );
                    Objective::MinimumVariance
                }
            }
        }
        Method::TargetReturn => Objective::TargetReturn {
            target: need(options.target, "target"),
        },
        Method::TargetVolatility => Objective::TargetVolatility {
            target: need(options.target, "target"),
        },
        Method::MaximumSharpe => Objective::MaximumSharpe { risk_free_rate },
        Method::MaximumDiversification => Objective::MaximumDiversification,
        Method::MinimumCvar => Objective::MinimumCvar {
            confidence: options.confidence.unwrap_or(0.95),
        },
        Method::MinimumSemivariance => Objective::MinimumSemivariance,
        Method::MinimumCdar => Objective::MinimumCdar {
            confidence: options.confidence.unwrap_or(0.95),
        },
        Method::EqualWeight => Objective::EqualWeight,
        Method::InverseVolatility => Objective::InverseVolatility,
        Method::RiskParity => Objective::RiskBudgeting { budgets: None },
        Method::RiskBudgeting => Objective::RiskBudgeting {
            budgets: options.budgets.as_ref().map(|by_asset| {
                labels
                    .iter()
                    .map(|l| by_asset.get(l).copied().unwrap_or(0.0))
                    .collect()
            }),
        },
        Method::HierarchicalRiskParity => Objective::HierarchicalRiskParity {
            linkage: options.linkage.unwrap_or_default(),
        },
    };
    report.into_result(objective)
}

/// An optimisation over [`Assets`], built step by step; [`Optimise::build`] runs it. The
/// limits apply to the methods that take them: equal weight, inverse volatility, risk parity,
/// risk budgeting and hierarchical risk parity are fully invested and long only, and take none.
#[derive(Debug, Clone)]
#[must_use = "an optimisation does nothing until `build` runs it"]
pub struct Optimise<'a> {
    assets: &'a Assets,
    options: OptimiseOptions,
    name: Option<String>,
}

impl Optimise<'_> {
    /// The largest weight of any asset, such as 0.4 for 40%.
    pub fn max_weight(mut self, max: f64) -> Self {
        self.options.max_weight = Some(max);
        self
    }

    /// Every asset's weight between `min` and `max`.
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.options.bounds = Some(Bound { min, max });
        self
    }

    /// One asset's weight between `min` and `max`.
    pub fn asset_bounds(mut self, asset: &str, min: f64, max: f64) -> Self {
        self.options
            .asset_bounds
            .insert(asset.to_owned(), Bound { min, max });
        self
    }

    /// The total weight of the `assets` of group `name` between `min` and `max`, such as a
    /// sector's.
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

    /// The total weight of the assets classed `class` under `scheme`, such as `Equity` under
    /// `asset_class`, between `min` and `max`.
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

    /// The annual expected return (target return) or volatility (target volatility) to aim
    /// for, or the smallest annual expected return for minimum CVaR, semivariance and CDaR.
    pub fn target(mut self, target: f64) -> Self {
        self.options.target = Some(target);
        self
    }

    /// The smallest effective number of assets, `1 / sum w^2`, such as 5.
    pub fn min_effective_assets(mut self, k: f64) -> Self {
        self.options.min_effective_assets = Some(k);
        self
    }

    /// Resampled optimisation: the average of the optimal weights for `samples` histories,
    /// such as 200, simulated from the estimates.
    pub fn resample(mut self, samples: usize) -> Self {
        self.options.resample = Some(samples);
        self
    }

    /// The weight on variance, for mean–variance methods.
    pub fn risk_aversion(mut self, risk_aversion: f64) -> Self {
        self.options.risk_aversion = Some(risk_aversion);
        self
    }

    /// The uncertainty of the expected returns, for robust mean–variance.
    pub fn uncertainty(mut self, uncertainty: ReturnUncertainty) -> Self {
        self.options.uncertainty = Some(uncertainty);
        self
    }

    /// The confidence of the conditional value at risk or drawdown at risk, for minimum CVaR
    /// and CDaR: 0.95 by default.
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.options.confidence = Some(confidence);
        self
    }

    /// Each asset's risk budget, for risk budgeting; equal shares without them.
    pub fn budget(mut self, asset: &str, budget: f64) -> Self {
        self.options
            .budgets
            .get_or_insert_with(BTreeMap::new)
            .insert(asset.to_owned(), budget);
        self
    }

    /// How hierarchical risk parity joins clusters: single linkage by default.
    pub fn linkage(mut self, linkage: Linkage) -> Self {
        self.options.linkage = Some(linkage);
        self
    }

    /// An annual risk-free rate for the Sharpe ratio, in place of the assets' own.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.options.risk_free_rate = Some(rate);
        self
    }

    /// An asset's annual expected return, in place of its historical mean.
    pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
        self.options
            .expected_returns
            .insert(asset.to_owned(), value);
        self
    }

    /// The weights held now, by asset, such as `[("US Equity", 0.5), ("Bonds", 0.5)]`, adding up
    /// to one with the cash, such as a portfolio's own weights: the report lists the trades from
    /// them, and the turnover and trade limits start from them.
    pub fn current_weights<S: Into<String>>(
        mut self,
        weights: impl IntoIterator<Item = (S, f64)>,
    ) -> Self {
        self.options.current_weights = weights.into_iter().map(|(a, w)| (a.into(), w)).collect();
        self
    }

    /// The largest one-way turnover from the current weights, such as 0.2 for 20% of the
    /// portfolio bought and as much sold.
    pub fn max_turnover(mut self, turnover: f64) -> Self {
        self.options.max_turnover = Some(turnover);
        self
    }

    /// The smallest trade, as a weight, such as 0.01: each asset is traded by at least this
    /// much, or not at all.
    pub fn min_trade(mut self, weight: f64) -> Self {
        self.options.min_trade = Some(weight);
        self
    }

    /// The largest number of assets traded from the current weights.
    pub fn max_trades(mut self, count: usize) -> Self {
        self.options.max_trades = Some(count);
        self
    }

    /// A benchmark's weights by asset, adding up to one with the cash:
    /// the report gives the tracking error against it, and `max_tracking_error` limits it.
    pub fn benchmark_weights<S: Into<String>>(
        mut self,
        weights: impl IntoIterator<Item = (S, f64)>,
    ) -> Self {
        self.options.benchmark_weights = weights.into_iter().map(|(a, w)| (a.into(), w)).collect();
        self
    }

    /// The largest ex-ante tracking error a year against the benchmark weights, such as 0.03.
    pub fn max_tracking_error(mut self, limit: f64) -> Self {
        self.options.max_tracking_error = Some(limit);
        self
    }

    /// Cash between `min` and `max` of the portfolio, such as 0 and 0.2, beside the risky
    /// assets: riskless, held in the risk-free asset and earning its return, or, without one,
    /// in `Cash` earning the risk-free rate.
    pub fn cash(mut self, min: f64, max: f64) -> Self {
        self.options.cash = Some(Bound { min, max });
        self
    }

    /// Keeps the cash of `currencies`, such as `["USD"]`, at its current weight, as
    /// [`RebalanceBuilder::keep_cash`](crate::RebalanceBuilder::keep_cash) keeps it: the rest is
    /// optimised, and the limits are measured on the whole portfolio. It needs the current
    /// weights.
    pub fn keep_cash<S: AsRef<str>>(mut self, currencies: impl IntoIterator<Item = S>) -> Self {
        self.options.keep_cash = Some(KeepCash::Currencies(
            currencies
                .into_iter()
                .map(|c| c.as_ref().to_owned())
                .collect(),
        ));
        self
    }

    /// Keeps every currency's cash at its current weight, as [`Optimise::keep_cash`] does for
    /// some.
    pub fn keep_all_cash(mut self) -> Self {
        self.options.keep_cash = Some(KeepCash::All(true));
        self
    }

    /// Draws the efficient frontier under the same limits in the optimisation report. It takes
    /// 30 more optimisations, so it adds time that grows with the number of assets, to seconds
    /// for hundreds of assets; without it, the report shows the weights against each asset and
    /// equal weight.
    pub fn with_frontier(mut self) -> Self {
        self.options.frontier = true;
        self
    }

    /// The options built so far.
    pub fn options(&self) -> &OptimiseOptions {
        &self.options
    }

    /// The report's title, such as `"Defensive equity"`: the method's name by default.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// Runs the optimisation: the weights in `results`, with the optimisation report in
    /// `report`.
    pub fn build(&self) -> Result<Optimisation> {
        let mut optimisation = self.assets.optimise_with(&self.options)?;
        optimisation.report.name.clone_from(&self.name);
        Ok(optimisation)
    }

    /// The efficient frontier under the limits built so far.
    pub fn frontier(&self, points: usize) -> Result<Vec<FrontierPortfolio>> {
        self.assets.frontier(points, &self.options)
    }
}

/// [`Assets`] built step by step from prices or returns; [`AssetsBuilder::build`] makes them.
///
/// ```
/// use finalytics::{Assets, datasets};
///
/// let assets = Assets::builder()
///     .prices(datasets::asset_class_prices())
///     .risk_free_asset("T-Bills")
///     .build()?;
/// assert_eq!(assets.periods_per_year(), 12.0);
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "the assets are made by `build`"]
pub struct AssetsBuilder {
    prices: Option<PriceTable>,
    csv: Option<String>,
    #[cfg(feature = "io")]
    file: Option<std::path::PathBuf>,
    returns: Option<(Vec<Date>, ReturnsTable)>,
    risk_free_asset: Option<String>,
    risk_free_rate: Option<f64>,
    periods_per_year: Option<f64>,
    covariance: Option<CovarianceMethod>,
    mean_returns: Option<MeanMethod>,
    classes: BTreeMap<String, BTreeMap<String, String>>,
}

impl AssetsBuilder {
    /// Each asset's class under the classification `scheme`, such as `asset_class`, for group
    /// limits by class: `.classes("asset_class", [("US Equity", "Equity"), ("Bonds", "Fixed
    /// income")])`. Repeatable, one scheme at a time.
    pub fn classes<S: Into<String>, C: Into<String>>(
        mut self,
        scheme: &str,
        classes: impl IntoIterator<Item = (S, C)>,
    ) -> Self {
        self.classes.insert(
            scheme.to_owned(),
            classes
                .into_iter()
                .map(|(a, c)| (a.into(), c.into()))
                .collect(),
        );
        self
    }

    /// The assets' prices: a date column and one column per asset. Prices should include
    /// income, as adjusted closes do.
    pub fn prices(mut self, prices: PriceTable) -> Self {
        self.prices = Some(prices);
        self
    }

    /// The assets' prices as CSV text: a `date` column and one column per asset, or the long
    /// layout of `date`, `instrument` and `price` columns.
    pub fn prices_csv(mut self, text: &str) -> Self {
        self.csv = Some(text.to_owned());
        self
    }

    /// The assets' prices from a CSV file, as [`AssetsBuilder::prices_csv`] reads them.
    #[cfg(feature = "io")]
    pub fn prices_file(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.file = Some(path.as_ref().to_owned());
        self
    }

    /// The assets' returns, in place of prices: `dates` are the dates each return ends on.
    pub fn returns(mut self, dates: Vec<Date>, returns: ReturnsTable) -> Self {
        self.returns = Some((dates, returns));
        self
    }

    /// One of the assets, such as T-bills, as the risk-free asset: its mean return is the
    /// risk-free rate, and optimisation leaves it out.
    pub fn risk_free_asset(mut self, asset: &str) -> Self {
        self.risk_free_asset = Some(asset.to_owned());
        self
    }

    /// An annual risk-free rate, for Sharpe ratios; zero by default.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = Some(rate);
        self
    }

    /// The number of periods in a year, in place of the one detected from the dates: 252 for
    /// daily returns, 52 for weekly, 12 for monthly.
    pub fn periods_per_year(mut self, periods: f64) -> Self {
        self.periods_per_year = Some(periods);
        self
    }

    /// The covariance estimator: Ledoit–Wolf by default.
    pub fn covariance(mut self, method: CovarianceMethod) -> Self {
        self.covariance = Some(method);
        self
    }

    /// The estimator of mean returns: the historical mean by default.
    pub fn mean_returns(mut self, method: MeanMethod) -> Self {
        self.mean_returns = Some(method);
        self
    }

    /// Makes the assets, reporting every problem with what was given.
    pub fn build(self) -> Result<Assets> {
        let mut report = ValidationReport::new();
        #[cfg(feature = "io")]
        let file = match &self.file {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(text) => Some(text),
                Err(e) => {
                    report.error("prices", format!("cannot read {}: {e}", path.display()));
                    None
                }
            },
            None => None,
        };
        #[cfg(not(feature = "io"))]
        let file: Option<String> = None;
        let given = usize::from(self.prices.is_some())
            + usize::from(self.csv.is_some())
            + usize::from(file.is_some())
            + usize::from(self.returns.is_some());
        if given != 1 && report.is_empty() {
            report.error("prices", "give the assets' prices, or their returns, once");
        }
        let settings = AssetSettings {
            risk_free_rate: self.risk_free_rate,
            risk_free_asset: self.risk_free_asset,
            periods_per_year: self.periods_per_year,
            covariance: self.covariance,
            mean_returns: self.mean_returns,
            classes: (!self.classes.is_empty()).then_some(self.classes),
        };
        settings.check(&mut report);
        report.into_result(())?;
        let assets = if let Some(prices) = &self.prices {
            Assets::from_prices(prices)?
        } else if let Some(text) = self.csv.as_deref().or(file.as_deref()) {
            Assets::from_csv(text)?
        } else if let Some((dates, returns)) = self.returns {
            Assets::from_returns(dates, returns)?
        } else {
            unreachable!("one source of prices or returns was checked above")
        };
        settings.apply(assets)
    }
}

/// The settings of [`Assets`] as JSON gives them, with the names of the builder's methods: the
/// Python and JavaScript packages use it; from Rust, use the builder.
#[doc(hidden)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssetSettings {
    pub risk_free_rate: Option<f64>,
    pub risk_free_asset: Option<String>,
    pub periods_per_year: Option<f64>,
    pub covariance: Option<CovarianceMethod>,
    pub mean_returns: Option<MeanMethod>,
    pub classes: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

impl AssetSettings {
    /// Adds each problem with the settings to `report`.
    pub fn check(&self, report: &mut ValidationReport) {
        if self.risk_free_asset.is_some() && self.risk_free_rate.is_some() {
            report.error(
                "risk_free_rate",
                "give a risk-free asset or a risk-free rate, not both",
            );
        }
        if let Some(rate) = self.risk_free_rate
            && !(rate.is_finite() && rate > -1.0)
        {
            report.error(
                "risk_free_rate",
                format!("must be an annual rate above -1, such as 0.04, not {rate}"),
            );
        }
        if let Some(periods) = self.periods_per_year
            && !(periods.is_finite() && periods > 0.0)
        {
            report.error(
                "periods_per_year",
                format!("must be greater than zero, such as 12 for monthly returns, not {periods}"),
            );
        }
        if let Some(CovarianceMethod::Ewma { decay }) = self.covariance
            && !(decay > 0.0 && decay < 1.0)
        {
            report.error(
                "covariance.decay",
                format!("must be between 0 and 1, such as 0.94, not {decay}"),
            );
        }
        if let Some(CovarianceMethod::Statistical { factors: 0 }) = self.covariance {
            report.error("covariance.factors", "must be at least 1, such as 3");
        }
        if let Some(MeanMethod::Ewma { decay }) = self.mean_returns
            && !(decay > 0.0 && decay < 1.0)
        {
            report.error(
                "mean_returns.decay",
                format!("must be between 0 and 1, such as 0.94, not {decay}"),
            );
        }
    }

    /// `assets` with these settings, every problem with them reported together.
    pub fn apply(&self, mut assets: Assets) -> Result<Assets> {
        // The settings' own checks, the risk-free asset and the classes are reported together;
        // a setting with a problem is left out while the others are checked.
        let mut report = ValidationReport::new();
        self.check(&mut report);
        let bad = |name: &str| report.issues().iter().any(|i| i.path.starts_with(name));
        if let Some(periods) = self.periods_per_year.filter(|_| !bad("periods_per_year")) {
            assets = assets.with_periods_per_year(periods);
        }
        if let Some(method) = self.covariance.filter(|_| !bad("covariance")) {
            assets = assets.with_covariance(method);
        }
        if let Some(rate) = self.risk_free_rate.filter(|_| !bad("risk_free_rate")) {
            assets = assets.with_risk_free_rate(rate);
        }
        if let Some(method) = self.mean_returns.filter(|_| !bad("mean_returns")) {
            assets.mean_method = method;
        }
        if let Some(asset) = &self.risk_free_asset {
            match assets.clone().with_risk_free_asset(asset) {
                Ok(with) => assets = with,
                Err(Error::Validation(found)) => report.merge("", found),
                Err(e) => return Err(e),
            }
        }
        for (scheme, classes) in self.classes.iter().flatten() {
            match assets.clone().with_classes(scheme, classes.clone()) {
                Ok(with) => assets = with,
                Err(Error::Validation(found)) => report.merge("", found),
                Err(e) => return Err(e),
            }
        }
        report.into_result(())?;
        // The estimators chosen are checked on the risky assets' returns, as optimisation uses
        // them, so that a choice the returns cannot support is reported here.
        let mut report = ValidationReport::new();
        let risky = assets.risky_returns()?;
        if let Some(method) = self.covariance
            && let Err(e) = covariance(&risky, method)
        {
            rehome(e, "covariance", &mut report)?;
        }
        if self.mean_returns.is_some()
            && let Err(e) = assets.risky_means(&risky)
        {
            rehome(e, "mean_returns", &mut report)?;
        }
        report.into_result(assets)
    }
}

/// Adds the problems of `error`, if it is a validation error, to `report` at `path`, the setting
/// that caused them; any other error is returned.
fn rehome(error: Error, path: &str, report: &mut ValidationReport) -> Result<()> {
    let Error::Validation(found) = error else {
        return Err(error);
    };
    for issue in found.issues() {
        report.error(path, issue.message.clone());
    }
    Ok(())
}

/// Makes the solver's weights `w` of `problem` exact where the solver is only close, so that
/// the engine's own weights pass its own checks: each within its bounds; a weight within the
/// no-trade tolerance ([`TRADED`](finalytics_opt::TRADED)) of the weight held now, when
/// `current` gives those, at that weight, so that an asset not traded is not traded by a
/// fraction; another within the tolerance of zero at zero; and what that changes in their total
/// taken up by the other weights in proportion, so that they add up to the budget.
pub(crate) fn clean_weights(w: &mut [f64], problem: &Problem, current: Option<&[f64]>) {
    let tolerance = finalytics_opt::TRADED;
    let constraints = &problem.constraints;
    let bounds: Vec<(f64, f64)> = problem
        .labels
        .iter()
        .map(|label| {
            let mut limits = constraints
                .asset_bounds
                .iter()
                .filter(|b| &b.asset == label)
                .peekable();
            if limits.peek().is_none() {
                return (constraints.bounds.min, constraints.bounds.max);
            }
            limits.fold((f64::NEG_INFINITY, f64::INFINITY), |(lo, hi), b| {
                (lo.max(b.min), hi.min(b.max))
            })
        })
        .collect();
    let mut fixed = vec![false; w.len()];
    for (i, weight) in w.iter_mut().enumerate() {
        let (lo, hi) = bounds[i];
        if lo <= hi {
            *weight = weight.clamp(lo, hi);
        }
        if let Some(c) = current.and_then(|c| c.get(i))
            && (*weight - c).abs() < tolerance
            && (lo..=hi).contains(c)
        {
            *weight = *c;
            fixed[i] = true;
        } else if weight.abs() < tolerance && (lo..=hi).contains(&0.0) {
            *weight = 0.0;
            fixed[i] = true;
        } else if *weight == lo || *weight == hi {
            fixed[i] = lo == hi;
        }
    }
    // The gap is taken up by the weights free to move, not held as they are and with room
    // before their bounds, in proportion; one that reaches a bound stops there, and the rest
    // take what it leaves.
    for _ in 0..=w.len() {
        let gap = constraints.budget - w.iter().sum::<f64>();
        if gap.abs() <= f64::EPSILON || !gap.is_finite() {
            return;
        }
        let free: Vec<usize> = (0..w.len())
            .filter(|&i| {
                let (lo, hi) = bounds[i];
                !fixed[i] && w[i] > 0.0 && if gap > 0.0 { w[i] < hi } else { w[i] > lo }
            })
            .collect();
        let total: f64 = free.iter().map(|&i| w[i]).sum();
        if total <= 0.0 {
            return;
        }
        for i in free {
            let (lo, hi) = bounds[i];
            w[i] = (w[i] + gap * w[i] / total).clamp(lo.min(hi), hi.max(lo));
        }
    }
}

/// The annual expected return and volatility of weights `w` on the estimates of `problem`.
fn figures(problem: &Problem, w: &[f64]) -> (Option<f64>, Option<f64>) {
    let expected = problem
        .expected_returns
        .as_ref()
        .map(|mu| mu.iter().zip(w).map(|(m, w)| m * w).sum::<f64>());
    let volatility = problem.covariance.as_ref().map(|c| {
        let n = w.len();
        let variance: f64 = (0..n)
            .map(|i| w[i] * (0..n).map(|j| c.get(i, j) * w[j]).sum::<f64>())
            .sum();
        variance.max(0.0).sqrt()
    });
    (expected, volatility)
}

/// The largest effective number of assets, `1 / sum w^2`, of fully invested weights within
/// `bounds`: the weights nearest equal, `clamp(level, min, max)` at the level where they sum to
/// one. The bounds must allow a fully invested portfolio.
fn most_effective_assets(bounds: &[Bound]) -> f64 {
    let total = |level: f64| -> f64 { bounds.iter().map(|b| level.clamp(b.min, b.max)).sum() };
    let mut low = bounds.iter().map(|b| b.min).fold(f64::INFINITY, f64::min);
    let mut high = bounds
        .iter()
        .map(|b| b.max)
        .fold(f64::NEG_INFINITY, f64::max);
    for _ in 0..200 {
        let mid = 0.5 * (low + high);
        if total(mid) < 1.0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    let level = 0.5 * (low + high);
    1.0 / bounds
        .iter()
        .map(|b| level.clamp(b.min, b.max).powi(2))
        .sum::<f64>()
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}
