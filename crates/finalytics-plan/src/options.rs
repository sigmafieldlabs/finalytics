//! What a plan simulates: the goal, the money paid in and out, what the money is invested in,
//! and how markets are simulated.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::Currency;
use finalytics_risk::PriceTable;

use crate::Month;

/// How often a regular amount falls due.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Frequency {
    /// Every month (the default).
    #[default]
    Month,
    /// Every three months.
    Quarter,
    /// Every twelve months.
    Year,
}

impl Frequency {
    /// The number of months between payments.
    pub fn months(self) -> usize {
        match self {
            Frequency::Month => 1,
            Frequency::Quarter => 3,
            Frequency::Year => 12,
        }
    }

    /// The frequency as a word, such as "month".
    pub fn noun(self) -> &'static str {
        match self {
            Frequency::Month => "month",
            Frequency::Quarter => "quarter",
            Frequency::Year => "year",
        }
    }
}

/// The goal a plan is tested against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Goal {
    /// An amount by a date, such as a house deposit, the money to start a business, or a
    /// savings milestone. Met when the plan is worth at least `amount` at the start of the month
    /// `by`.
    Target {
        /// The goal's name, such as "House deposit".
        #[serde(default)]
        name: Option<String>,
        /// The amount, in today's money.
        amount: f64,
        /// The month it is needed by.
        by: Month,
    },
    /// An amount paid regularly from `start` to `end`, such as college fees or retirement
    /// income. Met when every payment is made in full.
    Payments {
        /// The goal's name, such as "Retirement income".
        #[serde(default)]
        name: Option<String>,
        /// Each payment, in today's money.
        amount: f64,
        /// How often it is paid: every month by default.
        #[serde(default)]
        every: Frequency,
        /// The month of the first payment.
        start: Month,
        /// The month of the last payment, or of the last month the payments run to.
        end: Month,
    },
}

impl Goal {
    /// The goal's name, or a name for its kind.
    pub fn name(&self) -> &str {
        match self {
            Goal::Target { name, .. } => name.as_deref().unwrap_or("Target"),
            Goal::Payments { name, .. } => name.as_deref().unwrap_or("Payments"),
        }
    }
}

/// Money paid into the plan, or taken out of it, other than the goal: once, in the month `on`,
/// or regularly, `every` month, quarter or year from `start` to `end`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "PlanFlow"))]
#[non_exhaustive]
pub enum Flow {
    /// Money paid in.
    Saving {
        /// A name for it in the report, such as "Monthly saving".
        #[serde(default)]
        name: Option<String>,
        /// The amount, in today's money.
        amount: f64,
        /// The month of a single payment.
        #[serde(default)]
        on: Option<Month>,
        /// How often a regular payment is made.
        #[serde(default)]
        every: Option<Frequency>,
        /// The first month of a regular payment: the plan's start by default.
        #[serde(default)]
        start: Option<Month>,
        /// The last month of a regular payment: by default, the month before a target's date or
        /// before the first goal payment, or the end of a plan without a goal.
        #[serde(default)]
        end: Option<Month>,
        /// How the amount grows a year in money of the day: with inflation by default, so that
        /// it stays the same in today's money; 0 keeps it fixed in money of the day.
        #[serde(default)]
        growth: Option<f64>,
    },
    /// Money taken out, as planned and not scored, such as a car bought along the way. When the
    /// plan cannot cover it, what is left is paid.
    Spending {
        /// A name for it in the report, such as "Car".
        #[serde(default)]
        name: Option<String>,
        /// The amount, in today's money.
        amount: f64,
        /// The month of a single payment.
        #[serde(default)]
        on: Option<Month>,
        /// How often a regular payment is made.
        #[serde(default)]
        every: Option<Frequency>,
        /// The first month of a regular payment: the plan's start by default.
        #[serde(default)]
        start: Option<Month>,
        /// The last month of a regular payment: by default, the plan's last month (the month
        /// before a target's date, the last goal payment, or the end of a projection).
        #[serde(default)]
        end: Option<Month>,
        /// How the amount grows a year in money of the day: with inflation by default.
        #[serde(default)]
        growth: Option<f64>,
    },
}

/// The parts of a flow, whichever way the money goes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FlowParts<'a> {
    pub(crate) saving: bool,
    pub(crate) name: Option<&'a str>,
    pub(crate) amount: f64,
    pub(crate) on: Option<Month>,
    pub(crate) every: Option<Frequency>,
    pub(crate) start: Option<Month>,
    pub(crate) end: Option<Month>,
    pub(crate) growth: Option<f64>,
}

impl Flow {
    pub(crate) fn parts(&self) -> FlowParts<'_> {
        match self {
            Flow::Saving {
                name,
                amount,
                on,
                every,
                start,
                end,
                growth,
            } => FlowParts {
                saving: true,
                name: name.as_deref(),
                amount: *amount,
                on: *on,
                every: *every,
                start: *start,
                end: *end,
                growth: *growth,
            },
            Flow::Spending {
                name,
                amount,
                on,
                every,
                start,
                end,
                growth,
            } => FlowParts {
                saving: false,
                name: name.as_deref(),
                amount: *amount,
                on: *on,
                every: *every,
                start: *start,
                end: *end,
                growth: *growth,
            },
        }
    }

    /// Whether money is paid in.
    pub fn is_saving(&self) -> bool {
        matches!(self, Flow::Saving { .. })
    }

    /// The flow's name, or a name for what it is, such as "Monthly saving".
    pub fn name(&self) -> String {
        let p = self.parts();
        if let Some(name) = p.name {
            return name.to_owned();
        }
        match (p.saving, p.every) {
            (true, Some(Frequency::Month)) => "Monthly saving".to_owned(),
            (true, Some(Frequency::Quarter)) => "Quarterly saving".to_owned(),
            (true, Some(Frequency::Year)) => "Yearly saving".to_owned(),
            (true, None) => "Saving".to_owned(),
            (false, Some(Frequency::Month)) => "Monthly spending".to_owned(),
            (false, Some(Frequency::Quarter)) => "Quarterly spending".to_owned(),
            (false, Some(Frequency::Year)) => "Yearly spending".to_owned(),
            (false, None) => "Spending".to_owned(),
        }
    }

    /// The amount, in today's money.
    pub fn amount(&self) -> f64 {
        self.parts().amount
    }
}

