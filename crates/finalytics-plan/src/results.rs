//! What a plan's simulations show.

use serde::{Deserialize, Serialize};

use crate::Month;
use crate::options::Frequency;

/// Where a plan stands against the chance wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// The chance is at least the one from which there is room to spare.
    RoomToSpare,
    /// The chance is at least the one wanted.
    OnTrack,
    /// The chance is up to 15 points below the one wanted.
    NearlyOnTrack,
    /// The chance is more than 15 points below the one wanted.
    OffTrack,
}

impl Status {
    /// The status of `chance` against the chance `wanted` and the chance `spare` from which
    /// there is room to spare.
    pub fn of(chance: f64, wanted: f64, spare: f64) -> Status {
        if chance >= spare {
            Status::RoomToSpare
        } else if chance >= wanted {
            Status::OnTrack
        } else if wanted - chance <= 0.15 + 1e-12 {
            Status::NearlyOnTrack
        } else {
            Status::OffTrack
        }
    }

    /// The status in words, such as "Nearly on track".
    pub fn label(self) -> &'static str {
        match self {
            Status::RoomToSpare => "Room to spare",
            Status::OnTrack => "On track",
            Status::NearlyOnTrack => "Nearly on track",
            Status::OffTrack => "Off track",
        }
    }

    /// Whether the chance is at least the one wanted.
    pub fn meets(self) -> bool {
        matches!(self, Status::RoomToSpare | Status::OnTrack)
    }
}

/// A flow as simulated: when it falls due and how often.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FlowSummary {
    /// The flow's name, such as "Monthly saving".
    pub name: String,
    /// Whether money is paid in (a saving) or taken out (spending).
    pub saving: bool,
    /// The amount each time, in today's money unless the flow has a growth rate.
    pub amount: f64,
    /// How often it falls due; absent for a single payment.
    pub every: Option<Frequency>,
    /// The first month it falls due.
    pub first: Month,
    /// The last month it falls due; absent when it never does.
    pub last: Option<Month>,
    /// The number of times it falls due.
    pub count: usize,
    /// Whether it runs until the goal: to the month before a target's date or the first payment.
    pub until_goal: bool,
    /// The growth a year in money of the day, when it does not rise with inflation.
    pub growth: Option<f64>,
    /// For spending, the share of simulations in which every payment of it is made in full.
    pub paid_in_full: Option<f64>,
}

/// The 10th, 25th, 50th, 75th and 90th percentiles of a value across the simulations.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Percentiles {
    /// The 10th percentile: the weak market, 1 simulation in 10 below it.
    pub p10: f64,
    /// The 25th percentile.
    pub p25: f64,
    /// The median: the average market.
    pub p50: f64,
    /// The 75th percentile.
    pub p75: f64,
    /// The 90th percentile: the strong market, 1 simulation in 10 above it.
    pub p90: f64,
}

/// One simulation's value at each recorded month.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct SamplePath {
    /// The value at the start of each of the recorded months, in today's money.
    pub values: Vec<f64>,
    /// Whether this simulation meets the goal.
    pub met: bool,
}

/// A change of an amount that reaches, or keeps, the chance wanted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AmountChange {
    /// What is changed, such as "Monthly saving".
    pub what: String,
    /// The flow changed, by its place in the plan's flows; absent for the start value or the
    /// goal's amount.
    #[serde(default)]
    pub flow: Option<usize>,
    /// The amount in the plan.
    pub from: f64,
    /// The amount that gives the chance wanted, the rest of the plan as it is.
    pub to: f64,
}

/// A change of a month that reaches, or keeps, the chance wanted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MonthChange {
    /// What is changed: the target's date, or the month of the first payment.
    pub what: String,
    /// The month in the plan.
    pub from: Month,
    /// The month that gives the chance wanted, saving until then.
    pub to: Month,
}

/// What would change the outcome, one change at a time on the same simulations: below the
/// chance wanted, the smallest change that reaches it; at or above it, the largest change the
/// plan could absorb and keep it. A change that no amount within reach achieves is absent.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Changes {
    /// Whether the changes reach the chance wanted (true) or keep it (false).
    pub to_reach: bool,
    /// The plan's largest regular saving.
    pub saving: Option<AmountChange>,
    /// The value at the start.
    pub start_value: Option<AmountChange>,
    /// The goal's amount: the target, or each payment.
    pub amount: Option<AmountChange>,
    /// The target's date, or the month of the first payment, saving until then.
    pub date: Option<MonthChange>,
}

