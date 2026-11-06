//! Goal planning: the chance that a plan of saving, spending and investing meets one goal, an
//! amount by a date or payments over a period, from thousands of simulated markets drawn from
//! the assets' monthly returns.
//!
//! Every amount is in today's money. Each month's return is after inflation and fees, so an
//! amount stays what it would buy today; a flow can instead grow at its own rate in money of the
//! day. Each plan reports what would change its outcome, how it fares when markets do worse, and
//! how other investment mixes compare on the same simulations.
//!
//! ```
//! use finalytics::prelude::*;
//! use finalytics::planning::Frequency;
//!
//! let assets = Assets::builder()
//!     .prices(datasets::asset_class_prices())
//!     .build()?;
//! let plan = assets
//!     .plan()
//!     .target(150_000.0, "2030-06")
//!     .name("House deposit")
//!     .start_value(60_000.0)
//!     .saving(2_350.0, Frequency::Month)
//!     .weights([("US Equity", 0.6), ("Bonds", 0.4)])
//!     .inflation(0.025)
//!     .simulations(1_000)
//!     .build()?;
//! let chance = plan.results.outcome.chance.unwrap();
//! assert!(chance > 0.0 && chance < 1.0);
//! # Ok::<(), finalytics::Error>(())
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Error, Result, ValidationReport};
use finalytics_risk::{PriceTable, ReturnsTable, covariance};

pub use finalytics_plan::{
    AmountChange, Changes, CrashWindow, Flow, FlowSummary, Frequency, Goal, History, Inflation,
    MAX_MONTHS, MIN_HISTORY, Mix, MixOutcome, MixRisk, Model, Month, MonthChange, Outcome,
    PaymentsOutcome, Percentiles, PlanOptions, PlanResults, SamplePath, Status, WhatIf, simulate,
};
pub use finalytics_reports::{InflationBasis, PlanContext, PlanReport};

use crate::Assets;

