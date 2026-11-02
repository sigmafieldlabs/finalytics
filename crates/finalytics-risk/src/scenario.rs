//! Scenarios: what shocks to instruments, groups of instruments, currencies or factors, or a
//! replay of a historical period, would do to a portfolio's value and risk.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Decimal, Error, Result, TimeSeries, ValidationReport};
use finalytics_ledger::{Books, HoldingKey, InstrumentId, MarketData, Valuation};

use crate::decomposition::{GroupRisk, RiskContribution, group_risk, risk_decomposition};
use crate::distribution::normal_quantile;
use crate::drawdown::{Drawdown, drawdowns};
use crate::factor::FactorModel;
use crate::holdings::observe;
use crate::input::invalid;
use crate::linalg::symmetric_eigen;
use crate::matrix::Matrix;

/// A return for every instrument with a classification, such as all equities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GroupShock {
    /// The classification scheme, such as `asset_class`.
    pub scheme: String,
    /// The classification, such as `equity`.
    pub value: String,
    /// The return of each instrument in the group, in its own currency: -0.2 for a fall of 20%.
    pub shock: f64,
}

/// Returns of a factor model's factors, which give each instrument in the model the return its
/// exposures imply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FactorShocks {
    /// The model. Its assets are instrument identifiers; assets that are not held are ignored.
    pub model: FactorModel,
    /// The return of each shocked factor, by factor name.
    pub shocks: Vec<(String, f64)>,
    /// Whether the factors that are not shocked move too, by their expected return given the
    /// shocked ones under the model's factor covariance. Without it, they return zero.
    #[serde(default)]
    pub propagate: bool,
}

/// The shocks of a scenario. An instrument's return in its own currency comes from its own
/// shock, or else from the one group shock that matches it, or else from the factor shocks; an
/// instrument none of them reaches returns zero and is listed as not shocked. A currency shock
/// compounds with the instrument's return, and applies to cash in that currency.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Shocks {
    /// Returns of single instruments, by identifier.
    #[serde(default)]
    pub instruments: Vec<(InstrumentId, f64)>,
    /// Returns of every instrument in a group. An instrument without its own shock may match at
    /// most one of them.
    #[serde(default)]
    pub groups: Vec<GroupShock>,
    /// Changes in the value of currencies in the base currency: -0.1 when a currency loses 10%.
    #[serde(default)]
    pub currencies: Vec<(Currency, f64)>,
    /// Returns of factors.
    #[serde(default)]
    pub factors: Option<FactorShocks>,
}

/// A historical period to replay on today's holdings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HistoricalWindow {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// Instruments whose prices stand in for others' over the period, such as an index for a
    /// fund that did not exist then: `(instrument, proxy)` pairs.
    #[serde(default)]
    pub proxies: Vec<(InstrumentId, InstrumentId)>,
}

/// A stress of a covariance matrix: volatilities scaled and correlations moved toward one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct CorrelationStress {
    /// The factor every volatility is multiplied by. The default is 1.
    pub volatility_scale: f64,
    /// How far every correlation moves toward one, from 0 (not at all, the default) to 1 (all
    /// the way).
    pub correlation_shift: f64,
}

impl Default for CorrelationStress {
    fn default() -> Self {
        CorrelationStress {
            volatility_scale: 1.0,
            correlation_shift: 0.0,
        }
    }
}

/// The risk model for the risk before and after a scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioRiskModel {
    /// The covariance of the holdings' returns, labelled with instrument identifiers and, for
    /// cash, the currency code followed by " cash" (as holding returns are). Give this or
    /// `factor_model`.
    #[serde(default)]
    pub covariance: Option<Matrix>,
    /// A factor model of the holdings' returns, labelled in the same way.
    #[serde(default)]
    pub factor_model: Option<FactorModel>,
    /// The confidence of the value at risk. The default is 0.95.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// A stress of the covariance for the risk after the scenario, as in a crisis.
    #[serde(default)]
    pub stress: Option<CorrelationStress>,
}

fn default_confidence() -> f64 {
    0.95
}

/// What to report beyond the effect on each holding.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioOptions {
    /// Classification schemes to add the effect up by, such as `asset_class`. Instruments
    /// without the classification are in the group "unclassified", and cash in "cash".
    #[serde(default)]
    pub group_by: Vec<String>,
    /// The risk model for the risk before and after; without one, no risk is reported.
    #[serde(default)]
    pub risk: Option<ScenarioRiskModel>,
}