/// The outcome of the plan, or of one of its tests.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Outcome {
    /// The chance of meeting the goal; absent without a goal.
    pub chance: Option<f64>,
    /// The value at the end in the average market (the median), in today's money.
    pub typical: f64,
    /// For payments, the month by which half the simulations have run short, if they do.
    pub runs_out: Option<Month>,
}

/// The worst run of twelve months in the history, for the plan's weights.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CrashWindow {
    /// Its first month.
    pub from: Month,
    /// Its last month.
    pub to: Month,
    /// The plan's return over it, in money of the day, such as -0.26.
    pub loss: f64,
    /// The first month of the plan it is placed at.
    pub placed: Month,
}

/// The plan when markets do worse than the simulations assume.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct WhatIf {
    /// Every simulated month's return a percentage point a year lower.
    pub lower_returns: Outcome,
    /// The worst twelve months of the history placed before the target's date or at the first
    /// payment, in every simulation; absent with less than twelve months to place them in.
    pub crash: Option<Outcome>,
    /// The months placed.
    pub crash_window: Option<CrashWindow>,
}

/// The risk of a mix over the plan, before any money is paid in or out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MixRisk {
    /// The typical return a year after inflation and fees: the median across the simulations.
    pub typical_return: f64,
    /// The 5th percentile of twelve-month returns: 1 year in 20 is worse; absent for a plan
    /// shorter than a year.
    pub worst_year: Option<f64>,
    /// The median of each simulation's largest fall from a peak.
    pub largest_fall: f64,
    /// The volatility a year of the mix's returns in the history, in money of the day.
    pub volatility: f64,
}

/// What a goal of payments comes to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PaymentsOutcome {
    /// The share of all the planned payments made, across the simulations.
    pub paid: f64,
    /// The share made in the simulations that run short; absent if none does.
    pub paid_when_short: Option<f64>,
    /// The typical month the money runs short, among the simulations that run short.
    pub runs_short: Option<Month>,
    /// The month by which 1 simulation in 10 has run short: the weak market.
    pub runs_out_weak: Option<Month>,
    /// The month by which half the simulations have run short: the average market.
    pub runs_out_average: Option<Month>,
    /// The month by which 9 simulations in 10 have run short: the strong market.
    pub runs_out_strong: Option<Month>,
    /// The last month paid in full in at least the chance wanted of the simulations.
    pub lasts_until: Option<Month>,
}

/// One mix compared with the plan's on the same simulations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MixOutcome {
    /// The mix's name.
    pub name: String,
    /// The weights by asset at the start.
    pub weights: Vec<(String, f64)>,
    /// Whether the weights change along a glide path.
    pub glide_path: bool,
    /// The outcome.
    pub outcome: Outcome,
    /// The value at the end across the simulations.
    pub end: Percentiles,
    /// For payments, the month by which 1 simulation in 10 has run short.
    pub runs_out_weak: Option<Month>,
    /// The risk of the mix.
    pub risk: MixRisk,
    /// The mix when markets do worse.
    pub what_if: WhatIf,
}

/// What a plan's simulations show: the chance of the goal, the value over time, what would
/// change the outcome, the plan when markets do worse, and the alternatives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PlanResults {
    /// The first month.
    pub start: Month,
    /// The month after the last one simulated: a target's date, or the month after the last
    /// payment or the end of a projection.
    pub end: Month,
    /// The number of simulations.
    pub simulations: usize,
    /// The chance wanted.
    pub chance_wanted: f64,
    /// The chance from which there is room to spare.
    pub spare_from: f64,
    /// The status; absent without a goal.
    pub status: Option<Status>,
    /// The plan's outcome.
    pub outcome: Outcome,
    /// The months recorded: every month, or every few months for a long plan.
    pub months: Vec<Month>,
    /// The value at the start of each recorded month across the simulations.
    pub values: Vec<Percentiles>,
    /// The value at the end across the simulations.
    pub end_value: Percentiles,
    /// For a target, the typical amount short among the simulations that miss it.
    pub shortfall: Option<f64>,
    /// For payments, how many are made and when the money runs short.
    pub payments: Option<PaymentsOutcome>,
    /// For a target, the chance of having the amount by each month from the start to the
    /// target's date, saving as planned.
    #[serde(default)]
    pub chance_by_month: Vec<f64>,
    /// 100 simulations spread evenly from the best outcome to the worst, so that the share of
    /// them that meets the goal is the chance.
    pub sample: Vec<SamplePath>,
    /// The flows as simulated, in the order given.
    pub flows: Vec<FlowSummary>,
    /// What would change the outcome; absent without a goal.
    pub changes: Option<Changes>,
    /// The plan when markets do worse.
    pub what_if: WhatIf,
    /// The risk of the plan's mix.
    pub risk: MixRisk,
    /// The plan's mix at the start, by asset.
    pub weights: Vec<(String, f64)>,
    /// The alternatives, in the order given.
    pub alternatives: Vec<MixOutcome>,
}

