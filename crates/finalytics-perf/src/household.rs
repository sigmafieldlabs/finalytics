//! A household: several accounts in one view, by account and by group of accounts, with the
//! consolidated performance and each account's contribution to it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Decimal, Error, Result, ValidationReport};
use finalytics_ledger::{AccountId, Books, MarketData};

use crate::{FlowTiming, Performance};

/// Options for [`household`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct HouseholdOptions {
    /// Classification schemes of the accounts to group them by, such as `owner` or `wrapper`.
    /// Accounts without the classification are in the group "unclassified".
    pub group_by: Vec<String>,
    /// When external flows happen within the day: at the start (the default) or the end.
    pub timing: FlowTiming,
}

/// The performance of one account, a group of accounts or the whole household over a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountsPerformance {
    /// The account's identifier, the group's name, or "household".
    pub name: String,
    /// The accounts it includes.
    pub accounts: Vec<AccountId>,
    /// The value at the start of the period, in the base currency.
    pub value_start: f64,
    /// The value at the end of the period.
    pub value_end: f64,
    /// The external flows over the period, net: positive when more came in than went out.
    pub flows: f64,
    /// `value_end - value_start - flows`.
    pub gain: f64,
    /// The share of the household's value at the start, `None` if the household started with
    /// nothing.
    pub weight_start: Option<f64>,
    /// The time-weighted return over the period, `None` when it cannot be measured, such as for
    /// an account that gained or lost value on a day it started with none.
    pub time_weighted: Option<f64>,
    /// The money-weighted return (annualised internal rate of return), `None` when it cannot be
    /// found.
    pub money_weighted: Option<f64>,
    /// The contribution to the household's time-weighted return: each day's gain over the
    /// household's starting value for the day, linked by the household's growth. The
    /// contributions of all accounts add up to the household's return.
    pub contribution: f64,
}

/// Performance by the groups of one classification scheme of the accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemePerformance {
    /// The classification scheme.
    pub scheme: String,
    /// Each group, in name order.
    pub groups: Vec<AccountsPerformance>,
}

/// A household's performance over a period: consolidated, by account and by group of accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Household {
    /// The start of the period: the first date.
    pub start: Date,
    /// The end of the period: the last date.
    pub end: Date,
    /// Every account together.
    pub household: AccountsPerformance,
    /// Each account, in the portfolio's order.
    pub accounts: Vec<AccountsPerformance>,
    /// Each scheme the options group by.
    pub groups: Vec<SchemePerformance>,
    /// Prices and rates carried forward, and returns that could not be measured.
    pub warnings: ValidationReport,
}

fn f(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}

/// The household's performance over `dates` (strictly increasing, at least two): every account
/// together, each account, and each group of accounts by the schemes in `options`.
///
/// Each account's and group's own returns are measured on its own values and flows, so a
/// transfer from one account to another is a flow for each. The consolidated figures are
/// measured on the household's values and flows, in which a transfer between two of its
/// accounts nets to zero. Contributions are measured on the household's base, so that the
/// accounts' contributions, and the groups' within each scheme, add up exactly to the
/// household's time-weighted return.
pub fn household(
    books: &Books,
    market: &MarketData,
    dates: &[Date],
    options: &HouseholdOptions,
) -> Result<Household> {
    let mut report = ValidationReport::new();
    if dates.len() < 2 {
        report.error("dates", "needs at least two dates, for a period");
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
    report.into_result(())?;
    let series = books.account_series(dates, market)?;
    let mut warnings = series.warnings.clone();
    let consolidated = Performance::from_values(&series.combined(|_| true), options.timing)?;
    let n = dates.len();
    let (first, last) = (dates[0], dates[n - 1]);
    let household_start = consolidated.values[0];
    let base = |t: usize| match consolidated.timing {
        FlowTiming::StartOfDay => consolidated.values[t - 1] + consolidated.flows[t],
        FlowTiming::EndOfDay => consolidated.values[t - 1],
    };

    // The performance of `accounts` together, with warnings under `path`.
    let measure = |name: String,
                   accounts: Vec<AccountId>,
                   path: String,
                   warnings: &mut ValidationReport|
     -> AccountsPerformance {
        let combined = series.combined(|id| accounts.contains(id));
        let values: Vec<f64> = combined.values.iter().copied().map(f).collect();
        let flows: Vec<f64> = combined.flows.iter().copied().map(f).collect();
        let mut contribution = 0.0;
        for t in 1..n {
            let b = base(t);
            if b > 0.0 {
                let gain = values[t] - values[t - 1] - flows[t];
                contribution += gain / b * consolidated.growth[t - 1];
            }
        }
        let own = match Performance::from_values(&combined, options.timing) {
            Ok(p) => Some(p),
            Err(Error::Validation(issues)) => {
                let reason = issues
                    .issues()
                    .first()
                    .map_or_else(String::new, |i| i.message.clone());
                warnings.warning(
                    path,
                    format!("{name} has no time-weighted return over the period: {reason}"),
                );
                None
            }
            Err(e) => {
                warnings.warning(path, format!("{name} has no time-weighted return: {e}"));
                None
            }
        };
        let flow_total: f64 = flows[1..].iter().sum();
        AccountsPerformance {
            name,
            accounts,
            value_start: values[0],
            value_end: values[n - 1],
            flows: flow_total,
            gain: values[n - 1] - values[0] - flow_total,
            weight_start: (household_start > 0.0).then(|| values[0] / household_start),
            time_weighted: own.as_ref().and_then(|p| p.time_weighted(first, last)),
            money_weighted: own.as_ref().and_then(|p| p.money_weighted(first, last)),
            contribution,
        }
    };

    let all: Vec<AccountId> = books.accounts().iter().map(|a| a.id.clone()).collect();
    let mut total = measure("household".into(), all, "household".into(), &mut warnings);
    // The consolidated return is the household's own; its contribution is that return.
    total.time_weighted = consolidated.time_weighted(first, last);
    total.money_weighted = consolidated.money_weighted(first, last);
    let accounts: Vec<AccountsPerformance> = books
        .accounts()
        .iter()
        .enumerate()
        .map(|(k, a)| {
            measure(
                a.id.to_string(),
                vec![a.id.clone()],
                format!("accounts[{k}]"),
                &mut warnings,
            )
        })
        .collect();
    let groups = options
        .group_by
        .iter()
        .enumerate()
        .map(|(s, scheme)| {
            let mut members: BTreeMap<String, Vec<AccountId>> = BTreeMap::new();
            for a in books.accounts() {
                let group = a
                    .classifications
                    .get(scheme)
                    .cloned()
                    .unwrap_or_else(|| "unclassified".to_owned());
                members.entry(group).or_default().push(a.id.clone());
            }
            SchemePerformance {
                scheme: scheme.clone(),
                groups: members
                    .into_iter()
                    .enumerate()
                    .map(|(g, (group, accounts))| {
                        measure(
                            group,
                            accounts,
                            format!("groups[{s}].groups[{g}]"),
                            &mut warnings,
                        )
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(Household {
        start: first,
        end: last,
        household: total,
        accounts,
        groups,
        warnings,
    })
}