/// How a holding's return in a scenario was set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ShockSource {
    /// Its own shock.
    Instrument,
    /// A group shock.
    Group {
        /// The group's classification scheme.
        scheme: String,
        /// The group's classification.
        value: String,
    },
    /// The factor shocks and its exposures.
    Factor,
    /// Its prices over a historical period.
    History,
    /// Another instrument's prices over a historical period.
    Proxy {
        /// The instrument whose prices were used.
        instrument: InstrumentId,
    },
    /// Cash: only a change in its currency's value moves it.
    Cash,
    /// Nothing: its return in its own currency is zero.
    None,
}

/// What a scenario does to one holding: an instrument, or cash in one currency, added up across
/// accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingImpact {
    /// The holding.
    pub holding: HoldingKey,
    /// Its value before, in the base currency.
    pub value_before: f64,
    /// Its share of the portfolio's value before.
    pub weight_before: f64,
    /// Its return in its own currency.
    pub local_return: f64,
    /// The change in its currency's value in the base currency.
    pub currency_return: f64,
    /// Its return in the base currency, `(1 + local)(1 + currency) - 1`.
    pub total_return: f64,
    /// Its value after, in the base currency.
    pub value_after: f64,
    /// Its share of the portfolio's value after, or `None` if the portfolio is worth nothing
    /// after.
    pub weight_after: Option<f64>,
    /// `value_after - value_before`.
    pub change: f64,
    /// How its return was set.
    pub source: ShockSource,
}

/// What a scenario does to one group of holdings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GroupImpact {
    /// The group.
    pub group: String,
    /// Its value before, in the base currency.
    pub value_before: f64,
    /// Its value after.
    pub value_after: f64,
    /// `value_after - value_before`.
    pub change: f64,
    /// Its share of the portfolio's value before.
    pub weight_before: f64,
    /// Its share of the portfolio's value after, or `None` if the portfolio is worth nothing
    /// after.
    pub weight_after: Option<f64>,
    /// `change` as a share of the portfolio's value before: the groups' contributions add up to
    /// the portfolio's return.
    pub contribution: f64,
}

/// The effect of a scenario by the groups of one classification scheme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemeImpact {
    /// The classification scheme.
    pub scheme: String,
    /// Each group, in name order.
    pub groups: Vec<GroupImpact>,
}

/// Risk by the groups of one classification scheme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemeRisk {
    /// The classification scheme.
    pub scheme: String,
    /// Each group's share of the volatility, in name order.
    pub groups: Vec<GroupRisk>,
}

/// A portfolio's risk before or after a scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioRisk {
    /// The volatility of the portfolio's return, in the units of the risk model (scale the
    /// model to annualise).
    pub volatility: f64,
    /// The parametric value at risk of the portfolio's value at the model's confidence, a loss
    /// in the base currency: `z * volatility * value`.
    pub value_at_risk: f64,
    /// Each holding's contribution to the volatility, in the order of the holdings.
    pub contributions: Vec<RiskContribution>,
    /// The contributions added up by each classification scheme the options group by.
    pub groups: Vec<SchemeRisk>,
}

/// The hypothetical value of today's holdings over a historical period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioPath {
    /// Every date in the period with a price for a holding, and its start and end.
    pub dates: Vec<Date>,
    /// The holdings' value on each date, in the base currency.
    pub values: Vec<f64>,
    /// The largest fall from a peak over the period, `None` if the value never falls or is not
    /// always above zero.
    pub worst_drawdown: Option<Drawdown>,
}

/// What a scenario does to a portfolio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioResult {
    /// The date of the holdings.
    pub as_of: Date,
    /// The currency values are in.
    pub base_currency: Currency,
    /// The portfolio's value before.
    pub value_before: f64,
    /// The portfolio's value after.
    pub value_after: f64,
    /// `value_after - value_before`.
    pub change: f64,
    /// `change / value_before`.
    pub portfolio_return: f64,
    /// Each holding, in holding order: instruments by identifier, then cash by currency.
    pub holdings: Vec<HoldingImpact>,
    /// The instruments held whose return in their own currency no shock set.
    pub not_shocked: Vec<InstrumentId>,
    /// The factor returns applied, given and propagated, in the order of the model's factors;
    /// empty without factor shocks.
    pub factor_returns: Vec<(String, f64)>,
    /// The effect by each classification scheme the options group by.
    pub groups: Vec<SchemeImpact>,
    /// The risk before, with a risk model.
    pub risk_before: Option<ScenarioRisk>,
    /// The risk after, with the stressed covariance if the risk model has a stress; `None`
    /// without a risk model or if the portfolio is worth nothing after.
    pub risk_after: Option<ScenarioRisk>,
    /// For a historical scenario, the holdings' value over the period.
    pub path: Option<ScenarioPath>,
    /// Prices and rates carried forward from earlier dates, and shocks that matched nothing.
    pub warnings: ValidationReport,
}

