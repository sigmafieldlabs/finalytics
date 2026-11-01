//! Contribution to return by holding and by group.

use std::collections::BTreeMap;

use finalytics_core::{Date, Decimal, Error, Result, ValidationReport};
use finalytics_ledger::{HoldingKey, HoldingSeries};
use serde::{Deserialize, Serialize};

use crate::{FlowTiming, Performance};

/// What one holding added to the portfolio's return over a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Contribution {
    /// The holding.
    pub holding: HoldingKey,
    /// Its contribution to the portfolio's time-weighted return. Contributions of all holdings
    /// add up to the portfolio's return over the period.
    pub contribution: f64,
    /// The holding's own return in the base currency, when it can be measured.
    pub return_base: Option<f64>,
    /// The holding's own return in its own currency, when it can be measured.
    pub return_local: Option<f64>,
    /// `return_base - return_local`: what currency movements added or took away.
    pub currency_effect: Option<f64>,
    /// The holding's average share of the portfolio's value at the end of each day.
    pub average_weight: f64,
}

/// What a group of holdings (an asset class, a region) added to the portfolio's return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GroupContribution {
    /// The group.
    pub group: String,
    /// The sum of its holdings' contributions.
    pub contribution: f64,
    /// The sum of its holdings' average weights.
    pub average_weight: f64,
}

fn f(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}

impl Performance {
    /// Each holding's contribution to the portfolio's return from the valuation on or before
    /// `start` to the one on or before `end`.
    ///
    /// Each day's contribution is the holding's gain (change in value, less value moved in, plus
    /// income) over the portfolio's starting value for the day. Days are linked by the
    /// portfolio's growth up to each day, so the contributions add up exactly to the period's
    /// time-weighted return. `holdings` must cover the same dates as this performance.
    pub fn contributions(
        &self,
        holdings: &HoldingSeries,
        start: Date,
        end: Date,
    ) -> Result<Vec<Contribution>> {
        if holdings.dates != self.dates {
            let mut report = ValidationReport::new();
            report.error(
                "holdings.dates",
                "must be the dates the performance was measured on",
            );
            return Err(Error::Validation(report));
        }
        let idx = |date: Date| self.dates.partition_point(|&d| d <= date).checked_sub(1);
        let (Some(i), Some(j)) = (idx(start), idx(end)) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(holdings.lines.len());
        for line in &holdings.lines {
            let value_base: Vec<f64> = line.values_base.iter().copied().map(f).collect();
            let value_local: Vec<f64> = line.values_local.iter().copied().map(f).collect();
            let gain = |values: &[f64], trades: &[Decimal], income: &[Decimal], t: usize| {
                values[t] - values[t - 1] - f(trades[t]) + f(income[t])
            };
            let mut contribution = 0.0;
            let mut growth_base = Some(1.0);
            let mut growth_local = Some(1.0);
            let mut weights = 0.0;
            let mut days = 0u32;
            for t in i + 1..=j {
                let gain_base = gain(&value_base, &line.trades_base, &line.income_base, t);
                let gain_local = gain(&value_local, &line.trades_local, &line.income_local, t);
                let denominator = match self.timing {
                    FlowTiming::StartOfDay => self.values[t - 1] + self.flows[t],
                    FlowTiming::EndOfDay => self.values[t - 1],
                };
                if denominator > 0.0 {
                    contribution += gain_base / denominator * self.growth[t - 1] / self.growth[i];
                }
                growth_base = link(
                    growth_base,
                    value_base[t - 1],
                    f(line.trades_base[t]),
                    gain_base,
                );
                growth_local = link(
                    growth_local,
                    value_local[t - 1],
                    f(line.trades_local[t]),
                    gain_local,
                );
                if self.values[t] > 0.0 {
                    weights += value_base[t] / self.values[t];
                    days += 1;
                }
            }
            let return_base = growth_base.map(|g| g - 1.0);
            let return_local = growth_local.map(|g| g - 1.0);
            out.push(Contribution {
                holding: line.holding.clone(),
                contribution,
                return_base,
                return_local,
                currency_effect: return_base.zip(return_local).map(|(b, l)| b - l),
                average_weight: if days == 0 {
                    0.0
                } else {
                    weights / f64::from(days)
                },
            });
        }
        Ok(out)
    }
}

/// Links one day into a holding's own return. The day's base is its starting value plus any
/// purchases (value moved out by a sale is not invested during the day).
fn link(growth: Option<f64>, start: f64, trades: f64, gain: f64) -> Option<f64> {
    let growth = growth?;
    let base = start + trades.max(0.0);
    if base > 0.0 {
        Some(growth * (1.0 + gain / base))
    } else if gain.abs() < 1e-9 {
        Some(growth)
    } else {
        None
    }
}

/// Adds up contributions by group, using `group_of` to name each holding's group (for example
/// its asset class). Groups are returned in name order.
pub fn group_contributions(
    contributions: &[Contribution],
    mut group_of: impl FnMut(&HoldingKey) -> String,
) -> Vec<GroupContribution> {
    let mut groups: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for c in contributions {
        let entry = groups.entry(group_of(&c.holding)).or_default();
        entry.0 += c.contribution;
        entry.1 += c.average_weight;
    }
    groups
        .into_iter()
        .map(
            |(group, (contribution, average_weight))| GroupContribution {
                group,
                contribution,
                average_weight,
            },
        )
        .collect()
}
