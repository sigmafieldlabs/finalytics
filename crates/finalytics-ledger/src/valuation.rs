//! Valuing the books with market data.

use finalytics_core::{Currency, Date, Decimal, Error, Observation, Result, ValidationReport};
use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, HashMap};

use crate::{
    AccountId, Allocation, Books, Effect, HoldingKey, Holdings, InstrumentId, LotId, MarketData,
};

/// A position valued on a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

/// One account's values and external flows on each date of an [`AccountSeries`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountLine {
    /// The account.
    pub account: AccountId,
    /// The account's value at the end of each date, in the base currency.
    pub values: Vec<Decimal>,
    /// The external flows into the account since the previous date (deposits and transfers in
    /// positive; withdrawals and transfers out negative), each converted at its own date's rate.
    pub flows: Vec<Decimal>,
}

/// Each account's values and external flows on a set of dates: the input to performance by
/// account and by group of accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountSeries {
    /// The dates.
    pub dates: Vec<Date>,
    /// One line per account, in the portfolio's order.
    pub lines: Vec<AccountLine>,
    /// Prices and rates carried forward from earlier dates.
    pub warnings: ValidationReport,
}

impl AccountSeries {
    /// The value series of the accounts for which `include` is true: their values and flows
    /// added up. Value moved between two included accounts by a withdrawal from one and a
    /// deposit into the other nets to zero.
    pub fn combined(&self, mut include: impl FnMut(&AccountId) -> bool) -> ValueSeries {
        let n = self.dates.len();
        let mut values = vec![Decimal::ZERO; n];
        let mut flows = vec![Decimal::ZERO; n];
        for line in self.lines.iter().filter(|l| include(&l.account)) {
            for t in 0..n {
                values[t] += line.values[t];
                flows[t] += line.flows[t];
            }
        }
        ValueSeries {
            dates: self.dates.clone(),
            values,
            flows,
            warnings: self.warnings.clone(),
        }
    }
}