/// A holding valued before a scenario.
struct Held {
    key: HoldingKey,
    currency: Currency,
    value: f64,
}

/// How a scenario moves one holding.
struct Move {
    local: f64,
    currency: f64,
    source: ShockSource,
}

/// The effect of `shocks` on the holdings valued at the end of `as_of`.
///
/// Every problem with the shocks is reported at once: unknown instruments and factors, returns
/// below -100%, a shock to the base currency, and instruments in more than one shocked group.
/// Group shocks that match no instrument held are listed as warnings.
pub fn shock_scenario(
    books: &Books,
    market: &MarketData,
    as_of: Date,
    shocks: &Shocks,
    options: &ScenarioOptions,
) -> Result<ScenarioResult> {
    let valuation = books.value_on(as_of, market)?;
    let held = held(books, &valuation)?;
    let base = books.base_currency();
    let mut report = ValidationReport::new();
    let mut warnings = valuation.warnings.clone();
    check_options(options, &mut report);

    let mut instruments: BTreeMap<&InstrumentId, f64> = BTreeMap::new();
    for (k, (id, shock)) in shocks.instruments.iter().enumerate() {
        let path = format!("shocks.instruments[{k}]");
        if books.instrument(id).is_none() {
            report.error(&path, format!("unknown instrument {id}"));
        }
        check_shock(*shock, &path, &mut report);
        if instruments.insert(id, *shock).is_some() {
            report.error(&path, format!("{id} is shocked twice"));
        }
    }
    let mut groups: BTreeMap<(&str, &str), (usize, f64)> = BTreeMap::new();
    for (k, g) in shocks.groups.iter().enumerate() {
        let path = format!("shocks.groups[{k}]");
        check_shock(g.shock, &format!("{path}.shock"), &mut report);
        if groups
            .insert((g.scheme.as_str(), g.value.as_str()), (k, g.shock))
            .is_some()
        {
            report.error(
                &path,
                format!("{} = {} is shocked twice", g.scheme, g.value),
            );
        }
    }
    let mut currencies: BTreeMap<Currency, f64> = BTreeMap::new();
    for (k, (currency, shock)) in shocks.currencies.iter().enumerate() {
        let path = format!("shocks.currencies[{k}]");
        if *currency == base {
            report.error(
                &path,
                format!("{currency} is the base currency, whose value in itself cannot change"),
            );
        }
        check_shock(*shock, &path, &mut report);
        if currencies.insert(*currency, *shock).is_some() {
            report.error(&path, format!("{currency} is shocked twice"));
        }
    }
    let factor_returns = match &shocks.factors {
        Some(factors) => factor_returns(factors, &mut report),
        None => Vec::new(),
    };
    report.into_result(())?;

    let mut report = ValidationReport::new();
    let mut matched = vec![false; shocks.groups.len()];
    let mut moves = Vec::with_capacity(held.len());
    for h in &held {
        let currency = currencies.get(&h.currency).copied().unwrap_or(0.0);
        let id = match &h.key {
            HoldingKey::Cash(_) => {
                moves.push(Move {
                    local: 0.0,
                    currency,
                    source: ShockSource::Cash,
                });
                continue;
            }
            HoldingKey::Instrument(id) => id,
        };
        let classifications = books
            .instrument(id)
            .map(|i| &i.classifications)
            .ok_or_else(|| invalid("holdings", format!("unknown instrument {id}")))?;
        let matching: Vec<(usize, &GroupShock)> = shocks
            .groups
            .iter()
            .enumerate()
            .filter(|(_, g)| classifications.get(&g.scheme) == Some(&g.value))
            .collect();
        for (k, _) in &matching {
            matched[*k] = true;
        }
        let (local, source) = if let Some(shock) = instruments.get(id) {
            (*shock, ShockSource::Instrument)
        } else if let Some(&(_, g)) = matching.first() {
            if matching.len() > 1 {
                let names: Vec<String> = matching
                    .iter()
                    .map(|(_, g)| format!("{} = {}", g.scheme, g.value))
                    .collect();
                report.error(
                    "shocks.groups",
                    format!(
                        "{id} is in more than one shocked group ({}); give it its own shock",
                        names.join(", ")
                    ),
                );
            }
            (
                g.shock,
                ShockSource::Group {
                    scheme: g.scheme.clone(),
                    value: g.value.clone(),
                },
            )
        } else if let Some(local) = shocks
            .factors
            .as_ref()
            .and_then(|f| implied_return(&f.model, id.as_str(), &factor_returns))
        {
            if !(local.is_finite() && local >= -1.0) {
                report.error(
                    "shocks.factors",
                    format!("the factor shocks imply a return of {local} for {id}, below -100%"),
                );
            }
            (local, ShockSource::Factor)
        } else {
            (0.0, ShockSource::None)
        };
        moves.push(Move {
            local,
            currency,
            source,
        });
    }
    report.into_result(())?;
    for (k, g) in shocks.groups.iter().enumerate() {
        if !matched[k] {
            warnings.warning(
                format!("shocks.groups[{k}]"),
                format!("{} = {} matches no instrument held", g.scheme, g.value),
            );
        }
    }
    let factor_returns = match &shocks.factors {
        Some(f) => f
            .model
            .factors()
            .iter()
            .cloned()
            .zip(factor_returns)
            .collect(),
        None => Vec::new(),
    };
    finish(
        books,
        as_of,
        &held,
        moves,
        factor_returns,
        None,
        options,
        warnings,
    )
}

