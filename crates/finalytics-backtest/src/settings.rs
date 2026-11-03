//! How a backtest is run: when the portfolio is rebalanced, how strategies are validated, and
//! what trading costs.

use serde::{Deserialize, Serialize};

use finalytics_core::Date;

/// When the portfolio trades back to its target weights. Between trades the weights drift with
/// prices.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "RebalanceSchedule"))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Rebalance {
    /// At the end of each month.
    Monthly,
    /// At the end of each quarter (the default).
    #[default]
    Quarterly,
    /// At the end of each year.
    Yearly,
    /// At the end of any period in which a weight is more than `band` away from its target.
    Drift {
        /// The largest difference allowed between a weight and its target, such as 0.05 for
        /// five percentage points.
        band: f64,
    },
    /// Never: the portfolio buys its target weights once and holds them (buy and hold).
    Never,
}

impl Rebalance {
    /// When a weight drifts more than `band` from its target, such as 0.05 for five
    /// percentage points.
    pub fn drift(band: f64) -> Rebalance {
        Rebalance::Drift { band }
    }

    /// Whether the schedule follows the calendar: monthly, quarterly or yearly.
    pub fn is_calendar(&self) -> bool {
        matches!(
            self,
            Rebalance::Monthly | Rebalance::Quarterly | Rebalance::Yearly
        )
    }

    /// Whether `date` is the last date of its month, quarter or year, given the date after it.
    pub(crate) fn ends_period(&self, date: Date, next: Date) -> bool {
        let (y0, m0, _) = date.ymd();
        let (y1, m1, _) = next.ymd();
        match self {
            Rebalance::Monthly => (y0, m0) != (y1, m1),
            Rebalance::Quarterly => (y0, (m0 - 1) / 3) != (y1, (m1 - 1) / 3),
            Rebalance::Yearly => y0 != y1,
            Rebalance::Drift { .. } | Rebalance::Never => false,
        }
    }
}

/// How a strategy's weights are tested: on the history they were fitted to, or on later
/// history they had not seen.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Validation {
    /// Weights fitted once to the whole history and tested on it: what the strategy would have
    /// done with perfect knowledge of the period. The default.
    #[default]
    InSample,
    /// Weights fitted once to the history before `start`, and tested on the history from
    /// `start` on.
    Holdout {
        /// The first date of the test.
        start: Date,
    },
    /// Weights refitted at each calendar rebalance on the window of history before it, and
    /// held until the next: every return in the test comes after the weights were chosen. The
    /// window is given in years or in periods.
    WalkForward {
        /// The years of history each refit uses, such as 5.
        #[serde(default)]
        years: Option<f64>,
        /// The periods of history each refit uses, such as 60 months, in place of `years`.
        #[serde(default)]
        periods: Option<usize>,
        /// Whether each refit uses all the history so far (`true`) rather than only the last
        /// window (`false`, the default). The first fit still waits for a full window.
        #[serde(default)]
        expanding: bool,
    },
}

impl Validation {
    /// Walk-forward validation, refitting on the last `years` of history.
    pub fn walk_forward(years: f64) -> Validation {
        Validation::WalkForward {
            years: Some(years),
            periods: None,
            expanding: false,
        }
    }

    /// Walk-forward validation, refitting on the last `periods` periods of history.
    pub fn walk_forward_periods(periods: usize) -> Validation {
        Validation::WalkForward {
            years: None,
            periods: Some(periods),
            expanding: false,
        }
    }

    /// The same walk-forward validation, refitting on all the history so far.
    pub fn expanding(self) -> Validation {
        match self {
            Validation::WalkForward { years, periods, .. } => Validation::WalkForward {
                years,
                periods,
                expanding: true,
            },
            other => other,
        }
    }
}

/// How a backtest is run.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct Settings {
    /// How the strategies are validated: in sample by default.
    pub validation: Validation,
    /// When the portfolio trades back to its targets: quarterly by default.
    pub rebalance: Rebalance,
    /// The cost of trading, in basis points of the value traded: 10 is 0.1%. Zero by default.
    pub trading_cost_bps: f64,
}

impl Settings {
    /// Settings with `validation`, quarterly rebalancing and no trading costs.
    pub fn new(validation: Validation) -> Settings {
        Settings {
            validation,
            ..Settings::default()
        }
    }

    /// The same settings with another rebalancing schedule.
    pub fn with_rebalance(mut self, rebalance: Rebalance) -> Settings {
        self.rebalance = rebalance;
        self
    }

    /// The same settings with trading costs, in basis points of the value traded.
    pub fn with_trading_cost_bps(mut self, bps: f64) -> Settings {
        self.trading_cost_bps = bps;
        self
    }
}