impl Books {
    /// Each account's value at the end of each of `dates` (strictly increasing), with its
    /// external flows in between, in one pass over the transactions. Added up over every
    /// account, the lines give [`Books::value_series`].
    pub fn account_series(&self, dates: &[Date], market: &MarketData) -> Result<AccountSeries> {
        if let Some(pair) = dates.windows(2).find(|p| p[1] <= p[0]) {
            return Err(Error::UnorderedDates {
                previous: pair[0],
                next: pair[1],
            });
        }
        let base = self.base_currency();
        let mut look = Lookup {
            market,
            problems: ValidationReport::new(),
        };
        let index: HashMap<&AccountId, usize> = self
            .accounts()
            .iter()
            .enumerate()
            .map(|(k, a)| (&a.id, k))
            .collect();
        let mut totals: Vec<Totals> = self.accounts().iter().map(|_| Totals::default()).collect();
        let mut lines: Vec<AccountLine> = self
            .accounts()
            .iter()
            .map(|a| AccountLine {
                account: a.id.clone(),
                values: vec![Decimal::ZERO; dates.len()],
                flows: vec![Decimal::ZERO; dates.len()],
            })
            .collect();
        let mut events = self.events().iter().peekable();
        for (t, &date) in dates.iter().enumerate() {
            while let Some(event) = events.next_if(|e| e.date <= date) {
                for effect in &event.effects {
                    let account = match effect {
                        Effect::SetLot { lot } => &lot.account,
                        Effect::CloseLot { account, .. } | Effect::Cash { account, .. } => account,
                    };
                    if let Some(&k) = index.get(account) {
                        totals[k].apply_all(std::slice::from_ref(effect));
                    }
                }
                for f in &event.flows {
                    let Some(&k) = index.get(&f.account) else {
                        continue;
                    };
                    match market.fx_rate(f.currency, base, event.date).value() {
                        Some(rate) => lines[k].flows[t] += f.amount * rate,
                        None => look.problems.error(
                            format!("fx.{}{base}", f.currency),
                            format!(
                                "no rate on or shortly before {} for a flow of {} {}",
                                event.date, f.amount, f.currency
                            ),
                        ),
                    }
                }
            }
            let mut rates = Rates::new(base, date);
            let mut prices: HashMap<&InstrumentId, Decimal> = HashMap::new();
            for (k, account) in totals.iter().enumerate() {
                let mut total = Decimal::ZERO;
                for (id, quantity) in &account.quantities {
                    let Some(instrument) = self.instrument(id) else {
                        continue;
                    };
                    let price = *prices.entry(id).or_insert_with(|| look.price(id, date));
                    let local = *quantity * price * instrument.multiplier;
                    total += local * rates.get(instrument.currency, &mut look);
                }
                for (currency, amount) in &account.cash {
                    if !amount.is_zero() {
                        total += *amount * rates.get(*currency, &mut look);
                    }
                }
                lines[k].values[t] = total;
            }
        }
        if look.problems.has_errors() {
            return Err(Error::Validation(look.problems));
        }
        Ok(AccountSeries {
            dates: dates.to_vec(),
            lines,
            warnings: look.problems,
        })
    }

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
        let base = self.base_currency();
        let mut look = Lookup {
            market,
            problems: ValidationReport::new(),
        };
        let mut totals = Totals::default();
        let mut values = Vec::with_capacity(dates.len());
        let mut flows = Vec::with_capacity(dates.len());
        let mut events = self.events().iter().peekable();
        for &date in dates {
            let mut flow = Decimal::ZERO;
            while let Some(event) = events.next_if(|e| e.date <= date) {
                totals.apply_all(&event.effects);
                for f in &event.flows {
                    match market.fx_rate(f.currency, base, event.date).value() {
                        Some(rate) => flow += f.amount * rate,
                        None => look.problems.error(
                            format!("fx.{}{base}", f.currency),
                            format!(
                                "no rate on or shortly before {} for a flow of {} {}",
                                event.date, f.amount, f.currency
                            ),
                        ),
                    }
                }
            }
            let mut rates = Rates::new(base, date);
            let mut total = Decimal::ZERO;
            for (id, quantity) in &totals.quantities {
                let Some(instrument) = self.instrument(id) else {
                    continue;
                };
                let local = *quantity * look.price(id, date) * instrument.multiplier;
                total += local * rates.get(instrument.currency, &mut look);
            }
            for (currency, amount) in &totals.cash {
                if !amount.is_zero() {
                    total += *amount * rates.get(*currency, &mut look);
                }
            }
            values.push(total);
            flows.push(flow);
        }
        if look.problems.has_errors() {
            return Err(Error::Validation(look.problems));
        }
        Ok(ValueSeries {
            dates: dates.to_vec(),
            values,
            flows,
            warnings: look.problems,
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

/// Values, value moved in and income per holding over dates, in the holding's currency and in
/// the base currency: the input to contribution analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingSeries {
    /// The dates.
    pub dates: Vec<Date>,
    /// The base currency.
    pub base_currency: Currency,
    /// One line per instrument and per cash currency held at any time, in key order.
    pub lines: Vec<HoldingLine>,
    /// Prices and rates carried forward from earlier dates.
    pub warnings: ValidationReport,
}

/// One holding's values, trades and income on each date of a [`HoldingSeries`].
///
/// Trades and income are those since the previous date. Value moved between holdings (a
/// purchase moves cash into a share) appears as a trade out of one line and into another, so
/// trades across all lines add up to the portfolio's external flows plus its income.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingLine {
    /// The holding.
    pub holding: HoldingKey,
    /// The holding's own currency: the instrument's, or the cash currency.
    pub currency: Currency,
    /// The value at the end of each date, in `currency`.
    pub values_local: Vec<Decimal>,
    /// The value at the end of each date, in the base currency.
    pub values_base: Vec<Decimal>,
    /// The value moved in since the previous date, in `currency`.
    pub trades_local: Vec<Decimal>,
    /// The value moved in since the previous date, in the base currency.
    pub trades_base: Vec<Decimal>,
    /// The income earned since the previous date, in `currency`.
    pub income_local: Vec<Decimal>,
    /// The income earned since the previous date, in the base currency.
    pub income_base: Vec<Decimal>,
}

impl HoldingLine {
    fn new(holding: HoldingKey, currency: Currency, len: usize) -> HoldingLine {
        let zeros = vec![Decimal::ZERO; len];
        HoldingLine {
            holding,
            currency,
            values_local: zeros.clone(),
            values_base: zeros.clone(),
            trades_local: zeros.clone(),
            trades_base: zeros.clone(),
            income_local: zeros.clone(),
            income_base: zeros,
        }
    }
}

/// Quantities by instrument and cash by currency, added up across accounts, kept up to date from
/// the books' effects, so that a series of valuations needs one pass and one price lookup per
/// instrument and date.
#[derive(Default)]
struct Totals {
    /// The quantity of each open lot, to know what a change to it replaces.
    lots: HashMap<(AccountId, InstrumentId, LotId), Decimal>,
    quantities: BTreeMap<InstrumentId, Decimal>,
    cash: BTreeMap<Currency, Decimal>,
}

impl Totals {
    fn apply_all(&mut self, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::SetLot { lot } => {
                    let key = (lot.account.clone(), lot.instrument.clone(), lot.id.clone());
                    let before = self.lots.insert(key, lot.quantity).unwrap_or_default();
                    self.add(&lot.instrument, lot.quantity - before);
                }
                Effect::CloseLot {
                    account,
                    instrument,
                    lot,
                } => {
                    let key = (account.clone(), instrument.clone(), lot.clone());
                    if let Some(before) = self.lots.remove(&key) {
                        self.add(instrument, -before);
                    }
                }
                Effect::Cash {
                    currency, amount, ..
                } => *self.cash.entry(*currency).or_default() += *amount,
            }
        }
    }

    fn add(&mut self, instrument: &InstrumentId, change: Decimal) {
        let quantity = self.quantities.entry(instrument.clone()).or_default();
        *quantity += change;
        if quantity.is_zero() {
            self.quantities.remove(instrument);
        }
    }
}