/// The effect on the holdings valued at the end of `as_of` of the price and exchange-rate
/// changes over a historical period.
///
/// Each instrument returns the change in its price (or its proxy's) from the start of the period
/// to the end, and the change in its currency's exchange rate to the base currency; cash returns
/// the change in its exchange rate. Every missing price or rate is reported at once. The result's
/// path values the holdings on every date in the period with a price for one of them.
pub fn historical_scenario(
    books: &Books,
    market: &MarketData,
    as_of: Date,
    window: &HistoricalWindow,
    options: &ScenarioOptions,
) -> Result<ScenarioResult> {
    let valuation = books.value_on(as_of, market)?;
    let held = held(books, &valuation)?;
    let base = books.base_currency();
    let mut report = ValidationReport::new();
    check_options(options, &mut report);
    if window.start >= window.end {
        report.error("window", "must end after it starts");
    }
    let mut proxies: BTreeMap<&InstrumentId, &InstrumentId> = BTreeMap::new();
    for (k, (id, proxy)) in window.proxies.iter().enumerate() {
        let path = format!("window.proxies[{k}]");
        for x in [id, proxy] {
            if books.instrument(x).is_none() && market.prices(x).is_none() {
                report.error(&path, format!("unknown instrument {x}"));
            }
        }
        if proxies.insert(id, proxy).is_some() {
            report.error(&path, format!("{id} has two proxies"));
        }
    }
    report.into_result(())?;

    // The series behind each holding: its price (or its proxy's), and its currency's rate.
    let series: Vec<(Option<&InstrumentId>, Currency)> = held
        .iter()
        .map(|h| match &h.key {
            HoldingKey::Instrument(id) => (Some(*proxies.get(id).unwrap_or(&id)), h.currency),
            HoldingKey::Cash(c) => (None, *c),
        })
        .collect();
    let mut dates: BTreeSet<Date> = [window.start, window.end].into_iter().collect();
    for id in series.iter().filter_map(|(id, _)| *id) {
        if let Some(prices) = market.prices(id) {
            dates.extend(
                prices
                    .dates()
                    .iter()
                    .filter(|d| **d > window.start && **d < window.end),
            );
        }
    }
    let dates: Vec<Date> = dates.into_iter().collect();

    let mut problems = ValidationReport::new();
    let mut warnings = valuation.warnings.clone();
    // Each holding's level (price times rate) on each date, relative to the start.
    let mut levels: Vec<(Vec<f64>, Vec<f64>)> = Vec::with_capacity(held.len());
    let mut failed: BTreeSet<String> = BTreeSet::new();
    for (id, currency) in &series {
        let mut prices = Vec::with_capacity(dates.len());
        let mut rates = Vec::with_capacity(dates.len());
        for &date in &dates {
            let mut once = |path: String, observation, problems: &mut ValidationReport| {
                if failed.contains(&path) {
                    return 0.0;
                }
                let before = problems.issues().len();
                let value = observe(observation, &path, date, problems, &mut warnings);
                if problems.issues().len() > before {
                    failed.insert(path);
                }
                value
            };
            rates.push(once(
                format!("fx.{currency}{base}"),
                market.fx_rate(*currency, base, date),
                &mut problems,
            ));
            prices.push(match id {
                Some(id) => once(
                    format!("prices.{id}"),
                    market.price(id, date),
                    &mut problems,
                ),
                None => 1.0,
            });
        }
        levels.push((prices, rates));
    }
    for (h, (id, _)) in held.iter().zip(&series) {
        if let (HoldingKey::Instrument(held_id), Some(id)) = (&h.key, id) {
            let path = format!("prices.{id}");
            if failed.contains(&path) && held_id == *id && !proxies.contains_key(held_id) {
                problems.error(
                    &path,
                    format!("{held_id} has no price for the period; name a proxy for it"),
                );
            }
        }
    }
    if problems.has_errors() {
        return Err(Error::Validation(problems));
    }
    for ((prices, rates), (id, currency)) in levels.iter().zip(&series) {
        if prices[0] <= 0.0 || rates[0] <= 0.0 {
            let what = id.map_or_else(|| format!("{currency} cash"), |id| id.to_string());
            problems.error(
                "window.start",
                format!("{what} has a price or rate of zero at the start of the period"),
            );
        }
    }
    if problems.has_errors() {
        return Err(Error::Validation(problems));
    }

    let moves: Vec<Move> = held
        .iter()
        .zip(&levels)
        .map(|(h, (prices, rates))| {
            let last = dates.len() - 1;
            let currency = rates[last] / rates[0] - 1.0;
            match &h.key {
                HoldingKey::Cash(_) => Move {
                    local: 0.0,
                    currency,
                    source: ShockSource::Cash,
                },
                HoldingKey::Instrument(id) => Move {
                    local: prices[last] / prices[0] - 1.0,
                    currency,
                    source: match proxies.get(id) {
                        Some(proxy) => ShockSource::Proxy {
                            instrument: (*proxy).clone(),
                        },
                        None => ShockSource::History,
                    },
                },
            }
        })
        .collect();
    let values: Vec<f64> = (0..dates.len())
        .map(|t| {
            held.iter()
                .zip(&levels)
                .map(|(h, (p, r))| h.value * (p[t] / p[0]) * (r[t] / r[0]))
                .sum()
        })
        .collect();
    let worst_drawdown = if values.iter().all(|v| *v > 0.0) {
        TimeSeries::new(dates.clone(), values.clone())
            .and_then(|levels| drawdowns(&levels))
            .ok()
            .and_then(|all| all.into_iter().min_by(|a, b| a.depth.total_cmp(&b.depth)))
    } else {
        None
    };
    let path = ScenarioPath {
        dates,
        values,
        worst_drawdown,
    };
    finish(
        books,
        as_of,
        &held,
        moves,
        Vec::new(),
        Some(path),
        options,
        warnings,
    )
}

