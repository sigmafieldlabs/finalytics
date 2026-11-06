//! The planning report, on one plan's simulations: the chance of meeting the goal, the value
//! over time, the changes that would reach or keep the chance wanted, the plan when markets do
//! worse, and the investment mixes compared.
//!
//! The report addresses the person the plan is for. Every amount is in today's money, as the
//! simulations are, and every figure is one the engine computed: the page and the text only
//! choose the words.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::Currency;
use finalytics_plan::{
    Frequency, Goal, History, Mix, MixOutcome, MixRisk, Model, Month, Outcome, Percentiles,
    PlanOptions, PlanResults, Status,
};

use crate::backtest_html::lower_name;
use crate::tables::{Cell, Column, ColumnKind, ReportTable};

/// How the plan's amounts are kept in today's money.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InflationBasis {
    /// A fixed inflation rate a year.
    Rate {
        /// The rate, such as 0.025.
        rate: f64,
    },
    /// A price index, month by month in the history.
    Index {
        /// The index's name, such as "US CPI".
        name: String,
        /// Its average rise a year over the history.
        average: f64,
    },
}

/// What a plan simulated, beside what [`PlanResults`] records: what the report needs to
/// describe it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PlanContext {
    /// The engine's version.
    pub version: String,
    /// A SHA-256 digest of the assets' returns and the plan, in hexadecimal: the same inputs
    /// give the same report.
    pub digest: String,
    /// The goal; absent for a projection.
    pub goal: Option<Goal>,
    /// The value at the start, in today's money.
    pub start_value: f64,
    /// The plan's investment mix.
    pub mix: Mix,
    /// The mix's name, such as "Balanced 60/40", or "Your mix" when it has none.
    pub mix_name: String,
    /// The assets the simulations draw from, in the order of the history.
    pub assets: Vec<String>,
    /// The first month of the history of returns.
    pub history_start: Month,
    /// The last month of the history of returns.
    pub history_end: Month,
    /// How markets were simulated.
    pub model: Model,
    /// The expected returns a year given for the assets, if any.
    #[serde(default)]
    pub expected_returns: Option<BTreeMap<String, f64>>,
    /// How amounts are kept in today's money.
    pub inflation: InflationBasis,
    /// The fee a year, such as 0.0025.
    pub fee: f64,
    /// The currency of the amounts, if given.
    pub currency: Option<Currency>,
    /// The seed of the random draws.
    pub seed: u64,
}

/// The planning report: what a plan's simulations show, with the plan it simulated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PlanReport {
    /// What was simulated and how.
    pub context: PlanContext,
    /// What the simulations show.
    pub figures: PlanResults,
    /// The report's title, such as "Retirement income": shown when it is printed or laid out
    /// as a page.
    #[serde(default)]
    pub name: Option<String>,
}

/// What the planning report is built from. The `finalytics` crate assembles it.
#[doc(hidden)]
#[derive(Debug)]
pub struct PlanInputs<'a> {
    pub version: String,
    pub options: &'a PlanOptions,
    /// The monthly history the simulations drew from.
    pub history: &'a History,
    /// The price index's name, when the inflation is an index.
    pub index: Option<String>,
}

/// The planning report on `results`, the simulations of `inputs.options`.
#[doc(hidden)]
pub fn plan_report(results: PlanResults, inputs: PlanInputs<'_>) -> PlanReport {
    let o = inputs.options;
    let h = inputs.history;
    let months = h.returns.first().map_or(0, Vec::len);
    let inflation = match (&inputs.index, &h.inflation, &o.inflation) {
        (Some(name), Some(index), _) => {
            let growth: f64 = index.iter().map(|r| 1.0 + r).product();
            InflationBasis::Index {
                name: name.clone(),
                average: growth.powf(12.0 / months.max(1) as f64) - 1.0,
            }
        }
        (_, _, Some(finalytics_plan::Inflation::Rate { rate })) => {
            InflationBasis::Rate { rate: *rate }
        }
        _ => InflationBasis::Rate { rate: 0.0 },
    };
    let mix = o
        .weights
        .clone()
        .unwrap_or_else(|| Mix::fixed(Vec::<(String, f64)>::new()));
    PlanReport {
        context: PlanContext {
            version: inputs.version,
            digest: digest(h, o),
            goal: o.goal.clone(),
            start_value: o.start_value,
            mix_name: mix.name().unwrap_or("Your mix").to_owned(),
            mix,
            assets: h.assets.clone(),
            history_start: h.first,
            history_end: h.first.plus(months as i32 - 1),
            model: o.model,
            expected_returns: o.expected_returns.clone(),
            inflation,
            fee: o.fee,
            currency: o.currency,
            seed: o.seed,
        },
        figures: results,
        name: o.name.clone(),
    }
}

/// A SHA-256 digest of the history and the plan, in hexadecimal.
fn digest(h: &History, o: &PlanOptions) -> String {
    let mut hash = Sha256::new();
    hash.update(h.first.to_string().as_bytes());
    for (label, column) in h.assets.iter().zip(&h.returns) {
        hash.update(label.as_bytes());
        for r in column {
            hash.update(r.to_le_bytes());
        }
    }
    for r in h.inflation.iter().flatten() {
        hash.update(r.to_le_bytes());
    }
    // The options without the price index, whose months are in the history.
    let mut options = o.clone();
    if matches!(
        options.inflation,
        Some(finalytics_plan::Inflation::Index { .. })
    ) {
        options.inflation = None;
    }
    hash.update(
        serde_json::to_string(&options)
            .unwrap_or_default()
            .as_bytes(),
    );
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ── The goal ─────────────────────────────────────────────────────────────────────────────────

/// The goal, in the terms the report uses.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    Target {
        amount: f64,
        by: Month,
    },
    Payments {
        amount: f64,
        every: Frequency,
        start: Month,
        end: Month,
    },
    Projection,
}

impl PlanReport {
    pub(crate) fn kind(&self) -> Kind {
        match &self.context.goal {
            Some(Goal::Target { amount, by, .. }) => Kind::Target {
                amount: *amount,
                by: *by,
            },
            Some(Goal::Payments {
                amount,
                every,
                start,
                end,
                ..
            }) => Kind::Payments {
                amount: *amount,
                every: *every,
                start: *start,
                end: *end,
            },
            _ => Kind::Projection,
        }
    }

