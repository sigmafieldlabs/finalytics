//! Weights and returns of a portfolio's holdings, from its books and market data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Decimal, Error, Observation, Result, ValidationReport};
use finalytics_ledger::{Books, HoldingKey, MarketData, Valuation};

use crate::input::invalid;
use crate::matrix::ReturnsTable;

/// The weight of each holding in a valuation: each instrument and each cash currency, added up
/// across accounts, as a share of the total value in the base currency. Holdings are in
/// [`HoldingKey`] order.
pub fn holding_weights(valuation: &Valuation) -> Result<Vec<(HoldingKey, f64)>> {
    if valuation.total_base <= Decimal::ZERO {
        return Err(invalid(
            "total_base",
            "weights need a portfolio worth more than zero",
        ));
    }
    let mut values: BTreeMap<HoldingKey, Decimal> = BTreeMap::new();
    for p in &valuation.positions {
        *values
            .entry(HoldingKey::Instrument(p.instrument.clone()))
            .or_default() += p.market_value_base;
    }
    for c in &valuation.cash {
        *values.entry(HoldingKey::Cash(c.currency)).or_default() += c.value_base;
    }
    let total = to_f64(valuation.total_base);
    Ok(values
        .into_iter()
        .map(|(key, value)| (key, to_f64(value) / total))
        .collect())
}

/// Returns of holdings in the base currency between consecutive dates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingReturns {
    /// The holdings, in the order of the table's columns.
    pub holdings: Vec<HoldingKey>,
    /// The date each return ends on: every date given but the first.
    pub dates: Vec<Date>,
    /// The returns, one column per holding, labelled with the instrument identifier or, for
    /// cash, the currency code followed by " cash".
    pub table: ReturnsTable,
    /// Prices and rates carried forward from earlier dates.
    pub warnings: ValidationReport,
}

/// The returns of `holdings` in the base currency between consecutive `dates`, the input to
/// covariance estimation for risk decomposition.
///
/// An instrument's return is the change in its price converted to the base currency; supply
/// total-return prices to include income. Cash in a foreign currency returns the change in its
/// exchange rate, and cash in the base currency returns zero. Every missing price or rate is
/// reported at once.
pub fn holding_returns(
    books: &Books,
    holdings: &[HoldingKey],
    dates: &[Date],
    market: &MarketData,
) -> Result<HoldingReturns> {
    if let Some(pair) = dates.windows(2).find(|p| p[1] <= p[0]) {
        return Err(Error::UnorderedDates {
            previous: pair[0],
            next: pair[1],
        });
    }
    if dates.len() < 3 {
        return Err(invalid(
            "dates",
            "needs at least three dates, for two returns",
        ));
    }
    let base = books.base_currency();
    let mut problems = ValidationReport::new();
    let mut warnings = ValidationReport::new();
    let mut labels = Vec::with_capacity(holdings.len());
    let mut columns = Vec::with_capacity(holdings.len());
    for (k, holding) in holdings.iter().enumerate() {
        let (currency, label) = match holding {
            HoldingKey::Instrument(id) => match books.instrument(id) {
                Some(instrument) => (instrument.currency, id.to_string()),
                None => {
                    problems.error(format!("holdings[{k}]"), format!("unknown instrument {id}"));
                    continue;
                }
            },
            HoldingKey::Cash(currency) => (*currency, format!("{currency} cash")),
        };
        let found_before = problems.issues().len();
        let mut levels = Vec::with_capacity(dates.len());
        for &date in dates {
            let mut level = observe(
                market.fx_rate(currency, base, date),
                &format!("fx.{currency}{base}"),
                date,
                &mut problems,
                &mut warnings,
            );
            if let HoldingKey::Instrument(id) = holding {
                level *= observe(
                    market.price(id, date),
                    &format!("prices.{id}"),
                    date,
                    &mut problems,
                    &mut warnings,
                );
            }
            levels.push(level);
        }
        if problems.issues().len() > found_before {
            continue;
        }
        if levels.iter().any(|l| *l <= 0.0) {
            problems.error(
                format!("holdings[{k}]"),
                format!("{label} has a price or rate of zero, so its returns are undefined"),
            );
            continue;
        }
        labels.push(label);
        columns.push(levels.windows(2).map(|p| p[1] / p[0] - 1.0).collect());
    }
    if problems.has_errors() {
        return Err(Error::Validation(problems));
    }
    Ok(HoldingReturns {
        holdings: holdings.to_vec(),
        dates: dates[1..].to_vec(),
        table: ReturnsTable::new(labels, columns)?,
        warnings,
    })
}

pub(crate) fn observe(
    observation: Observation<Decimal>,
    path: &str,
    date: Date,
    problems: &mut ValidationReport,
    warnings: &mut ValidationReport,
) -> f64 {
    match observation {
        Observation::Observed(value) => to_f64(value),
        Observation::Carried { value, from } => {
            warnings.warning(path, format!("value for {date} carried from {from}"));
            to_f64(value)
        }
        Observation::Missing => {
            problems.error(path, format!("no value on or shortly before {date}"));
            0.0
        }
    }
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}