/// A whole number with thousands separators, such as 1,234,567.
fn grouped(x: f64) -> String {
    let text = format!("{:.0}", x.abs());
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if x < -0.5 { format!("-{out}") } else { out }
}

fn share(x: f64) -> String {
    format!("{:.1}%", 100.0 * x)
}

impl std::fmt::Display for PlanResults {
    /// The results in brief, 80 columns wide: the chance, the value at the end, the changes,
    /// the tests when markets do worse and the alternatives. The report describes them in full.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "PlanResults · {} to {} · {} simulations",
            self.start,
            self.end.plus(-1),
            grouped(self.simulations as f64)
        )?;
        writeln!(f)?;
        if let (Some(chance), Some(status)) = (self.outcome.chance, self.status) {
            writeln!(
                f,
                "  Chance {} (wanted {}, room to spare from {}): {}",
                share(chance),
                share(self.chance_wanted),
                share(self.spare_from),
                status.label()
            )?;
        }
        let e = &self.end_value;
        writeln!(
            f,
            "  Value at the end   p10 {:>12}  p50 {:>12}  p90 {:>12}",
            grouped(e.p10),
            grouped(e.p50),
            grouped(e.p90)
        )?;
        if let Some(s) = self.shortfall {
            writeln!(f, "  Typical shortfall  {:>16}", grouped(s))?;
        }
        if let Some(p) = &self.payments {
            let month = |m: Option<Month>| m.map_or_else(|| "never".to_owned(), |m| m.to_string());
            writeln!(
                f,
                "  Payments made {}; runs out: weak {}, average {}",
                share(p.paid),
                month(p.runs_out_weak),
                month(p.runs_out_average)
            )?;
        }
        if let Some(c) = &self.changes {
            writeln!(f)?;
            writeln!(
                f,
                "  Changes that {} the chance wanted",
                if c.to_reach { "reach" } else { "keep" }
            )?;
            for a in [&c.saving, &c.start_value, &c.amount].into_iter().flatten() {
                writeln!(
                    f,
                    "    {:<24}{:>14} -> {:>14}",
                    a.what,
                    grouped(a.from),
                    grouped(a.to)
                )?;
            }
            if let Some(d) = &c.date {
                writeln!(
                    f,
                    "    {:<24}{:>14} -> {:>14}",
                    d.what,
                    d.from.to_string(),
                    d.to.to_string()
                )?;
            }
        }
        writeln!(f)?;
        let chance = |o: &Outcome| o.chance.map_or_else(|| "-".to_owned(), share);
        writeln!(f, "  {:<40}{:>10}{:>16}", "", "Chance", "Median end")?;
        let mut rows = vec![
            ("As it stands", self.outcome),
            ("Returns 1% a year lower", self.what_if.lower_returns),
        ];
        if let Some(c) = self.what_if.crash {
            rows.push(("The worst 12 months of the history", c));
        }
        for (name, o) in rows {
            writeln!(
                f,
                "  {name:<40}{:>10}{:>16}",
                chance(&o),
                grouped(o.typical)
            )?;
        }
        for a in &self.alternatives {
            let name: String = a.name.chars().take(38).collect();
            writeln!(
                f,
                "  {:<40}{:>10}{:>16}",
                format!("Mix: {name}"),
                chance(&a.outcome),
                grouped(a.outcome.typical)
            )?;
        }
        Ok(())
    }
}