/// A plan: the planning report and the raw results of its simulations. Printing it prints the
/// report, and showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Plan {
    /// The planning report.
    pub report: PlanReport,
    /// The raw results: the chance, the value over time, the changes, the tests when markets do
    /// worse and the alternatives.
    pub results: PlanResults,
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl Assets {
    /// A plan on these assets, built step by step: the goal, the money paid in and out, the
    /// investment mix and the inflation; [`PlanBuilder::build`] runs it.
    pub fn plan(&self) -> PlanBuilder<'_> {
        PlanBuilder {
            assets: self,
            options: PlanOptions::default(),
            problems: ValidationReport::new(),
            index: None,
        }
    }

    /// The plan of `options` simulated on these assets' monthly returns: the planning report,
    /// with the raw results. [`Assets::plan`] builds the same options step by step.
    ///
    /// The assets' returns are compounded into calendar months. With a price index as the
    /// inflation, the history is the months both cover. The plan starts, unless it says
    /// otherwise, in the month after the assets' last.
    pub fn plan_with(&self, options: &PlanOptions) -> Result<Plan> {
        self.plan_named(options, None)
    }

    fn plan_named(&self, options: &PlanOptions, index_name: Option<&str>) -> Result<Plan> {
        let (first, returns) = self.monthly()?;
        let mut options = options.clone();
        if options.start.is_none() {
            // The month after the last return, whole or not.
            options.start = self.dates().last().map(|d| Month::of(*d).plus(1));
        }
        let mut history = History {
            first,
            assets: self.labels().to_vec(),
            returns,
            inflation: None,
            estimates: None,
        };
        let mut index = None;
        if let Some(Inflation::Index { prices }) = &options.inflation {
            let name = index_name
                .map(str::to_owned)
                .or_else(|| prices.labels().first().cloned())
                .unwrap_or_else(|| "price index".to_owned());
            history = with_index(history, prices)?;
            index = Some(name);
        }
        if matches!(options.model, Model::Normal) {
            history.estimates = Some(self.monthly_estimates(&history)?);
        }
        let results = simulate(&history, &options)?;
        let report = finalytics_reports::plan_report(
            results.clone(),
            finalytics_reports::PlanInputs {
                version: crate::VERSION.to_owned(),
                options: &options,
                history: &history,
                index,
            },
        );
        Ok(Plan { report, results })
    }

    /// The assets' returns compounded into calendar months, from the first whole month, and
    /// that month.
    fn monthly(&self) -> Result<(Month, Vec<Vec<f64>>)> {
        let invalid = |message: String| {
            let mut report = ValidationReport::new();
            report.error("assets", message);
            Error::Validation(report)
        };
        let dates = self.dates();
        if self.periods_per_year() < 11.5 {
            return Err(invalid(format!(
                "have {} returns a year: a plan needs monthly returns or more frequent ones",
                self.periods_per_year()
            )));
        }
        let columns = self.returns().columns();
        // The calendar month of each return, and whether there is more than one in a month.
        let months: Vec<Month> = dates.iter().map(|d| Month::of(*d)).collect();
        let several = self.periods_per_year() > 13.0;
        let mut groups: Vec<(Month, std::ops::Range<usize>)> = Vec::new();
        for (i, m) in months.iter().enumerate() {
            match groups.last_mut() {
                Some((last, range)) if last == m => range.end = i + 1,
                _ => groups.push((*m, i..i + 1)),
            }
        }
        if several {
            // A first month whose returns start after its start, or a last month whose returns
            // end more than a week before its end, is only part of a month. Without the date of
            // the prices before the first return, a first month whose returns start more than a
            // week into it is part of a month.
            let partial_first = match (groups.first(), self.origin()) {
                (Some((m, _)), Some(origin)) => Month::of(origin) == *m,
                (Some((m, _)), None) => m
                    .first_day()
                    .add_days(7)
                    .is_ok_and(|week_in| dates[0] > week_in),
                _ => false,
            };
            if partial_first {
                groups.remove(0);
            }
            if let Some((m, range)) = groups.last()
                && m.last_day()
                    .add_days(-7)
                    .is_ok_and(|week_before| dates[range.end - 1] < week_before)
            {
                groups.pop();
            }
        }
        for pair in groups.windows(2) {
            if pair[0].0.plus(1) != pair[1].0 {
                return Err(invalid(format!(
                    "have no return in {}: a plan needs every month",
                    pair[0].0.plus(1)
                )));
            }
        }
        let first = groups
            .first()
            .map(|g| g.0)
            .ok_or_else(|| invalid("have no whole month of returns".to_owned()))?;
        let returns = columns
            .iter()
            .map(|c| {
                groups
                    .iter()
                    .map(|(_, range)| {
                        c[range.clone()].iter().map(|r| 1.0 + r).product::<f64>() - 1.0
                    })
                    .collect()
            })
            .collect();
        Ok((first, returns))
    }

    /// The monthly mean returns and covariance of `history`, by the assets' estimators, for the
    /// normal model.
    fn monthly_estimates(&self, history: &History) -> Result<(Vec<f64>, Vec<f64>)> {
        let table = ReturnsTable::new(history.assets.clone(), history.returns.clone())?;
        let means = match self.mean_method() {
            finalytics_risk::MeanMethod::Historical => history
                .returns
                .iter()
                .map(|c| c.iter().sum::<f64>() / c.len() as f64)
                .collect(),
            method => finalytics_risk::mean_returns(&table, method)?.means,
        };
        let cov = covariance(&table, self.covariance_method())?;
        Ok((means, cov.matrix.values().to_vec()))
    }
}