/// FX rates into the base currency on one date, each looked up once.
struct Rates {
    base: Currency,
    date: Date,
    found: Vec<(Currency, Decimal)>,
}

impl Rates {
    fn new(base: Currency, date: Date) -> Rates {
        Rates {
            base,
            date,
            found: Vec::new(),
        }
    }

    fn get(&mut self, currency: Currency, look: &mut Lookup<'_>) -> Decimal {
        if let Some((_, rate)) = self.found.iter().find(|(c, _)| *c == currency) {
            return *rate;
        }
        let rate = look.fx(currency, self.base, self.date);
        self.found.push((currency, rate));
        rate
    }
}

struct Lookup<'a> {
    market: &'a MarketData,
    problems: ValidationReport,
}

impl Lookup<'_> {
    fn fx(&mut self, from: Currency, to: Currency, date: Date) -> Decimal {
        match self.market.fx_rate(from, to, date) {
            Observation::Observed(rate) => rate,
            Observation::Carried { value, from: since } => {
                self.problems.warning(
                    format!("fx.{from}{to}"),
                    format!("rate for {date} carried from {since}"),
                );
                value
            }
            Observation::Missing => {
                self.problems.error(
                    format!("fx.{from}{to}"),
                    format!("no rate on or shortly before {date}"),
                );
                Decimal::ZERO
            }
        }
    }

    fn price(&mut self, instrument: &InstrumentId, date: Date) -> Decimal {
        match self.market.price(instrument, date) {
            Observation::Observed(price) => price,
            Observation::Carried { value, from } => {
                self.problems.warning(
                    format!("prices.{instrument}"),
                    format!("price for {date} carried from {from}"),
                );
                value
            }
            Observation::Missing => {
                self.problems.error(
                    format!("prices.{instrument}"),
                    format!("no price on or shortly before {date}"),
                );
                Decimal::ZERO
            }
        }
    }
}