/// `covariance` with every volatility multiplied by `stress.volatility_scale` and every
/// correlation moved toward one by `stress.correlation_shift`:
/// `s^2 ((1 - a) C + a v v')`, with `v` the volatilities. As a mixture of two covariance
/// matrices, the result is positive semidefinite when `covariance` is.
///
/// ```
/// use finalytics_risk::{CorrelationStress, Matrix, stressed_covariance};
///
/// let covariance = Matrix::new(vec!["a".into(), "b".into()], vec![0.04, 0.0, 0.0, 0.09])?;
/// let stress = CorrelationStress { volatility_scale: 1.0, correlation_shift: 1.0 };
/// let stressed = stressed_covariance(&covariance, &stress)?;
/// assert!((stressed.get(0, 1) - 0.2 * 0.3).abs() < 1e-15);
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn stressed_covariance(covariance: &Matrix, stress: &CorrelationStress) -> Result<Matrix> {
    let mut report = ValidationReport::new();
    check_stress(stress, "stress", &mut report);
    for (i, v) in covariance.diagonal().iter().enumerate() {
        if *v < 0.0 {
            report.error(
                format!("values[{}]", i * covariance.dimension() + i),
                "is a negative variance",
            );
        }
    }
    report.into_result(())?;
    Ok(stress_matrix(covariance, stress))
}

