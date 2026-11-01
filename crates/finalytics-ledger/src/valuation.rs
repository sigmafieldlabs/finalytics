//! Valuing the books with market data.

use finalytics_core::{Currency, Date, Decimal, Error, Observation, Result, ValidationReport};
use serde::{Deserialize, Serialize};

use crate::{AccountId, Books, Holdings, InstrumentId, MarketData};

/// A position valued on a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionValue {
    /// The account.
    pub account: AccountId,
    /// The instrument.
    pub instrument: InstrumentId,
    /// The quantity held.
    pub quantity: Decimal,
    /// The price used, per unit, in the instrument's currency.
    pub price: Decimal,
    /// The date the price was observed (earlier than the valuation date if carried forward).
    pub price_date: Date,
    /// `quantity × price × multiplier`, in the instrument's currency.
    pub market_value_local: Decimal,
    /// Units of base currency per unit of the instrument's currency.
    pub fx_rate: Decimal,
    /// The market value in the base currency.
    pub market_value_base: Decimal,
    /// The cost basis in the instrument's currency.
    pub cost_local: Decimal,
    /// The cost basis in the base currency.
    pub cost_base: Decimal,
    /// `market_value_local - cost_local`.
    pub unrealised_local: Decimal,
    /// `market_value_base - cost_base`, which includes the currency effect.
    pub unrealised_base: Decimal,
}

/// A cash balance valued on a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashValue {
    /// The account.
    pub account: AccountId,
    /// The currency.
    pub currency: Currency,
    /// The balance.
    pub amount: Decimal,
    /// Units of base currency per unit of `currency`.
    pub fx_rate: Decimal,
    /// The balance in the base currency.
    pub value_base: Decimal,
}

/// The holdings valued on a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Valuation {
    /// The valuation date.
    pub date: Date,
    /// The base currency.
    pub base_currency: Currency,
    /// The positions, by account and instrument.
    pub positions: Vec<PositionValue>,
    /// The cash balances, by account and currency.
    pub cash: Vec<CashValue>,
    /// Positions and cash, in the base currency.
    pub total_base: Decimal,
    /// Prices and rates carried forward from earlier dates.
    pub warnings: ValidationReport,
}

/// Daily values and external flows in the base currency, the input to performance measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueSeries {
    /// The dates.
    pub dates: Vec<Date>,
    /// The value of the portfolio at the end of each date.
    pub values: Vec<Decimal>,
    /// The external flows since the previous date (deposits and transfers in positive;
    /// withdrawals and transfers out negative), each converted at its own date's rate.
    pub flows: Vec<Decimal>,
    /// Prices and rates carried forward from earlier dates.
    pub warnings: ValidationReport,
}

impl Books {
    /// The holdings at the end of `date`, valued with `market`.
    ///
    /// Every price and rate that is missing (beyond the carry-forward limit) is reported at once,
    /// as a validation error; carried values are listed in [`Valuation::warnings`].
    pub fn value_on(&self, date: Date, market: &MarketData) -> Result<Valuation> {
        let mut problems = ValidationReport::new();
        let mut valuation =
            self.value_holdings(&self.holdings_on(date), date, market, &mut problems);
        if problems.has_errors() {
            return Err(Error::Validation(problems));
        }
        valuation.warnings = problems;
        Ok(valuation)
    }

    /// The value of the portfolio at the end of each of `dates` (strictly increasing), with the
    /// external flows in between, in one pass over the transactions.
    pub fn value_series(&self, dates: &[Date], market: &MarketData) -> Result<ValueSeries> {
        if let Some(pair) = dates.windows(2).find(|p| p[1] <= p[0]) {
            return Err(Error::UnorderedDates {
                previous: pair[0],
                next: pair[1],
            });
        }
        let mut problems = ValidationReport::new();
        let mut holdings = Holdings::default();
        let mut values = Vec::with_capacity(dates.len());
        let mut flows = Vec::with_capacity(dates.len());
        let mut events = self.events().iter().peekable();
        for &date in dates {
            let mut flow = Decimal::ZERO;
            while let Some(event) = events.next_if(|e| e.date <= date) {
                holdings.apply_all(&event.effects);
                for f in &event.flows {
                    match market
                        .fx_rate(f.currency, self.base_currency(), event.date)
                        .value()
                    {
                        Some(rate) => flow += f.amount * rate,
                        None => problems.error(
                            format!("fx.{}{}", f.currency, self.base_currency()),
                            format!(
                                "no rate on or shortly before {} for a flow of {} {}",
                                event.date, f.amount, f.currency
                            ),
                        ),
                    }
                }
            }
            let valuation = self.value_holdings(&holdings, date, market, &mut problems);
            values.push(valuation.total_base);
            flows.push(flow);
        }
        if problems.has_errors() {
            return Err(Error::Validation(problems));
        }
        Ok(ValueSeries {
            dates: dates.to_vec(),
            values,
            flows,
            warnings: problems,
        })
    }

    fn value_holdings(
        &self,
        holdings: &Holdings,
        date: Date,
        market: &MarketData,
        problems: &mut ValidationReport,
    ) -> Valuation {
        let base = self.base_currency();
        let mut total = Decimal::ZERO;
        let rate_of = |currency: Currency, problems: &mut ValidationReport| -> Decimal {
            match market.fx_rate(currency, base, date) {
                Observation::Observed(rate) => rate,
                Observation::Carried { value, from } => {
                    problems.warning(
                        format!("fx.{currency}{base}"),
                        format!("rate for {date} carried from {from}"),
                    );
                    value
                }
                Observation::Missing => {
                    problems.error(
                        format!("fx.{currency}{base}"),
                        format!("no rate on or shortly before {date}"),
                    );
                    Decimal::ZERO
                }
            }
        };

        let mut positions = Vec::new();
        for position in holdings.positions() {
            let Some(instrument) = self.instrument(&position.instrument) else {
                continue;
            };
            let (price, price_date) = match market.price(&position.instrument, date) {
                Observation::Observed(price) => (price, date),
                Observation::Carried { value, from } => {
                    problems.warning(
                        format!("prices.{}", position.instrument),
                        format!("price for {date} carried from {from}"),
                    );
                    (value, from)
                }
                Observation::Missing => {
                    problems.error(
                        format!("prices.{}", position.instrument),
                        format!("no price on or shortly before {date}"),
                    );
                    (Decimal::ZERO, date)
                }
            };
            let fx_rate = rate_of(instrument.currency, problems);
            let market_value_local = position.quantity * price * instrument.multiplier;
            let market_value_base = market_value_local * fx_rate;
            total += market_value_base;
            positions.push(PositionValue {
                account: position.account,
                instrument: position.instrument,
                quantity: position.quantity,
                price,
                price_date,
                market_value_local,
                fx_rate,
                market_value_base,
                cost_local: position.cost_local,
                cost_base: position.cost_base,
                unrealised_local: market_value_local - position.cost_local,
                unrealised_base: market_value_base - position.cost_base,
            });
        }
        let mut cash = Vec::new();
        for (account, currency, amount) in holdings.cash() {
            let fx_rate = if amount.is_zero() {
                Decimal::ONE
            } else {
                rate_of(currency, problems)
            };
            let value_base = amount * fx_rate;
            total += value_base;
            cash.push(CashValue {
                account: account.clone(),
                currency,
                amount,
                fx_rate,
                value_base,
            });
        }
        Valuation {
            date,
            base_currency: base,
            positions,
            cash,
            total_base: total,
            warnings: ValidationReport::new(),
        }
    }
}