    /// The title: `name`, the report's own, the goal's, or the kind of plan.
    pub(crate) fn title(&self, name: Option<&str>) -> String {
        if let Some(n) = name.or(self.name.as_deref()) {
            return n.to_owned();
        }
        let given = match &self.context.goal {
            Some(Goal::Target { name, .. } | Goal::Payments { name, .. }) => name.clone(),
            _ => None,
        };
        given.unwrap_or_else(|| {
            match self.kind() {
                Kind::Target { .. } => "Savings target",
                Kind::Payments { .. } => "Regular payments",
                Kind::Projection => "Projection",
            }
            .to_owned()
        })
    }

    /// The goal's name inside a sentence, such as "house deposit".
    pub(crate) fn goal_name(&self) -> String {
        let given = match &self.context.goal {
            Some(Goal::Target { name, .. } | Goal::Payments { name, .. }) => name.clone(),
            _ => None,
        };
        match given {
            Some(n) => lower_name(&n),
            None => match self.kind() {
                Kind::Target { .. } => "target".to_owned(),
                _ => "payments".to_owned(),
            },
        }
    }

    pub(crate) fn money(&self) -> Money {
        Money(self.context.currency)
    }

    /// The last month simulated: a target's date is the month after it.
    pub(crate) fn last_month(&self) -> Month {
        match self.kind() {
            Kind::Target { by, .. } => by,
            _ => self.figures.end.plus(-1),
        }
    }

    /// When the value at the end is measured, such as "June 2030".
    pub(crate) fn end_long(&self) -> String {
        self.last_month().long_name()
    }

    pub(crate) fn wanted(&self) -> f64 {
        self.figures.chance_wanted
    }

    /// Whether any flow sets its own growth instead of rising with inflation.
    pub(crate) fn any_growth(&self) -> bool {
        self.figures.flows.iter().any(|f| f.growth.is_some())
    }

    /// What is paid in less what is taken out, in today's money, with no growth; absent when a
    /// flow sets its own growth.
    pub(crate) fn put_in(&self) -> Option<f64> {
        if self.any_growth() {
            return None;
        }
        let flows: f64 = self
            .figures
            .flows
            .iter()
            .map(|f| {
                let total = f.amount * f.count as f64;
                if f.saving { total } else { -total }
            })
            .sum();
        Some(self.context.start_value + flows)
    }

    /// For payments, the month the money would run short with no growth at all, if it does.
    pub(crate) fn runs_out_without_growth(&self) -> Option<Month> {
        let Kind::Payments {
            amount,
            every,
            start,
            end,
        } = self.kind()
        else {
            return None;
        };
        if self.any_growth() {
            return None;
        }
        let first = self.figures.start;
        let due = |m: Month, from: Month, last: Option<Month>, step: usize| {
            let k = from.months_until(m);
            k >= 0 && last.is_some_and(|l| m <= l) && (k as usize).is_multiple_of(step)
        };
        let mut v = self.context.start_value;
        let mut m = first;
        while m <= end {
            // Savings are paid in first, then spending is taken out.
            let savings = self.figures.flows.iter().filter(|f| f.saving);
            let spending = self.figures.flows.iter().filter(|f| !f.saving);
            for f in savings.chain(spending) {
                let step = f.every.map_or(1, Frequency::months);
                if due(m, f.first, f.last, step) {
                    if f.saving {
                        v += f.amount;
                    } else {
                        v -= f.amount.min(v);
                    }
                }
            }
            if due(m, start, Some(end), every.months()) {
                if v < amount * (1.0 - 1e-12) {
                    return Some(m);
                }
                v -= amount;
            }
            m = m.plus(1);
        }
        None
    }

    /// The plan's own mix as one of the mixes compared.
    pub(crate) fn own_mix(&self) -> MixRow<'_> {
        let f = &self.figures;
        MixRow {
            name: &self.context.mix_name,
            weights: &f.weights,
            outcome: f.outcome,
            end: f.end_value,
            runs_out_weak: f.payments.as_ref().and_then(|p| p.runs_out_weak),
            risk: f.risk,
            crash: f.what_if.crash,
        }
    }

    /// The plan's mix and the alternatives, from the steadiest to the most volatile, with
    /// whether each is the plan's.
    pub(crate) fn mixes(&self) -> Vec<(MixRow<'_>, bool)> {
        let mut all = vec![(self.own_mix(), true)];
        all.extend(
            self.figures
                .alternatives
                .iter()
                .map(|m| (MixRow::of(m), false)),
        );
        all.sort_by(|a, b| a.0.risk.volatility.total_cmp(&b.0.risk.volatility));
        all
    }
}

/// One of the mixes compared: the plan's or an alternative.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MixRow<'a> {
    pub(crate) name: &'a str,
    pub(crate) weights: &'a [(String, f64)],
    pub(crate) outcome: Outcome,
    pub(crate) end: Percentiles,
    pub(crate) runs_out_weak: Option<Month>,
    pub(crate) risk: MixRisk,
    pub(crate) crash: Option<Outcome>,
}

impl MixRow<'_> {
    fn of(m: &MixOutcome) -> MixRow<'_> {
        MixRow {
            name: &m.name,
            weights: &m.weights,
            outcome: m.outcome,
            end: m.end,
            runs_out_weak: m.runs_out_weak,
            risk: m.risk,
            crash: m.what_if.crash,
        }
    }
}

// ── Words ────────────────────────────────────────────────────────────────────────────────────

/// Amounts of money in the plan's currency, if it has one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Money(pub(crate) Option<Currency>);

impl Money {
    fn sign(self) -> (&'static str, String) {
        match self.0 {
            None => ("", String::new()),
            Some(c) => match c.code() {
                "USD" => ("$", String::new()),
                "EUR" => ("€", String::new()),
                "GBP" => ("£", String::new()),
                "JPY" => ("¥", String::new()),
                code => ("", format!("{code} ")),
            },
        }
    }

    fn with_sign(self, digits: String, negative: bool) -> String {
        let (symbol, code) = self.sign();
        let minus = if negative { "−" } else { "" };
        format!("{code}{minus}{symbol}{digits}")
    }

    /// The amount in whole units, such as "$2,350".
    pub(crate) fn whole(self, x: f64) -> String {
        let x = if x.abs() < 0.5 { 0.0 } else { x };
        self.with_sign(crate::text::money(x.abs()), x < 0.0)
    }