/// `history` with each month's inflation from the price index `prices`, cut to the months the
/// index covers: the last level in each month, with a month missing from the index filled in
/// between its neighbours at a steady rate.
fn with_index(mut history: History, prices: &PriceTable) -> Result<History> {
    let invalid = |message: String| {
        let mut report = ValidationReport::new();
        report.error("inflation.prices", message);
        Error::Validation(report)
    };
    if prices.labels().len() != 1 {
        return Err(invalid(format!(
            "have {} columns: give a price index of one column",
            prices.labels().len()
        )));
    }
    let mut levels: BTreeMap<Month, f64> = BTreeMap::new();
    for (d, v) in prices.dates().iter().zip(&prices.columns()[0]) {
        if let Some(v) = v {
            if !(v.is_finite() && *v > 0.0) {
                return Err(invalid(format!(
                    "have a level of {v} on {d}: levels must be above zero"
                )));
            }
            levels.insert(Month::of(*d), *v);
        }
    }
    let (Some((&lo, _)), Some((&hi, _))) = (levels.first_key_value(), levels.last_key_value())
    else {
        return Err(invalid("have no levels".to_owned()));
    };
    // The level at the end of each month from the first to the last, filling the gaps.
    let level = |m: Month| -> Option<f64> {
        if m < lo || m > hi {
            return None;
        }
        if let Some(v) = levels.get(&m) {
            return Some(*v);
        }
        let (&a, &va) = levels.range(..m).next_back()?;
        let (&b, &vb) = levels.range(m..).next()?;
        let w = f64::from(a.months_until(m)) / f64::from(a.months_until(b));
        Some((va.ln() + w * (vb.ln() - va.ln())).exp())
    };
    let n = history.returns.first().map_or(0, Vec::len) as i32;
    let months: Vec<i32> = (0..n)
        .filter(|&t| {
            let m = history.first.plus(t);
            level(m.plus(-1)).is_some() && level(m).is_some()
        })
        .collect();
    let (Some(&a), Some(&b)) = (months.first(), months.last()) else {
        return Err(invalid(format!(
            "run from {lo} to {hi}; the assets' returns run from {} to {}: give an index that \
             covers them",
            history.first,
            history.first.plus(n - 1)
        )));
    };
    let inflation: Vec<f64> = (a..=b)
        .map(|t| {
            let m = history.first.plus(t);
            level(m).unwrap_or(1.0) / level(m.plus(-1)).unwrap_or(1.0) - 1.0
        })
        .collect();
    history.returns = history
        .returns
        .into_iter()
        .map(|c| c[a as usize..=b as usize].to_vec())
        .collect();
    history.first = history.first.plus(a);
    history.inflation = Some(inflation);
    Ok(history)
}

/// A plan on the assets, built step by step; [`PlanBuilder::build`] runs it. Months are given
/// as text, such as `"2030-06"`; a month that does not read as one is reported with the plan's
/// other problems when it is built.
#[derive(Debug, Clone)]
#[must_use]
pub struct PlanBuilder<'a> {
    assets: &'a Assets,
    options: PlanOptions,
    problems: ValidationReport,
    index: Option<String>,
}