fn stress_matrix(covariance: &Matrix, stress: &CorrelationStress) -> Matrix {
    let n = covariance.dimension();
    let s2 = stress.volatility_scale * stress.volatility_scale;
    let a = stress.correlation_shift;
    let sd: Vec<f64> = covariance.diagonal().iter().map(|v| v.sqrt()).collect();
    let mut values = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let c = if i == j {
                covariance.get(i, i)
            } else {
                (1.0 - a) * covariance.get(i, j) + a * sd[i] * sd[j]
            };
            values.push(s2 * c);
        }
    }
    Matrix::from_parts(covariance.labels().to_vec(), values)
}

fn check_shock(shock: f64, path: &str, report: &mut ValidationReport) {
    if !(shock.is_finite() && shock >= -1.0) {
        report.error(path, "must be a finite return of -100% or more");
    }
}

fn check_stress(stress: &CorrelationStress, path: &str, report: &mut ValidationReport) {
    if !(stress.volatility_scale.is_finite() && stress.volatility_scale >= 0.0) {
        report.error(
            format!("{path}.volatility_scale"),
            "must be a finite number of zero or more",
        );
    }
    if !(0.0..=1.0).contains(&stress.correlation_shift) {
        report.error(
            format!("{path}.correlation_shift"),
            "must be between 0 and 1",
        );
    }
}

fn check_options(options: &ScenarioOptions, report: &mut ValidationReport) {
    if let Some(risk) = &options.risk {
        match (&risk.covariance, &risk.factor_model) {
            (Some(_), Some(_)) => report.error(
                "options.risk",
                "give a covariance or a factor model, not both",
            ),
            (None, None) => report.error("options.risk", "needs a covariance or a factor model"),
            _ => {}
        }
        if !(risk.confidence > 0.0 && risk.confidence < 1.0) {
            report.error(
                "options.risk.confidence",
                "must be between 0 and 1, exclusive",
            );
        }
        if let Some(stress) = &risk.stress {
            check_stress(stress, "options.risk.stress", report);
        }
    }
    let mut schemes = BTreeSet::new();
    for (k, scheme) in options.group_by.iter().enumerate() {
        if !schemes.insert(scheme) {
            report.error(
                format!("options.group_by[{k}]"),
                format!("{scheme} is listed twice"),
            );
        }
    }
}

/// Each holding valued in the base currency, added up across accounts, in holding order.
fn held(books: &Books, valuation: &Valuation) -> Result<Vec<Held>> {
    let total = to_f64(valuation.total_base);
    if total.is_nan() || total <= 0.0 {
        return Err(invalid(
            "as_of",
            format!(
                "the portfolio is worth {} on {}; a scenario needs a value above zero",
                valuation.total_base, valuation.date
            ),
        ));
    }
    let mut values: BTreeMap<HoldingKey, (Currency, Decimal)> = BTreeMap::new();
    for p in &valuation.positions {
        let currency = books
            .instrument(&p.instrument)
            .map_or(valuation.base_currency, |i| i.currency);
        values
            .entry(HoldingKey::Instrument(p.instrument.clone()))
            .or_insert((currency, Decimal::ZERO))
            .1 += p.market_value_base;
    }
    for c in &valuation.cash {
        values
            .entry(HoldingKey::Cash(c.currency))
            .or_insert((c.currency, Decimal::ZERO))
            .1 += c.value_base;
    }
    Ok(values
        .into_iter()
        .map(|(key, (currency, value))| Held {
            key,
            currency,
            value: to_f64(value),
        })
        .collect())
}