    /// The amount to three significant figures, such as "$160,000".
    pub(crate) fn about(self, x: f64) -> String {
        self.whole(three_figures(x))
    }

    /// The amount compactly, such as "$1.2M" or "$250k".
    pub(crate) fn short(self, x: f64) -> String {
        let a = x.abs();
        let digits = if a >= 1e9 {
            format!("{}B", trim(a / 1e9))
        } else if a >= 1e6 {
            format!("{}M", trim(a / 1e6))
        } else if a >= 1e4 {
            format!("{:.0}k", a / 1e3)
        } else if a >= 1e3 {
            format!("{}k", trim(a / 1e3))
        } else {
            format!("{a:.0}")
        };
        self.with_sign(digits, x < 0.0 && a >= 0.5)
    }

    /// A change in an amount, such as "+$470" or "−$2,000".
    pub(crate) fn change(self, x: f64) -> String {
        let text = self.whole(x.abs());
        if x < 0.0 {
            format!("−{text}")
        } else {
            format!("+{text}")
        }
    }
}

fn trim(x: f64) -> String {
    let text = format!("{x:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

/// `x` rounded to three significant figures, and to whole units below 1,000.
pub(crate) fn three_figures(x: f64) -> f64 {
    let a = x.abs();
    if a < 1000.0 {
        return x.round();
    }
    let step = 10f64.powi(a.log10().floor() as i32 - 2);
    (x / step).round() * step
}

/// A share as a whole percentage, such as "63%".
pub(crate) fn pc(x: f64) -> String {
    let n = (100.0 * x).round() as i64;
    if n < 0 {
        format!("−{}%", -n)
    } else {
        format!("{n}%")
    }
}

/// A share as a percentage to one decimal, such as "2.5%".
pub(crate) fn pc1(x: f64) -> String {
    let text = format!("{:.1}%", 100.0 * x);
    match text.strip_prefix('-') {
        Some(rest) => format!("−{rest}"),
        None => text,
    }
}

/// A fee as a percentage, with as many decimals as it has, such as "0.25%".
pub(crate) fn fee_pc(x: f64) -> String {
    let text = format!("{:.2}", 100.0 * x);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}%")
}

/// "a" or "an" before a percentage, as it is read: "an 80% chance", "a 63% chance".
pub(crate) fn article(x: f64) -> &'static str {
    let n = (100.0 * x).round() as i64;
    if n == 8 || n == 11 || n == 18 || (80..90).contains(&n) {
        "an"
    } else {
        "a"
    }
}

/// A chance with its article, such as "an 80% chance".
pub(crate) fn a_chance(x: f64) -> String {
    format!("{} {} chance", article(x), pc(x))
}

/// A number of months in years and months, such as "3 years 2 months".
pub(crate) fn duration(n: i32) -> String {
    let n = n.unsigned_abs() as usize;
    let (y, m) = (n / 12, n % 12);
    let mut parts = Vec::new();
    if y > 0 {
        parts.push(crate::text::plural(y, "year"));
    }
    if m > 0 {
        parts.push(crate::text::plural(m, "month"));
    }
    if parts.is_empty() {
        "0 months".to_owned()
    } else {
        parts.join(" ")
    }
}

/// How often, as an adjective, such as "monthly".
pub(crate) fn adjective(f: Frequency) -> &'static str {
    match f {
        Frequency::Quarter => "quarterly",
        Frequency::Year => "yearly",
        _ => "monthly",
    }
}

/// The status's class on the page and its sign.
pub(crate) fn status_style(s: Status) -> (&'static str, &'static str) {
    match s {
        Status::RoomToSpare | Status::OnTrack => ("good", "✓"),
        Status::NearlyOnTrack => ("warn", "!"),
        _ => ("bad", "✕"),
    }
}

/// What is left at the end, or when the money runs out: for payments.
pub(crate) fn left_or_out(m: Money, value: f64, runs_out: Option<Month>) -> String {
    match runs_out {
        Some(month) => format!("Runs out {}", month.short_name()),
        None => format!("{} left", m.short(value)),
    }
}

/// The plan's outcome in a market: what a target or projection ends with, or what payments
/// leave or when they run out.
pub(crate) fn outcome_money(r: &PlanReport, o: &Outcome) -> String {
    let m = r.money();
    match r.kind() {
        Kind::Payments { .. } => left_or_out(m, o.typical, o.runs_out),
        _ => m.about(o.typical),
    }
}