impl Books {
    /// Each holding's values, trades and income on `dates` (strictly increasing), in one pass
    /// over the transactions.
    pub fn holding_series(&self, dates: &[Date], market: &MarketData) -> Result<HoldingSeries> {
        if let Some(pair) = dates.windows(2).find(|p| p[1] <= p[0]) {
            return Err(Error::UnorderedDates {
                previous: pair[0],
                next: pair[1],
            });
        }
        let base = self.base_currency();
        let mut look = Lookup {
            market,
            problems: ValidationReport::new(),
        };
        let mut lines: BTreeMap<HoldingKey, HoldingLine> = BTreeMap::new();
        let mut totals = Totals::default();
        let mut events = self.events().iter().peekable();
        let currency_of = |key: &HoldingKey| match key {
            HoldingKey::Cash(c) => *c,
            HoldingKey::Instrument(id) => self.instrument(id).map_or(base, |i| i.currency),
        };
        for (i, &date) in dates.iter().enumerate() {
            while let Some(event) = events.next_if(|e| e.date <= date) {
                totals.apply_all(&event.effects);
                for allocation in &event.allocations {
                    let mut add = |key: HoldingKey,
                                   currency: Currency,
                                   amount: Decimal,
                                   income: bool,
                                   look: &mut Lookup<'_>| {
                        let line_currency = currency_of(&key);
                        let local = amount * look.fx(currency, line_currency, event.date);
                        let in_base = amount * look.fx(currency, base, event.date);
                        let line = lines
                            .entry(key.clone())
                            .or_insert_with(|| HoldingLine::new(key, line_currency, dates.len()));
                        if income {
                            line.income_local[i] += local;
                            line.income_base[i] += in_base;
                        } else {
                            line.trades_local[i] += local;
                            line.trades_base[i] += in_base;
                        }
                    };
                    match allocation {
                        Allocation::Trade {
                            holding,
                            currency,
                            amount,
                        } => add(holding.clone(), *currency, *amount, false, &mut look),
                        Allocation::Income {
                            holding,
                            currency,
                            amount,
                        } => add(holding.clone(), *currency, *amount, true, &mut look),
                        Allocation::Exchange { from, to, quantity } => {
                            let Some(instrument) = self.instrument(to) else {
                                continue;
                            };
                            let value =
                                *quantity * look.price(to, event.date) * instrument.multiplier;
                            add(
                                HoldingKey::Instrument(to.clone()),
                                instrument.currency,
                                value,
                                false,
                                &mut look,
                            );
                            add(
                                HoldingKey::Instrument(from.clone()),
                                instrument.currency,
                                -value,
                                false,
                                &mut look,
                            );
                        }
                    }
                }
            }
            let mut rates = Rates::new(base, date);
            for (id, quantity) in &totals.quantities {
                let Some(instrument) = self.instrument(id) else {
                    continue;
                };
                let key = HoldingKey::Instrument(id.clone());
                let local = *quantity * look.price(id, date) * instrument.multiplier;
                let in_base = local * rates.get(instrument.currency, &mut look);
                let line = lines
                    .entry(key.clone())
                    .or_insert_with(|| HoldingLine::new(key, instrument.currency, dates.len()));
                line.values_local[i] += local;
                line.values_base[i] += in_base;
            }
            for (&currency, &amount) in &totals.cash {
                let key = HoldingKey::Cash(currency);
                let in_base = if amount.is_zero() {
                    Decimal::ZERO
                } else {
                    amount * rates.get(currency, &mut look)
                };
                let line = lines
                    .entry(key.clone())
                    .or_insert_with(|| HoldingLine::new(key, currency, dates.len()));
                line.values_local[i] += amount;
                line.values_base[i] += in_base;
            }
        }
        if look.problems.has_errors() {
            return Err(Error::Validation(look.problems));
        }
        Ok(HoldingSeries {
            dates: dates.to_vec(),
            base_currency: base,
            lines: lines.into_values().collect(),
            warnings: look.problems,
        })
    }
}