/// The factor returns, one per factor of the model: the given shocks and, with propagation,
/// the expected returns of the others given them, `F_us F_ss^+ f_s`.
fn factor_returns(factors: &FactorShocks, report: &mut ValidationReport) -> Vec<f64> {
    let names = factors.model.factors();
    let mut given: Vec<Option<f64>> = vec![None; names.len()];
    for (k, (name, shock)) in factors.shocks.iter().enumerate() {
        let path = format!("shocks.factors.shocks[{k}]");
        if !shock.is_finite() {
            report.error(&path, "must be a finite return");
        }
        match names.iter().position(|n| n == name) {
            None => report.error(&path, format!("unknown factor {name}")),
            Some(i) if given[i].is_some() => {
                report.error(&path, format!("{name} is shocked twice"));
            }
            Some(i) => given[i] = Some(*shock),
        }
    }
    let shocked: Vec<usize> = (0..names.len()).filter(|i| given[*i].is_some()).collect();
    let mut returns: Vec<f64> = given.iter().map(|g| g.unwrap_or(0.0)).collect();
    if !factors.propagate || shocked.is_empty() || shocked.len() == names.len() {
        return returns;
    }
    let f = factors.model.factor_covariance();
    let s = shocked.len();
    let block: Vec<f64> = shocked
        .iter()
        .flat_map(|&i| shocked.iter().map(move |&j| f.get(i, j)))
        .collect();
    let Some((values, vectors)) = symmetric_eigen(&block, s) else {
        report.error(
            "shocks.factors",
            "the covariance of the shocked factors could not be decomposed",
        );
        return returns;
    };
    // The pseudo-inverse of the shocked factors' covariance applied to their returns.
    let fs: Vec<f64> = shocked.iter().map(|&i| returns[i]).collect();
    let largest = values.first().copied().unwrap_or(0.0).max(0.0);
    let tolerance = largest * s as f64 * f64::EPSILON * 16.0;
    let mut solved = vec![0.0; s];
    for (lambda, v) in values.iter().zip(&vectors) {
        if *lambda > tolerance {
            let projection: f64 = v.iter().zip(&fs).map(|(a, b)| a * b).sum::<f64>() / lambda;
            for (x, vi) in solved.iter_mut().zip(v) {
                *x += projection * vi;
            }
        }
    }
    for u in (0..names.len()).filter(|i| given[*i].is_none()) {
        returns[u] = shocked
            .iter()
            .zip(&solved)
            .map(|(&i, x)| f.get(u, i) * x)
            .sum();
    }
    returns
}

/// The return the factor returns imply for `asset`, if the model has it.
fn implied_return(model: &FactorModel, asset: &str, returns: &[f64]) -> Option<f64> {
    let k = model.assets().iter().position(|a| a == asset)?;
    Some(
        model.exposures()[k]
            .iter()
            .zip(returns)
            .map(|(b, f)| b * f)
            .sum(),
    )
}

fn label(key: &HoldingKey) -> String {
    match key {
        HoldingKey::Instrument(id) => id.to_string(),
        HoldingKey::Cash(c) => format!("{c} cash"),
    }
}

fn group_of(books: &Books, key: &HoldingKey, scheme: &str) -> String {
    match key {
        HoldingKey::Cash(_) => "cash".to_owned(),
        HoldingKey::Instrument(id) => books
            .instrument(id)
            .and_then(|i| i.classifications.get(scheme))
            .cloned()
            .unwrap_or_else(|| "unclassified".to_owned()),
    }
}