/// The changes to reach or keep the chance wanted, as (label, change, detail).
pub(crate) fn levers(r: &PlanReport) -> Vec<(String, String, String)> {
    let Some(c) = &r.figures.changes else {
        return Vec::new();
    };
    let m = r.money();
    let up = c.to_reach;
    let until_goal = r.figures.flows.iter().any(|f| f.saving && f.until_goal);
    let mut out = Vec::new();
    if let Some(s) = &c.saving {
        let flow = s.flow.and_then(|i| r.figures.flows.get(i));
        let noun = flow.and_then(|f| f.every).map_or("month", Frequency::noun);
        let until = match (flow, r.kind()) {
            (Some(f), Kind::Payments { .. }) if f.until_goal => f
                .last
                .map(|l| format!(", until {}", l.long_name()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        let label = if up {
            format!("Saving more each {noun}")
        } else {
            format!("Saving less each {noun}")
        };
        out.push((
            label,
            m.change(s.to - s.from),
            format!(
                "{} a {noun} instead of {}{until}",
                m.whole(s.to),
                m.whole(s.from)
            ),
        ));
    }
    if let Some(s) = &c.start_value {
        let label = if up {
            "Adding to it today"
        } else {
            "Setting aside less today"
        };
        out.push((
            label.to_owned(),
            m.change(s.to - s.from),
            format!(
                "Starting with {} instead of {}",
                m.whole(s.to),
                m.whole(s.from)
            ),
        ));
    }
    if let Some(d) = &c.date {
        let months = d.from.months_until(d.to);
        let saving = if until_goal {
            ", saving until then"
        } else {
            ""
        };
        let (label, detail) = match (r.kind(), months >= 0) {
            (Kind::Payments { .. }, true) => (
                "Starting the payments later",
                format!("Starting in {}{saving}", d.to.long_name()),
            ),
            (Kind::Payments { .. }, false) => (
                "Starting the payments sooner",
                format!("Starting in {}{saving}", d.to.long_name()),
            ),
            (_, true) => (
                "Allowing more time",
                format!("Reaching it by {}{saving}", d.to.long_name()),
            ),
            (_, false) => (
                "Reaching it sooner",
                format!("By {}{saving}", d.to.long_name()),
            ),
        };
        let sign = if months >= 0 { "+" } else { "−" };
        out.push((
            label.to_owned(),
            format!("{sign}{}", duration(months)),
            detail,
        ));
    }
    if let Some(a) = &c.amount {
        let (label, detail) = match r.kind() {
            Kind::Payments { every, .. } => (
                if a.to < a.from {
                    "Taking a little less"
                } else {
                    "Taking a little more"
                },
                format!(
                    "{} a {} instead of {}",
                    m.whole(a.to),
                    every.noun(),
                    m.whole(a.from)
                ),
            ),
            _ => (
                if a.to < a.from {
                    "A lower target"
                } else {
                    "A higher target"
                },
                format!("{} instead of {}", m.whole(a.to), m.whole(a.from)),
            ),
        };
        out.push((label.to_owned(), m.change(a.to - a.from), detail));
    }
    out
}

/// The heading of the changes, such as "Getting on track".
pub(crate) fn levers_heading(r: &PlanReport) -> &'static str {
    match (&r.figures.changes, r.figures.status) {
        (Some(c), _) if c.to_reach => "Getting on track",
        (_, Some(Status::RoomToSpare)) => "Room to spare",
        _ => "How much room there is",
    }
}

/// The sentence after the changes, on the mixes compared.
pub(crate) fn levers_note(r: &PlanReport) -> String {
    let wanted = r.wanted();
    let Some(c) = &r.figures.changes else {
        return String::new();
    };
    let any = !levers(r).is_empty();
    let mut out = if !any && c.to_reach {
        format!("No single change within reach gives {}.", a_chance(wanted))
    } else if c.to_reach {
        "Any one of these would be enough on its own.".to_owned()
    } else {
        format!(
            "Any one of these on its own would still keep {}.",
            a_chance(wanted)
        )
    };
    let alts = &r.figures.alternatives;
    let best = alts.iter().max_by(|a, b| {
        a.outcome
            .chance
            .unwrap_or(0.0)
            .total_cmp(&b.outcome.chance.unwrap_or(0.0))
    });
    if let Some(best) = best {
        let chance = best.outcome.chance.unwrap_or(0.0);
        if c.to_reach && chance < wanted {
            out.push_str(&format!(
                " A different investment mix wouldn't: the best of the mixes we tried, {}, gives {}.",
                best.name,
                a_chance(chance)
            ));
        } else if c.to_reach {
            out.push_str(&format!(
                " So would a different mix: {} gives {}.",
                best.name,
                a_chance(chance)
            ));
        } else {
            // A steadier mix that still keeps the chance wanted.
            let worst = |w: Option<f64>| w.unwrap_or(f64::NEG_INFINITY);
            let own = worst(r.figures.risk.worst_year);
            let steadier = alts
                .iter()
                .filter(|a| {
                    a.outcome.chance.unwrap_or(0.0) >= wanted && worst(a.risk.worst_year) > own
                })
                .max_by(|a, b| worst(a.risk.worst_year).total_cmp(&worst(b.risk.worst_year)));
            if let Some(s) = steadier {
                out.push_str(&format!(
                    " So would a steadier mix: {} gives {}, with a worst year in 20 of {} instead of {}.",
                    s.name,
                    a_chance(s.outcome.chance.unwrap_or(0.0)),
                    pc(worst(s.risk.worst_year)),
                    pc(own)
                ));
            }
        }
    }
    out
}

/// The plan's tests when markets do worse, short enough for a column of text.
pub(crate) fn stress_labels(r: &PlanReport) -> [&'static str; 3] {
    let crash = match r.kind() {
        Kind::Payments { .. } => "Crash as payments start",
        Kind::Target { .. } => "Crash before the goal",
        Kind::Projection => "Crash in the last year",
    };
    ["Your plan as it stands", "Returns 1% a year lower", crash]
}

/// The plan's tests when markets do worse, as (title, description, outcome).
pub(crate) fn stress_rows(r: &PlanReport) -> Vec<(String, String, Outcome)> {
    let f = &r.figures;
    let typical = f.risk.typical_return;
    let mut rows = vec![
        (
            "Your plan as it stands".to_owned(),
            format!(
                "Markets as in our {} scenarios",
                crate::text::money(f.simulations as f64)
            ),
            f.outcome,
        ),
        (
            "Returns are lower than expected".to_owned(),
            format!(
                "Your mix earns 1% a year less: typically {} a year after inflation, not {}",
                pc1(typical - 0.01),
                pc1(typical)
            ),
            f.what_if.lower_returns,
        ),
    ];
    if let (Some(crash), Some(w)) = (f.what_if.crash, f.what_if.crash_window) {
        // Twelve months, or as many as the plan has left.
        let span = match w.from.months_until(w.to) + 1 {
            12 => "year".to_owned(),
            1 => "month".to_owned(),
            n => format!("{n} months"),
        };
        let (title, when) = match r.kind() {
            Kind::Payments { .. } => (
                "Markets crash as the payments start".to_owned(),
                format!("in the first {span} of payments"),
            ),
            Kind::Target { by, .. } => (
                "Markets crash just before the money is needed".to_owned(),
                format!("in the {span} before {}", by.long_name()),
            ),
            Kind::Projection => (
                "Markets crash in the last year".to_owned(),
                format!("in the {span} to {}", r.end_long()),
            ),
        };
        let moves = if w.loss < 0.0 {
            format!("falls {}", pc(-w.loss))
        } else {
            format!("gains only {}", pc(w.loss))
        };
        rows.push((
            title,
            format!(
                "Your mix {moves} {when}, as it did from {} to {}",
                w.from.short_name(),
                w.to.short_name()
            ),
            crash,
        ));
    }
    rows
}

/// What the tests when markets do worse mean for the plan, in a few sentences.
pub(crate) fn stress_meaning(r: &PlanReport) -> Option<String> {
    let f = &r.figures;
    let plan = f.outcome.chance?;
    let wanted = r.wanted();
    let lower = f.what_if.lower_returns.chance.unwrap_or(0.0);
    let crash = f.what_if.crash.and_then(|c| c.chance);
    let payments = matches!(r.kind(), Kind::Payments { .. });
    let mut out = Vec::new();
    let years = r.figures.start.months_until(r.figures.end) as f64 / 12.0;
    match crash {
        Some(crash) if plan - crash > plan - lower + 0.02 => out.push(if payments {
            "A crash as the payments start is the bigger danger: the payments keep selling \
             investments while prices are low, so less is left to recover."
                .to_owned()
        } else {
            "A crash just before the money is needed is the bigger danger: there is no time to \
             recover."
                .to_owned()
        }),
        Some(crash) if plan - lower > plan - crash + 0.02 => out.push(format!(
            "Lower returns are the bigger danger for this plan: over {}, a small difference each \
             year adds up.",
            if years >= 2.0 {
                format!("{} years", years.round())
            } else {
                duration(r.figures.start.months_until(r.figures.end))
            }
        )),
        Some(crash) => out.push(format!(
            "Both matter about as much: lower returns take your chance to {}, and a crash to {}.",
            pc(lower),
            pc(crash)
        )),
        None => {}
    }
    if f.status.is_some_and(Status::meets) {
        match crash {
            Some(crash) if lower >= wanted && crash >= wanted => out.push(format!(
                "The plan keeps at least the {} you want in both.",
                pc(wanted)
            )),
            _ if lower >= wanted => out.push(format!(
                "The room to spare absorbs lower returns: the chance stays at {}, above the {} you \
                 want.",
                pc(lower),
                pc(wanted)
            )),
            _ => {}
        }
    }
    if let Kind::Target { amount, .. } = r.kind() {
        let spare = f.outcome.typical - amount;
        if spare > 0.0 && spare < 0.1 * amount {
            out.push(format!(
                "The plan has little to spare: typically you would have only {} more than the goal.",
                r.money().about(spare)
            ));
        }
    }
    if let Some(crash) = crash {
        let own = f.what_if.crash.map_or(0.0, |c| c.typical);
        let best = f
            .alternatives
            .iter()
            .filter_map(|a| a.what_if.crash.map(|c| (a, c)))
            .max_by(|a, b| {
                a.1.chance
                    .unwrap_or(0.0)
                    .total_cmp(&b.1.chance.unwrap_or(0.0))
            });
        if let Some((alt, c)) = best
            && c.chance.unwrap_or(0.0) > crash + 0.02
        {
            let alt_chance = c.chance.unwrap_or(0.0);
            if payments {
                out.push(format!(
                    "Of the mixes we tried, {} holds up best in the same crash, at {}.",
                    alt.name,
                    pc(alt_chance)
                ));
            } else {
                let kind = if alt.risk.volatility < f.risk.volatility {
                    "A more cautious mix"
                } else {
                    "A different mix"
                };
                let but = if alt_chance < wanted {
                    " but not avoid it"
                } else {
                    ""
                };
                out.push(format!(
                    "{kind} would soften the blow{but}: in the same crash, {} would typically leave \
                     you {}, against {} with your mix.",
                    alt.name,
                    r.money().about(c.typical),
                    r.money().about(own)
                ));
            }
        }
    }
    (!out.is_empty()).then(|| out.join(" "))
}

/// The plan's note on today's money, as agreed for every amount in the report.
pub(crate) fn todays_money(r: &PlanReport) -> String {
    let c = &r.context;
    let m = r.money();
    let fee = if c.fee > 0.0 {
        format!(" and fees ({} a year)", fee_pc(c.fee))
    } else {
        String::new()
    };
    let example_amount = match r.kind() {
        Kind::Target { amount, .. } | Kind::Payments { amount, .. } => amount,
        Kind::Projection => r
            .figures
            .flows
            .iter()
            .map(|f| f.amount)
            .fold(c.start_value, f64::max),
    };
    let year = r.last_month().year();
    let example = format!(
        "{} in {year} buys what {} buys today",
        m.whole(example_amount),
        m.whole(example_amount)
    );
    let mut out = match &c.inflation {
        InflationBasis::Rate { rate } if rate.abs() < 1e-12 => {
            if c.fee > 0.0 {
                format!(
                    "No inflation is assumed, so amounts are in money of the day; returns are after fees ({} a year).",
                    fee_pc(c.fee)
                )
            } else {
                "No inflation is assumed, so amounts are in money of the day.".to_owned()
            }
        }
        InflationBasis::Rate { rate } => format!(
            "Returns are after inflation ({} a year){fee}, so every amount is in today's money: {example}.",
            pc1(*rate)
        ),
        InflationBasis::Index { name, .. } => format!(
            "Returns are after inflation (the {name}'s, month by month){fee}, so every amount is in today's money: {example}."
        ),
    };
    for f in r.figures.flows.iter().filter(|f| f.growth.is_some()) {
        out.push_str(&format!(
            " {} is set in money of the day instead, rising {} a year.",
            f.name,
            pc1(f.growth.unwrap_or(0.0))
        ));
    }
    out
}

/// The plan as steps in time: (month, when, what, detail, amount).
pub(crate) fn steps(r: &PlanReport) -> Vec<(Month, String, String, String, String)> {
    let m = r.money();
    let f = &r.figures;
    let wanted = pc(r.wanted());
    let mut out = Vec::new();
    if r.context.start_value > 0.0 {
        out.push((
            f.start,
            f.start.long_name(),
            "Already set aside".to_owned(),
            "What the plan starts with".to_owned(),
            m.whole(r.context.start_value),
        ));
    }
    for flow in &f.flows {
        let sign = if flow.saving { 1.0 } else { -1.0 };
        let (when, detail) = match (flow.every, flow.last) {
            (None, _) => (
                flow.first.long_name(),
                if flow.saving {
                    "Paid in as planned".to_owned()
                } else {
                    "Taken out as planned".to_owned()
                },
            ),
            (Some(every), Some(last)) => (
                format!(
                    "Every {}, {} to {}",
                    every.noun(),
                    flow.first.short_name(),
                    last.short_name()
                ),
                crate::text::plural(flow.count, "payment"),
            ),
            (Some(every), None) => (
                format!("Every {} from {}", every.noun(), flow.first.short_name()),
                "None falls due before the goal".to_owned(),
            ),
        };
        let detail = match flow.growth {
            Some(g) => format!("{detail}, rising {} a year", pc1(g)),
            None => detail,
        };
        out.push((
            flow.first,
            when,
            flow.name.clone(),
            detail,
            m.whole(sign * flow.amount),
        ));
    }
    let goal = format!("Goal: {}", r.goal_name());
    match r.kind() {
        Kind::Target { amount, by } => out.push((
            by,
            by.long_name(),
            goal,
            format!("Chance wanted: at least {wanted}"),
            m.whole(amount),
        )),
        Kind::Payments {
            amount,
            every,
            start,
            end,
        } => {
            let count = start.months_until(end) as usize / every.months() + 1;
            out.push((
                start,
                format!(
                    "Every {}, {} to {}",
                    every.noun(),
                    start.short_name(),
                    end.short_name()
                ),
                goal,
                format!(
                    "{count} {} payments; chance wanted: at least {wanted} that every one is paid in full",
                    adjective(every)
                ),
                m.whole(-amount),
            ));
        }
        Kind::Projection => {}
    }
    out.sort_by_key(|s| s.0);
    out
}

/// The notes under the plan's steps.
pub(crate) fn steps_note(r: &PlanReport) -> String {
    let mut out = todays_money(r);
    let spending = r.figures.flows.iter().any(|f| !f.saving);
    match r.kind() {
        Kind::Payments { .. } => out.push_str(
            " If there is ever not enough left for a payment, the plan pays what it can and \
             counts that payment as missed.",
        ),
        _ if spending => out.push_str(
            " If there is ever not enough left for a payment, the plan pays what it can.",
        ),
        _ => {}
    }
    out
}

/// How markets were simulated, in a sentence or two.
pub(crate) fn scenarios_method(r: &PlanReport) -> String {
    let c = &r.context;
    let f = &r.figures;
    let sims = crate::text::money(f.simulations as f64);
    let span = format!(
        "{} to {}",
        c.history_start.short_name(),
        c.history_end.short_name()
    );
    let mut out = match c.model {
        Model::Bootstrap { block } => format!(
            "We simulated {sims} possible paths for the markets, month by month, by replaying \
             blocks of past monthly returns, about {} months at a time, in a random order (a \
             stationary block bootstrap). The past returns are the assets' monthly returns from \
             {span}.",
            trim(block)
        ),
        Model::Normal => format!(
            "We simulated {sims} possible paths for the markets, month by month, drawing each \
             month's returns from a normal distribution with the mean returns and covariance of \
             the assets' monthly returns from {span}."
        ),
        _ => format!(
            "We replayed every stretch of the assets' monthly returns from {span} as long as the \
             plan, in order: {sims} in all."
        ),
    };
    if let Some(e) = &c.expected_returns {
        let given: Vec<String> = e
            .iter()
            .map(|(a, r)| format!("{a} {} a year", pc1(*r)))
            .collect();
        let how = match c.model {
            Model::Normal => "are the distribution's means",
            _ => "shift every past month of each asset by the gap to its average",
        };
        out.push_str(&format!(
            " The expected returns given {how}: {}.",
            given.join(", ")
        ));
    }
    if let InflationBasis::Index { name, average } = &c.inflation {
        out.push_str(&format!(
            " Inflation is the {name}'s in the same months, which averaged {} a year.",
            pc1(*average)
        ));
    }
    out
}

/// The limits of the method, in words.
pub(crate) fn limits(r: &PlanReport) -> String {
    match r.context.model {
        Model::Normal => "The normal distribution has fewer extreme months than markets have \
                          shown, so large falls are rarer in it than in the history. The chance \
                          is an estimate, not a promise."
            .to_owned(),
        _ if r.context.expected_returns.is_some() => "The scenarios replay months that \
            happened, shifted to the expected returns given, so none has a month much worse than \
            the worst in the data. The chance is an estimate from past markets, not a promise."
            .to_owned(),
        _ => "The scenarios only replay months that happened, so none has a month worse than the \
              worst in the data. The chance is an estimate from past markets, not a promise."
            .to_owned(),
    }
}

/// The method notes, as (term, explanation), for the page (`page`) or the text.
pub(crate) fn method_notes(r: &PlanReport, page: bool) -> Vec<(String, String)> {
    let f = &r.figures;
    let wanted = r.wanted();
    let m = r.money();
    let mut out = vec![
        ("Market scenarios".to_owned(), scenarios_method(r)),
        (
            "Strong, average and weak markets".to_owned(),
            "The average market is the middle scenario (the median). Only 1 scenario in 10 does \
             better than the strong market (the 90th percentile), and only 1 in 10 does worse \
             than the weak market (the 10th)."
                .to_owned(),
        ),
    ];
    match r.kind() {
        Kind::Target { amount, by } => out.push((
            "Chance".to_owned(),
            format!(
                "The share of scenarios in which the value at the start of {} is at least {}.",
                by.long_name(),
                m.whole(amount)
            ),
        )),
        Kind::Payments { start, end, .. } => out.push((
            "Chance".to_owned(),
            format!(
                "The share of scenarios in which every payment from {} to {} is made in full.",
                start.long_name(),
                end.long_name()
            ),
        )),
        Kind::Projection => {}
    }
    if f.status.is_some() {
        out.push((
            "Status".to_owned(),
            format!(
                "Room to spare when the chance is at least {}; on track when it is at least the \
                 {} you want; nearly on track when it is up to 15 points below it; off track \
                 below that.{}",
                pc(f.spare_from),
                pc(wanted),
                if page {
                    " The thin green arc on the ring marks the on-track zone."
                } else {
                    ""
                }
            ),
        ));
    }
    out.push((
        "Today's money".to_owned(),
        format!("{} Taxes aren't included.", todays_money(r)),
    ));
    if let Some(c) = &f.changes {
        let text = if c.to_reach {
            format!(
                "Each change is worked out on the same scenarios, one at a time: the smallest \
                 change that gives {}.",
                a_chance(wanted)
            )
        } else {
            format!(
                "Each change is worked out on the same scenarios, one at a time: the largest \
                 change that still keeps {}.",
                a_chance(wanted)
            )
        };
        out.push((levers_heading(r).to_owned(), text));
    }
    let spending: Vec<String> = f
        .flows
        .iter()
        .filter_map(|s| {
            s.paid_in_full
                .map(|p| format!("{} is paid in full in {} of scenarios", s.name, pc(p)))
        })
        .collect();
    if !spending.is_empty() {
        out.push((
            "Spending".to_owned(),
            format!(
                "Spending is paid as far as the money goes: {}.",
                spending.join("; ")
            ),
        ));
    }
    if matches!(r.kind(), Kind::Payments { .. }) {
        out.push((
            "Runs out".to_owned(),
            "The month the money can no longer pay in full. In a weak market, that is the month \
             by which 1 scenario in 10 has run out; in an average market, half have."
                .to_owned(),
        ));
    }
    let crash = match f.what_if.crash_window {
        Some(w) => format!(
            " A crash: the markets of the worst {} in the data for your mix ({} to {}) \
             replace the same months of every scenario{}.",
            match w.from.months_until(w.to) + 1 {
                12 => "12 months".to_owned(),
                1 => "month".to_owned(),
                n => format!("{n} months (as many as the plan has left)"),
            },
            w.from.short_name(),
            w.to.short_name(),
            if f.alternatives.is_empty() {
                ""
            } else {
                "; other mixes are tested with the same months"
            }
        ),
        None => String::new(),
    };
    out.push((
        "What if markets do worse".to_owned(),
        format!("Lower returns: every scenario earns 1 percentage point a year less.{crash}"),
    ));
    let years = f.start.months_until(f.end) as f64 / 12.0;
    out.push((
        "Worst year and largest fall".to_owned(),
        format!(
            "Worst year in 20: only 1 twelve-month stretch in 20 of the scenarios loses more. \
             Largest fall: the typical largest fall from a peak over the plan's {}, before any \
             money is paid in or out. Volatility: of the mix's monthly returns in the history, \
             a year, before inflation.",
            if years >= 2.0 {
                format!("{} years", years.round())
            } else {
                duration(f.start.months_until(f.end))
            }
        ),
    ));
    out.push(("What this can't tell you".to_owned(), limits(r)));
    out.push((
        "Repeatable".to_owned(),
        format!(
            "Seed {}: the same plan gives the same report.",
            r.context.seed
        ),
    ));
    out
}

/// The disclosure on projections, in full.
pub(crate) const IMPORTANT: &str = "The projections in this report about the likelihood of \
    investment outcomes are hypothetical in nature, do not reflect actual investment results and \
    are not guarantees of future results. Results may vary with each use and over time: \
    different data, assumptions or seeds give different figures, while the same inputs and seed \
    give the same report.";

/// The disclosure that the report is not advice, in full.
pub(crate) const NOT_ADVICE: &str = "This report is for illustration and information only. It \
    is not investment, financial, tax or legal advice, and it is not a recommendation or an \
    offer to buy or sell any security or to follow any strategy. The changes it shows illustrate \
    what the figures would need; they are not suggestions to act. Before making financial \
    decisions, consider your own circumstances and, where appropriate, seek advice from a \
    qualified professional.";

// ── Tables ───────────────────────────────────────────────────────────────────────────────────

fn col(key: &str, label: &str, kind: ColumnKind) -> Column {
    Column {
        key: key.to_owned(),
        label: label.to_owned(),
        kind,
    }
}

fn table(name: &str, title: &str, columns: Vec<Column>, rows: Vec<Vec<Cell>>) -> ReportTable {
    ReportTable {
        name: name.to_owned(),
        title: title.to_owned(),
        columns,
        rows,
    }
}

fn number(x: f64) -> Cell {
    if x.is_finite() {
        Cell::Number(x)
    } else {
        Cell::Missing
    }
}

fn maybe(x: Option<f64>) -> Cell {
    x.map_or(Cell::Missing, number)
}

fn month_cell(m: Option<Month>) -> Cell {
    m.map_or(Cell::Missing, |m| Cell::Text(m.to_string()))
}

impl PlanReport {
    /// The report as labelled tables for export: the outcome, the plan's flows, the value over
    /// time, the chance by month for a target, the changes, the tests when markets do worse,
    /// and the mixes compared.
    pub fn tables(&self) -> Vec<ReportTable> {
        let f = &self.figures;
        let amount = match self.context.currency {
            Some(currency) => ColumnKind::Money { currency },
            None => ColumnKind::Number,
        };
        let mut out = Vec::new();

        // The outcome.
        let p = f.payments.as_ref();
        out.push(table(
            "outcome",
            "Outcome",
            vec![
                col("status", "Status", ColumnKind::Text),
                col("chance", "Chance", ColumnKind::Fraction),
                col("chance_wanted", "Chance wanted", ColumnKind::Fraction),
                col("end_weak", "Weak market", amount),
                col("end_average", "Average market", amount),
                col("end_strong", "Strong market", amount),
                col("shortfall", "Typical shortfall", amount),
                col("paid", "Share of payments made", ColumnKind::Fraction),
                col("runs_out_weak", "Runs out, weak market", ColumnKind::Text),
                col(
                    "runs_out_average",
                    "Runs out, average market",
                    ColumnKind::Text,
                ),
            ],
            vec![vec![
                f.status
                    .map_or(Cell::Missing, |s| Cell::Text(s.label().to_owned())),
                maybe(f.outcome.chance),
                number(f.chance_wanted),
                number(f.end_value.p10),
                number(f.end_value.p50),
                number(f.end_value.p90),
                maybe(f.shortfall),
                maybe(p.map(|p| p.paid)),
                month_cell(p.and_then(|p| p.runs_out_weak)),
                month_cell(p.and_then(|p| p.runs_out_average)),
            ]],
        ));

        out.push(table(
            "flows",
            "Money in and out",
            vec![
                col("flow", "Flow", ColumnKind::Text),
                col("direction", "In or out", ColumnKind::Text),
                col("amount", "Amount", amount),
                col("every", "Every", ColumnKind::Text),
                col("first", "First", ColumnKind::Text),
                col("last", "Last", ColumnKind::Text),
                col("count", "Payments", ColumnKind::Integer),
                col("growth", "Growth a year", ColumnKind::Fraction),
                col("paid_in_full", "Paid in full", ColumnKind::Fraction),
            ],
            f.flows
                .iter()
                .map(|s| {
                    vec![
                        Cell::Text(s.name.clone()),
                        Cell::Text(if s.saving { "In" } else { "Out" }.to_owned()),
                        number(s.amount),
                        s.every
                            .map_or(Cell::Missing, |e| Cell::Text(e.noun().to_owned())),
                        month_cell(Some(s.first)),
                        month_cell(s.last),
                        number(s.count as f64),
                        maybe(s.growth),
                        maybe(s.paid_in_full),
                    ]
                })
                .collect(),
        ));

        out.push(table(
            "value_over_time",
            "Value over time",
            vec![
                col("month", "Month", ColumnKind::Text),
                col("p10", "Weak market (10th percentile)", amount),
                col("p25", "25th percentile", amount),
                col("p50", "Average market (median)", amount),
                col("p75", "75th percentile", amount),
                col("p90", "Strong market (90th percentile)", amount),
            ],
            f.months
                .iter()
                .zip(&f.values)
                .map(|(m, v)| {
                    vec![
                        Cell::Text(m.to_string()),
                        number(v.p10),
                        number(v.p25),
                        number(v.p50),
                        number(v.p75),
                        number(v.p90),
                    ]
                })
                .collect(),
        ));

        if !f.chance_by_month.is_empty() {
            out.push(table(
                "chance_by_month",
                "Chance by month",
                vec![
                    col("month", "Month", ColumnKind::Text),
                    col(
                        "chance",
                        "Chance of having the target",
                        ColumnKind::Fraction,
                    ),
                ],
                f.chance_by_month
                    .iter()
                    .enumerate()
                    .map(|(t, c)| vec![Cell::Text(f.start.plus(t as i32).to_string()), number(*c)])
                    .collect(),
            ));
        }

        if let Some(c) = &f.changes {
            let mut rows = Vec::new();
            for a in [&c.saving, &c.start_value, &c.amount].into_iter().flatten() {
                rows.push(vec![
                    Cell::Text(a.what.clone()),
                    number(a.from),
                    number(a.to),
                    Cell::Missing,
                    Cell::Missing,
                ]);
            }
            if let Some(d) = &c.date {
                rows.push(vec![
                    Cell::Text(d.what.clone()),
                    Cell::Missing,
                    Cell::Missing,
                    month_cell(Some(d.from)),
                    month_cell(Some(d.to)),
                ]);
            }
            let title = if c.to_reach {
                "Changes that reach the chance wanted"
            } else {
                "Changes that keep the chance wanted"
            };
            out.push(table(
                "changes",
                title,
                vec![
                    col("change", "Change", ColumnKind::Text),
                    col("from", "From", amount),
                    col("to", "To", amount),
                    col("from_month", "From month", ColumnKind::Text),
                    col("to_month", "To month", ColumnKind::Text),
                ],
                rows,
            ));
        }

        out.push(table(
            "what_if",
            "When markets do worse",
            vec![
                col("situation", "Situation", ColumnKind::Text),
                col("chance", "Chance", ColumnKind::Fraction),
                col("average", "Average market", amount),
                col("runs_out", "Runs out, average market", ColumnKind::Text),
            ],
            stress_rows(self)
                .into_iter()
                .map(|(title, _, o)| {
                    vec![
                        Cell::Text(title),
                        maybe(o.chance),
                        number(o.typical),
                        month_cell(o.runs_out),
                    ]
                })
                .collect(),
        ));

        out.push(table(
            "mixes",
            "Investment mixes",
            vec![
                col("mix", "Mix", ColumnKind::Text),
                col("yours", "Your mix", ColumnKind::Boolean),
                col("chance", "Chance", ColumnKind::Fraction),
                col("end_weak", "Weak market", amount),
                col("end_average", "Average market", amount),
                col("end_strong", "Strong market", amount),
                col("runs_out_weak", "Runs out, weak market", ColumnKind::Text),
                col(
                    "typical_return",
                    "Typical return a year",
                    ColumnKind::Fraction,
                ),
                col("worst_year", "Worst year in 20", ColumnKind::Fraction),
                col("largest_fall", "Largest fall", ColumnKind::Fraction),
                col("volatility", "Volatility", ColumnKind::Fraction),
                col("crash_chance", "Chance after a crash", ColumnKind::Fraction),
            ],
            self.mixes()
                .into_iter()
                .map(|(m, yours)| {
                    vec![
                        Cell::Text(m.name.to_owned()),
                        Cell::Boolean(yours),
                        maybe(m.outcome.chance),
                        number(m.end.p10),
                        number(m.end.p50),
                        number(m.end.p90),
                        month_cell(m.runs_out_weak),
                        number(m.risk.typical_return),
                        maybe(m.risk.worst_year),
                        number(m.risk.largest_fall),
                        number(m.risk.volatility),
                        maybe(m.crash.and_then(|c| c.chance)),
                    ]
                })
                .collect(),
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_read_as_people_say_them() {
        let usd = Money(Some(Currency::new("USD").unwrap()));
        assert_eq!(usd.whole(2350.4), "$2,350");
        assert_eq!(usd.whole(-20_000.0), "−$20,000");
        assert_eq!(usd.about(160_437.0), "$160,000");
        assert_eq!(usd.about(63_812.0), "$63,800");
        assert_eq!(usd.short(1_234_000.0), "$1.2M");
        assert_eq!(usd.short(250_000.0), "$250k");
        assert_eq!(usd.short(2_500.0), "$2.5k");
        assert_eq!(usd.change(470.0), "+$470");
        assert_eq!(usd.change(-2_000.0), "−$2,000");
        let chf = Money(Some(Currency::new("CHF").unwrap()));
        assert_eq!(chf.whole(1_000.0), "CHF 1,000");
        assert_eq!(Money(None).whole(1_000.0), "1,000");
        assert_eq!(a_chance(0.8), "an 80% chance");
        assert_eq!(a_chance(0.63), "a 63% chance");
        assert_eq!(a_chance(0.18), "an 18% chance");
        assert_eq!(duration(40), "3 years 4 months");
        assert_eq!(duration(12), "1 year");
        assert_eq!(fee_pc(0.0025), "0.25%");
        assert_eq!(fee_pc(0.01), "1%");
        assert_eq!(pc(-0.26), "−26%");
    }
}