/// What the money is invested in: fixed weights, or weights that change along a glide path.
/// The weights are restored every month.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Mix {
    /// The same weights throughout.
    Fixed {
        /// A name for the mix, such as "Balanced 60/40".
        #[serde(default)]
        name: Option<String>,
        /// The weights by asset, adding up to one.
        weights: BTreeMap<String, f64>,
    },
    /// Weights given at some months, moving in a straight line between them and held before the
    /// first and after the last.
    GlidePath {
        /// A name for the mix, such as "Glide path".
        #[serde(default)]
        name: Option<String>,
        /// The weights by asset at each month, each set adding up to one.
        points: BTreeMap<Month, BTreeMap<String, f64>>,
    },
}

impl Mix {
    /// The mix's name, if it has one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Mix::Fixed { name, .. } | Mix::GlidePath { name, .. } => name.as_deref(),
        }
    }

    /// The same mix under `name`.
    pub fn named(mut self, given: &str) -> Mix {
        match &mut self {
            Mix::Fixed { name, .. } | Mix::GlidePath { name, .. } => *name = Some(given.to_owned()),
        }
        self
    }

    /// Fixed `weights` by asset.
    pub fn fixed<S: Into<String>>(weights: impl IntoIterator<Item = (S, f64)>) -> Mix {
        Mix::Fixed {
            name: None,
            weights: weights.into_iter().map(|(a, w)| (a.into(), w)).collect(),
        }
    }
}

/// How prices rise, for amounts in today's money.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Inflation {
    /// A fixed rate a year, such as 0.025; 0 keeps every figure in money of the day.
    Rate {
        /// The rate a year.
        rate: f64,
    },
    /// A price index, such as the US consumer price index: one column of levels at least
    /// monthly over the assets' history. Each simulated month takes the inflation of the same
    /// past month as its returns.
    Index {
        /// The index's levels.
        prices: PriceTable,
    },
}

/// How the simulations draw each month's returns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Model {
    /// A stationary block bootstrap (Politis and Romano, 1994): runs of past months, all assets
    /// together, of random length averaging `block` months, wrapping round at the end of the
    /// history. The default, with blocks of 12 months.
    Bootstrap {
        /// The average length of a run, in months: 12 by default.
        #[serde(default = "twelve")]
        block: f64,
    },
    /// A normal distribution with the assets' expected returns and covariance, per month.
    Normal,
    /// Every stretch of the history as long as the plan, each once, in order: as many
    /// simulations as there are stretches.
    Historical,
}

fn twelve() -> f64 {
    12.0
}

impl Default for Model {
    fn default() -> Model {
        Model::Bootstrap { block: 12.0 }
    }
}

/// What a plan simulates and how.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct PlanOptions {
    /// The goal; without one, the plan is a projection to `end`.
    pub goal: Option<Goal>,
    /// The chance of meeting the goal wanted, such as 0.8.
    pub chance: f64,
    /// The chance from which the plan has room to spare: 10 points above `chance` by default,
    /// at most 0.99.
    pub spare: Option<f64>,
    /// The first month: the month after the assets' last date by default.
    pub start: Option<Month>,
    /// The last month of a projection without a goal.
    pub end: Option<Month>,
    /// What the money set aside is worth at the start, in today's money.
    pub start_value: f64,
    /// Money paid in and taken out, other than the goal.
    pub flows: Vec<Flow>,
    /// What the money is invested in.
    pub weights: Option<Mix>,
    /// Other mixes to compare on the same simulations, each named.
    pub alternatives: Vec<Mix>,
    /// The fee a year, as a share of the value, taken monthly.
    pub fee: f64,
    /// How prices rise: required, as a rate or an index.
    pub inflation: Option<Inflation>,
    /// How each month's returns are drawn: a block bootstrap of 12 months by default.
    pub model: Model,
    /// Expected returns a year by asset, in place of the history's: they shift the bootstrap's
    /// returns and set the normal model's means.
    pub expected_returns: Option<BTreeMap<String, f64>>,
    /// The number of simulations: 10,000 by default.
    pub simulations: usize,
    /// The seed of the random numbers: the same seed gives the same plan on every platform.
    pub seed: u64,
    /// The currency of the amounts, for the report.
    pub currency: Option<Currency>,
    /// The report's title: the goal's name by default.
    pub name: Option<String>,
}

impl Default for PlanOptions {
    fn default() -> PlanOptions {
        PlanOptions {
            goal: None,
            chance: 0.8,
            spare: None,
            start: None,
            end: None,
            start_value: 0.0,
            flows: Vec::new(),
            weights: None,
            alternatives: Vec::new(),
            fee: 0.0,
            inflation: None,
            model: Model::default(),
            expected_returns: None,
            simulations: 10_000,
            seed: 0,
            currency: None,
            name: None,
        }
    }
}

impl PlanOptions {
    /// The chance from which the plan has room to spare.
    pub fn spare_from(&self) -> f64 {
        self.spare.unwrap_or_else(|| (self.chance + 0.10).min(0.99))
    }
}
