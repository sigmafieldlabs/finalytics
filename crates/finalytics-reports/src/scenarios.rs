//! The scenarios report: named shock and historical scenarios run together on the same
//! holdings and risk model, side by side.

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Error, Issue, Result, ValidationReport};
use finalytics_ledger::{Books, HoldingKey, MarketData};
use finalytics_risk::{
    CorrelationStress, HistoricalWindow, ScenarioOptions, ScenarioResult, ScenarioRiskModel,
    Shocks, historical_scenario, shock_scenario,
};

use crate::context::{Context, Provenance, ReportKind, ReportOptions, Source, label, to_f64};
use crate::risk::holdings_risk;
use crate::shapes::{Grid, Unit};

/// What a scenario does: shocks, or a historical period replayed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioDefinition {
    /// Shocks to instruments, groups, currencies or factors.
    Shocks {
        /// The shocks.
        shocks: Shocks,
    },
    /// A historical period.
    Historical {
        /// The period, with any proxies.
        window: HistoricalWindow,
    },
}

/// A scenario with a name, such as "Equities -20%" or "2008".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NamedScenario {
    /// The name.
    pub name: String,
    /// What it does.
    pub scenario: ScenarioDefinition,
    /// A stress of the covariance for the risk after this scenario, as in a crisis.
    #[serde(default)]
    pub stress: Option<CorrelationStress>,
}

/// One scenario's result, or why it could not be run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioOutcome {
    /// The scenario's name.
    pub name: String,
    /// The result; `None` when the scenario could not be run.
    pub result: Option<ScenarioResult>,
    /// The problems that stopped it, with paths into the scenario.
    pub problems: Vec<Issue>,
}

/// One scenario in a line, for a summary table or a bar chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioSummary {
    /// The scenario's name.
    pub name: String,
    /// The change in value, in the base currency.
    pub change: f64,
    /// The portfolio's return.
    pub portfolio_return: f64,
    /// The annualised volatility after.
    pub volatility_after: Option<f64>,
    /// The value at risk after, at the first confidence of the options.
    pub value_at_risk_after: Option<f64>,
    /// The holding that lost most.
    pub worst: Option<HoldingKey>,
    /// Its change in value.
    pub worst_change: Option<f64>,
}

/// The scenarios report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenariosReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The portfolio's value before every scenario.
    pub value: f64,
    /// The first date of the returns behind the risk model, when there is one.
    pub risk_start: Option<Date>,
    /// Each scenario, in the order given.
    pub scenarios: Vec<ScenarioOutcome>,
    /// The scenarios that ran, in a line each.
    pub summary: Vec<ScenarioSummary>,
    /// The change in each holding's value (rows) in each scenario that ran (columns).
    pub changes: Grid,
    /// Each holding's return in the base currency in each scenario that ran.
    pub returns: Grid,
}

/// The scenarios of the options, run on the holdings on the report's date with the risk model
/// the risk report estimates (the holdings' covariance, annualised), so that each has its risk
/// before and after.
pub fn scenarios_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<ScenariosReport> {
    let cx = Context::new(books, market, source, options)?;
    let valuation = books.value_on(cx.as_of, market)?;
    let mut warnings = valuation.warnings.clone();
    let (risk_model, risk_start) = match holdings_risk(&cx, &valuation) {
        Ok(e) => (
            Some(ScenarioRiskModel {
                covariance: Some(e.risk.covariance),
                factor_model: None,
                confidence: options.confidences.first().copied().unwrap_or(0.95),
                stress: None,
            }),
            Some(e.risk.start),
        ),
        Err(e) => {
            warnings.warning(
                "risk",
                format!("the holdings' risk could not be estimated, so scenarios have none: {e}"),
            );
            (None, None)
        }
    };
    let mut outcomes = Vec::with_capacity(options.scenarios.len());
    for named in &options.scenarios {
        let scenario_options = ScenarioOptions {
            group_by: options.group_by.clone(),
            risk: risk_model.clone().map(|mut m| {
                m.stress = named.stress;
                m
            }),
        };
        let result = match &named.scenario {
            ScenarioDefinition::Shocks { shocks } => {
                shock_scenario(books, market, cx.as_of, shocks, &scenario_options)
            }
            ScenarioDefinition::Historical { window } => {
                historical_scenario(books, market, cx.as_of, window, &scenario_options)
            }
        };
        outcomes.push(match result {
            Ok(r) => ScenarioOutcome {
                name: named.name.clone(),
                result: Some(r),
                problems: Vec::new(),
            },
            Err(Error::Validation(report)) => ScenarioOutcome {
                name: named.name.clone(),
                result: None,
                problems: report.issues().to_vec(),
            },
            Err(e) => {
                let mut report = ValidationReport::new();
                report.error("", e.to_string());
                ScenarioOutcome {
                    name: named.name.clone(),
                    result: None,
                    problems: report.issues().to_vec(),
                }
            }
        });
    }

    let ran: Vec<(&str, &ScenarioResult)> = outcomes
        .iter()
        .filter_map(|o| o.result.as_ref().map(|r| (o.name.as_str(), r)))
        .collect();
    let summary = ran
        .iter()
        .map(|(name, r)| {
            let worst = r
                .holdings
                .iter()
                .min_by(|a, b| a.change.total_cmp(&b.change))
                .filter(|h| h.change < 0.0);
            ScenarioSummary {
                name: (*name).to_owned(),
                change: r.change,
                portfolio_return: r.portfolio_return,
                volatility_after: r.risk_after.as_ref().map(|x| x.volatility),
                value_at_risk_after: r.risk_after.as_ref().map(|x| x.value_at_risk),
                worst: worst.map(|h| h.holding.clone()),
                worst_change: worst.map(|h| h.change),
            }
        })
        .collect();
    let holdings: Vec<HoldingKey> = ran
        .first()
        .map(|(_, r)| r.holdings.iter().map(|h| h.holding.clone()).collect())
        .unwrap_or_default();
    let grid = |name: &str, unit: Unit, pick: fn(&finalytics_risk::HoldingImpact) -> f64| Grid {
        name: name.to_owned(),
        unit,
        rows: holdings.iter().map(label).collect(),
        columns: ran.iter().map(|(n, _)| (*n).to_owned()).collect(),
        values: holdings
            .iter()
            .map(|key| {
                ran.iter()
                    .map(|(_, r)| r.holdings.iter().find(|h| &h.holding == key).map(pick))
                    .collect()
            })
            .collect(),
    };
    Ok(ScenariosReport {
        provenance: cx.provenance(ReportKind::Scenarios, warnings),
        value: to_f64(valuation.total_base),
        risk_start,
        changes: grid("Change in value", Unit::Money { currency: cx.base }, |h| {
            h.change
        }),
        returns: grid("Return", Unit::Fraction, |h| h.total_return),
        scenarios: outcomes,
        summary,
    })
}