impl PlanBuilder<'_> {
    fn month(&mut self, path: &str, text: &str) -> Option<Month> {
        match text.parse() {
            Ok(m) => Some(m),
            Err(e) => {
                let message = match e {
                    Error::InvalidDate(m) => m,
                    other => other.to_string(),
                };
                self.problems.error(path, message);
                None
            }
        }
    }

    /// The goal: `amount` by the start of the month `by`.
    pub fn target(mut self, amount: f64, by: &str) -> Self {
        if let Some(by) = self.month("goal.by", by) {
            self.options.goal = Some(Goal::Target {
                name: self.options.name.clone(),
                amount,
                by,
            });
        }
        self
    }

    /// The goal: `amount` every month from `start` to `end`, each paid in full.
    pub fn payments(self, amount: f64, start: &str, end: &str) -> Self {
        self.payments_every(amount, Frequency::Month, start, end)
    }

    /// The goal: `amount` every month, quarter or year from `start` to `end`.
    pub fn payments_every(mut self, amount: f64, every: Frequency, start: &str, end: &str) -> Self {
        let first = self.month("goal.start", start);
        let last = self.month("goal.end", end);
        if let (Some(start), Some(end)) = (first, last) {
            self.options.goal = Some(Goal::Payments {
                name: self.options.name.clone(),
                amount,
                every,
                start,
                end,
            });
        }
        self
    }

    /// No goal: a projection of the value to the month `end`.
    pub fn projection(mut self, end: &str) -> Self {
        self.options.goal = None;
        self.options.end = self.month("end", end);
        self
    }

    /// The plan's name, the report's title, such as "Retirement income"; the goal's name too.
    pub fn name(mut self, name: &str) -> Self {
        self.options.name = Some(name.to_owned());
        if let Some(Goal::Target { name: n, .. } | Goal::Payments { name: n, .. }) =
            &mut self.options.goal
        {
            *n = Some(name.to_owned());
        }
        self
    }

    /// The chance wanted of meeting the goal: 0.8 unless given.
    pub fn chance(mut self, chance: f64) -> Self {
        self.options.chance = chance;
        self
    }

    /// The chance from which the plan has room to spare: 10 points above the chance wanted
    /// unless given.
    pub fn spare(mut self, spare: f64) -> Self {
        self.options.spare = Some(spare);
        self
    }

    /// The plan's first month: the month after the assets' last unless given.
    pub fn start(mut self, start: &str) -> Self {
        self.options.start = self.month("start", start);
        self
    }

    /// The value at the start.
    pub fn start_value(mut self, value: f64) -> Self {
        self.options.start_value = value;
        self
    }

    /// A regular saving of `amount` every month, quarter or year from the start until the goal.
    pub fn saving(self, amount: f64, every: Frequency) -> Self {
        self.flow(Flow::Saving {
            name: None,
            amount,
            on: None,
            every: Some(every),
            start: None,
            end: None,
            growth: None,
        })
    }

    /// A single payment out of `amount` in the month `on`, such as a car.
    pub fn spending_on(mut self, name: &str, amount: f64, on: &str) -> Self {
        let path = format!("flows[{}].on", self.options.flows.len());
        let on = self.month(&path, on);
        self.flow(Flow::Spending {
            name: Some(name.to_owned()),
            amount,
            on,
            every: None,
            start: None,
            end: None,
            growth: None,
        })
    }

    /// Money paid in or out: a [`Flow`].
    pub fn flow(mut self, flow: Flow) -> Self {
        self.options.flows.push(flow);
        self
    }

    /// The investment mix: fixed weights by asset, rebalanced every month.
    pub fn weights<S: Into<String>>(mut self, weights: impl IntoIterator<Item = (S, f64)>) -> Self {
        let name = self
            .options
            .weights
            .as_ref()
            .and_then(Mix::name)
            .map(str::to_owned);
        let mut mix = Mix::fixed(weights);
        if let Some(n) = name {
            mix = mix.named(&n);
        }
        self.options.weights = Some(mix);
        self
    }

    /// The investment mix: fixed weights or a glide path, a [`Mix`].
    pub fn mix(mut self, mix: Mix) -> Self {
        self.options.weights = Some(mix);
        self
    }

    /// The name of the investment mix, such as "Balanced 60/40".
    pub fn mix_name(mut self, name: &str) -> Self {
        let mix = self
            .options
            .weights
            .take()
            .unwrap_or_else(|| Mix::fixed(Vec::<(String, f64)>::new()));
        self.options.weights = Some(mix.named(name));
        self
    }

    /// Another mix to compare with the plan's on the same simulations, by its `name`.
    pub fn alternative<S: Into<String>>(
        mut self,
        name: &str,
        weights: impl IntoIterator<Item = (S, f64)>,
    ) -> Self {
        self.options
            .alternatives
            .push(Mix::fixed(weights).named(name));
        self
    }

    /// The fee a year, taken monthly, such as 0.0025.
    pub fn fee(mut self, fee: f64) -> Self {
        self.options.fee = fee;
        self
    }

    /// The inflation a year, such as 0.025: every amount stays in today's money.
    pub fn inflation(mut self, rate: f64) -> Self {
        self.options.inflation = Some(Inflation::Rate { rate });
        self
    }

    /// A price index of one column, such as `datasets::us_cpi()`, as the inflation: each month's
    /// inflation is drawn with the same month's returns.
    pub fn inflation_index(mut self, prices: PriceTable) -> Self {
        self.index = prices.labels().first().cloned();
        self.options.inflation = Some(Inflation::Index { prices });
        self
    }

    /// How markets are simulated: the stationary block bootstrap unless given.
    pub fn model(mut self, model: Model) -> Self {
        self.options.model = model;
        self
    }

    /// An asset's expected return a year, in place of its history's average.
    pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
        self.options
            .expected_returns
            .get_or_insert_with(BTreeMap::new)
            .insert(asset.to_owned(), value);
        self
    }

    /// The number of simulations: 10,000 unless given.
    pub fn simulations(mut self, n: usize) -> Self {
        self.options.simulations = n;
        self
    }

    /// The seed of the random draws: 0 unless given.
    pub fn seed(mut self, seed: u64) -> Self {
        self.options.seed = seed;
        self
    }

    /// The currency of the amounts, such as "USD", for the report.
    pub fn currency(mut self, code: &str) -> Self {
        match Currency::new(code) {
            Ok(c) => self.options.currency = Some(c),
            Err(e) => self.problems.error("currency", e.to_string()),
        }
        self
    }

    /// The options as built, with the problems found so far.
    pub fn options(&self) -> Result<PlanOptions> {
        self.problems.clone().into_result(self.options.clone())
    }

    /// Runs the plan.
    pub fn build(&self) -> Result<Plan> {
        let options = self.options()?;
        self.assets.plan_named(&options, self.index.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets;

    fn assets() -> Assets {
        Assets::builder()
            .prices(datasets::asset_class_prices())
            .build()
            .unwrap()
    }

    #[test]
    fn monthly_returns_cover_the_assets_months() {
        let a = assets();
        let (first, returns) = a.monthly().unwrap();
        assert_eq!(first.to_string(), "1996-11");
        assert_eq!(returns.len(), 6);
        assert_eq!(returns[0].len(), a.dates().len());
        assert!((returns[0][0] - a.returns().columns()[0][0]).abs() < 1e-15);
    }

    #[test]
    fn a_price_index_is_matched_to_the_months() {
        let a = assets();
        let (first, returns) = a.monthly().unwrap();
        let history = History {
            first,
            assets: a.labels().to_vec(),
            returns,
            inflation: None,
            estimates: None,
        };
        let h = with_index(history, &datasets::us_cpi()).unwrap();
        let inflation = h.inflation.as_ref().unwrap();
        // The index ends in September 2026, a month before the assets.
        assert_eq!(h.first.to_string(), "1996-11");
        assert_eq!(inflation.len(), h.returns[0].len());
        assert_eq!(
            h.first.plus(inflation.len() as i32 - 1).to_string(),
            "2026-09"
        );
        let average = inflation
            .iter()
            .map(|r| 1.0 + r)
            .product::<f64>()
            .powf(12.0 / inflation.len() as f64)
            - 1.0;
        assert!(average > 0.015 && average < 0.04, "{average}");
    }

    #[test]
    fn problems_are_reported_together() {
        let a = assets();
        let err = a
            .plan()
            .target(100_000.0, "2030-13")
            .spending_on("Car", 20_000.0, "soon")
            .currency("dollars")
            .build()
            .unwrap_err();
        let Error::Validation(report) = err else {
            panic!("not a validation error")
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["goal.by", "flows[0].on", "currency"]);
    }

    #[test]
    fn a_plan_runs_with_an_index_and_starts_after_the_assets() {
        let a = assets();
        let plan = a
            .plan()
            .payments(4_000.0, "2045-01", "2054-12")
            .name("Retirement income")
            .start_value(250_000.0)
            .saving(2_000.0, Frequency::Month)
            .weights([("US Equity", 0.6), ("Bonds", 0.4)])
            .inflation_index(datasets::us_cpi())
            .simulations(500)
            .build()
            .unwrap();
        assert_eq!(plan.results.start.to_string(), "2026-11");
        assert_eq!(
            plan.report.context.inflation,
            InflationBasis::Index {
                name: "US CPI".to_owned(),
                average: match plan.report.context.inflation {
                    InflationBasis::Index { average, .. } => average,
                    _ => 0.0,
                }
            }
        );
        assert!(plan.to_string().contains("FINALYTICS  ·  PLANNING"));
    }
}