#[allow(clippy::too_many_arguments)]
fn finish(
    books: &Books,
    as_of: Date,
    held: &[Held],
    moves: Vec<Move>,
    factor_returns: Vec<(String, f64)>,
    path: Option<ScenarioPath>,
    options: &ScenarioOptions,
    mut warnings: ValidationReport,
) -> Result<ScenarioResult> {
    let value_before: f64 = held.iter().map(|h| h.value).sum();
    let afters: Vec<f64> = held
        .iter()
        .zip(&moves)
        .map(|(h, m)| h.value * (1.0 + m.local) * (1.0 + m.currency))
        .collect();
    let value_after: f64 = afters.iter().sum();
    let weight_after = |v: f64| (value_after > 0.0).then(|| v / value_after);
    let mut not_shocked = Vec::new();
    let holdings: Vec<HoldingImpact> = held
        .iter()
        .zip(moves)
        .zip(&afters)
        .map(|((h, m), &after)| {
            if let (ShockSource::None, HoldingKey::Instrument(id)) = (&m.source, &h.key) {
                not_shocked.push(id.clone());
            }
            HoldingImpact {
                holding: h.key.clone(),
                value_before: h.value,
                weight_before: h.value / value_before,
                local_return: m.local,
                currency_return: m.currency,
                total_return: (1.0 + m.local) * (1.0 + m.currency) - 1.0,
                value_after: after,
                weight_after: weight_after(after),
                change: after - h.value,
                source: m.source,
            }
        })
        .collect();
    let groups = options
        .group_by
        .iter()
        .map(|scheme| {
            let mut totals: BTreeMap<String, (f64, f64)> = BTreeMap::new();
            for h in &holdings {
                let entry = totals
                    .entry(group_of(books, &h.holding, scheme))
                    .or_default();
                entry.0 += h.value_before;
                entry.1 += h.value_after;
            }
            SchemeImpact {
                scheme: scheme.clone(),
                groups: totals
                    .into_iter()
                    .map(|(group, (before, after))| GroupImpact {
                        group,
                        value_before: before,
                        value_after: after,
                        change: after - before,
                        weight_before: before / value_before,
                        weight_after: weight_after(after),
                        contribution: (after - before) / value_before,
                    })
                    .collect(),
            }
        })
        .collect();
    let (risk_before, risk_after) = match &options.risk {
        None => (None, None),
        Some(model) => {
            let (before, after) =
                risk(books, model, &holdings, options, value_before, value_after)?;
            if after.is_none() {
                warnings.warning(
                    "risk_after",
                    "the portfolio is worth nothing after the scenario, so it has no risk",
                );
            }
            (Some(before), after)
        }
    };
    if !(value_after.is_finite()
        && risk_before
            .iter()
            .chain(&risk_after)
            .all(|r| r.volatility.is_finite() && r.value_at_risk.is_finite()))
    {
        return Err(invalid(
            "shocks",
            "the scenario's values are too large to represent",
        ));
    }
    Ok(ScenarioResult {
        as_of,
        base_currency: books.base_currency(),
        value_before,
        value_after,
        change: value_after - value_before,
        portfolio_return: value_after / value_before - 1.0,
        holdings,
        not_shocked,
        factor_returns,
        groups,
        risk_before,
        risk_after,
        path,
        warnings,
    })
}

/// The risk before and, if the portfolio is worth something after, after.
fn risk(
    books: &Books,
    model: &ScenarioRiskModel,
    holdings: &[HoldingImpact],
    options: &ScenarioOptions,
    value_before: f64,
    value_after: f64,
) -> Result<(ScenarioRisk, Option<ScenarioRisk>)> {
    let full = match (&model.covariance, &model.factor_model) {
        (Some(c), _) => c.clone(),
        (None, Some(m)) => m.covariance(),
        (None, None) => unreachable!("checked with the options"),
    };
    let base = books.base_currency();
    let labels: Vec<String> = holdings.iter().map(|h| label(&h.holding)).collect();
    let mut report = ValidationReport::new();
    let positions: Vec<Option<usize>> = holdings
        .iter()
        .zip(&labels)
        .map(|(h, l)| {
            let found = full.labels().iter().position(|x| x == l);
            if found.is_none() && h.holding != HoldingKey::Cash(base) {
                report.error("options.risk", format!("{l} is not in the risk model"));
            }
            found
        })
        .collect();
    report.into_result(())?;
    let n = holdings.len();
    let mut values = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            if let (Some(a), Some(b)) = (positions[i], positions[j]) {
                values[i * n + j] = full.get(a, b);
            }
        }
    }
    let covariance = Matrix::from_parts(labels, values);
    let z = normal_quantile(model.confidence);
    let snapshot = |covariance: &Matrix, weights: &[f64], value: f64| -> Result<ScenarioRisk> {
        let decomposition = risk_decomposition(weights, covariance)?;
        let groups = options
            .group_by
            .iter()
            .map(|scheme| SchemeRisk {
                scheme: scheme.clone(),
                groups: group_risk(&decomposition, |k, _| {
                    group_of(books, &holdings[k].holding, scheme)
                }),
            })
            .collect();
        Ok(ScenarioRisk {
            volatility: decomposition.volatility,
            value_at_risk: z * decomposition.volatility * value,
            contributions: decomposition.contributions,
            groups,
        })
    };
    let before: Vec<f64> = holdings.iter().map(|h| h.weight_before).collect();
    let risk_before = snapshot(&covariance, &before, value_before)?;
    let risk_after = match holdings
        .iter()
        .map(|h| h.weight_after)
        .collect::<Option<Vec<f64>>>()
    {
        None => None,
        Some(after) => {
            let stressed = match &model.stress {
                Some(stress) => stress_matrix(&covariance, stress),
                None => covariance,
            };
            Some(snapshot(&stressed, &after, value_after)?)
        }
    };
    Ok((risk_before, risk_after))
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}
