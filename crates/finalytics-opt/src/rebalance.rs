//! Trades that move an account from its current holdings to target weights.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{
    Currency, Date, Decimal, Error, Money, Observation, Result, Rounding, ValidationReport,
};
use finalytics_ledger::{
    AccountId, Books, HoldingKey, InstrumentId, LotMethod, LotPick, LotSale, MarketData,
    Transaction, TransactionId, TransactionKind,
};

/// Choices for [`rebalance`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct RebalanceOptions {
    /// How sales choose lots; first in, first out by default. Highest cost first defers gains.
    pub lot_method: LotMethod,
    /// Trade whole units only, rounding each trade towards zero.
    pub whole_units: bool,
    /// Leave out trades worth less than this in the base currency.
    pub minimum_trade: Decimal,
}

impl Default for RebalanceOptions {
    fn default() -> Self {
        RebalanceOptions {
            lot_method: LotMethod::Fifo,
            whole_units: false,
            minimum_trade: Decimal::ZERO,
        }
    }
}

/// Buy or sell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// A purchase.
    Buy,
    /// A sale.
    Sell,
}

/// A proposed trade in one instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposedTrade {
    /// The instrument.
    pub instrument: InstrumentId,
    /// Buy or sell.
    pub side: Side,
    /// The units to trade, greater than zero.
    pub quantity: Decimal,
    /// The price per unit, in the instrument's currency.
    pub price: Decimal,
    /// The instrument's currency.
    pub currency: Currency,
    /// `quantity × price × multiplier`, in the instrument's currency.
    pub value_local: Decimal,
    /// The value in the base currency.
    pub value_base: Decimal,
    /// For a sale, the lots it closes.
    pub lots: Vec<LotSale>,
    /// For a sale, the gain it would realise in the base currency, before fees.
    pub realised_gain_base: Option<Decimal>,
}

/// A proposed currency conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposedConversion {
    /// The amount sold.
    pub sell: Money,
    /// The amount bought.
    pub buy: Money,
}

/// The trades that move an account to target weights, and their effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Rebalance {
    /// The account.
    pub account: AccountId,
    /// The date of the prices used.
    pub date: Date,
    /// The account's value in the base currency.
    pub value_base: Decimal,
    /// Sales, then purchases.
    pub trades: Vec<ProposedTrade>,
    /// Conversions that fund purchases in other currencies and reach the target cash balances.
    pub conversions: Vec<ProposedConversion>,
    /// Each holding's weight before the trades.
    pub weights_before: Vec<(HoldingKey, f64)>,
    /// Each holding's weight after the trades.
    pub weights_after: Vec<(HoldingKey, f64)>,
    /// The value traded, sales and purchases, as a share of the account's value.
    pub turnover: f64,
    /// The gain the sales would realise in the base currency.
    pub realised_gain_base: Decimal,
    /// Carried prices, a shortfall of cash, and trades left out by rounding.
    pub warnings: ValidationReport,
}

/// Proposes the trades that move `account` to `targets`: a weight for each holding, as a share of
/// the account's value at `date`. Instruments held but not listed are sold; cash in the base
/// currency takes whatever weight the targets leave, and cash in other currencies is converted
/// to its target (zero if not listed). Only long positions are proposed.
///
/// Sales choose lots by [`RebalanceOptions::lot_method`], and [`Rebalance::transactions`]
/// returns the trades as transactions that name those lots, ready to apply to the books.
pub fn rebalance(
    books: &Books,
    market: &MarketData,
    date: Date,
    account: &AccountId,
    targets: &[(HoldingKey, f64)],
    options: &RebalanceOptions,
) -> Result<Rebalance> {
    let base = books.base_currency();
    let mut problems = ValidationReport::new();
    let mut warnings = ValidationReport::new();
    let total: f64 = targets.iter().map(|(_, w)| w).sum();
    let mut seen = std::collections::BTreeSet::new();
    for (k, (holding, weight)) in targets.iter().enumerate() {
        if !seen.insert(holding) {
            problems.error(format!("targets[{k}]"), "lists a holding a second time");
        }
        if !(weight.is_finite() && *weight >= 0.0) {
            problems.error(format!("targets[{k}]"), "must be a weight of zero or more");
        }
        if let HoldingKey::Instrument(id) = holding
            && books.instrument(id).is_none()
        {
            problems.error(format!("targets[{k}]"), format!("unknown instrument {id}"));
        }
    }
    if total > 1.0 + 1e-9 {
        problems.error(
            "targets",
            format!("the weights add up to {total}, more than 1"),
        );
    }
    if options.minimum_trade < Decimal::ZERO {
        problems.error("options.minimum_trade", "must not be negative");
    }
    if problems.has_errors() {
        return Err(Error::Validation(problems));
    }

    let holdings = books.holdings_on(date);
    let mut quantities: BTreeMap<InstrumentId, Decimal> = BTreeMap::new();
    for p in holdings
        .positions()
        .into_iter()
        .filter(|p| &p.account == account)
    {
        quantities.insert(p.instrument, p.quantity);
    }
    let mut cash: BTreeMap<Currency, Decimal> = holdings
        .cash()
        .filter(|(a, _, _)| *a == account)
        .map(|(_, c, amount)| (c, amount))
        .collect();
    cash.entry(base).or_default();

    let mut look = Look {
        market,
        date,
        problems: &mut problems,
        warnings: &mut warnings,
    };
    let mut rates: BTreeMap<Currency, Decimal> = BTreeMap::new();
    let mut units: BTreeMap<InstrumentId, Unit> = BTreeMap::new();
    let wanted: BTreeMap<&HoldingKey, f64> = targets.iter().map(|(k, w)| (k, *w)).collect();
    let instruments: Vec<InstrumentId> = quantities
        .keys()
        .cloned()
        .chain(targets.iter().filter_map(|(k, _)| match k {
            HoldingKey::Instrument(id) => Some(id.clone()),
            HoldingKey::Cash(_) => None,
        }))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for currency in cash
        .keys()
        .copied()
        .chain(targets.iter().filter_map(|(k, _)| match k {
            HoldingKey::Cash(c) => Some(*c),
            HoldingKey::Instrument(_) => None,
        }))
    {
        rates
            .entry(currency)
            .or_insert_with(|| look.fx(currency, base));
    }
    for id in &instruments {
        // Every instrument here is held or a validated target, so it is known to the books.
        let Some(instrument) = books.instrument(id) else {
            continue;
        };
        let rate = *rates
            .entry(instrument.currency)
            .or_insert_with(|| look.fx(instrument.currency, base));
        let price = look.price(id);
        units.insert(
            id.clone(),
            Unit {
                price,
                currency: instrument.currency,
                local: price * instrument.multiplier,
                base: price * instrument.multiplier * rate,
            },
        );
    }
    if problems.has_errors() {
        return Err(Error::Validation(problems));
    }

    let value_of = |quantities: &BTreeMap<InstrumentId, Decimal>,
                    cash: &BTreeMap<Currency, Decimal>| {
        let positions: Decimal = quantities.iter().map(|(id, q)| *q * units[id].base).sum();
        let balances: Decimal = cash.iter().map(|(c, a)| *a * rates[c]).sum();
        positions + balances
    };
    let value_base = value_of(&quantities, &cash);
    if value_base <= Decimal::ZERO {
        return Err(invalid(
            "account",
            format!("account {account} has no value to rebalance"),
        ));
    }
    let weights = |quantities: &BTreeMap<InstrumentId, Decimal>,
                   cash: &BTreeMap<Currency, Decimal>| {
        let total = to_f64(value_of(quantities, cash));
        let mut out: Vec<(HoldingKey, f64)> = quantities
            .iter()
            .filter(|(_, q)| !q.is_zero())
            .map(|(id, q)| {
                (
                    HoldingKey::Instrument(id.clone()),
                    to_f64(*q * units[id].base) / total,
                )
            })
            .chain(
                cash.iter()
                    .filter(|(_, a)| !a.is_zero())
                    .map(|(c, a)| (HoldingKey::Cash(*c), to_f64(*a * rates[c]) / total)),
            )
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    };
    let weights_before = weights(&quantities, &cash);

    let mut sales = Vec::new();
    let mut purchases = Vec::new();
    for id in &instruments {
        let unit = &units[id];
        if unit.base <= Decimal::ZERO {
            warnings.warning(
                format!("prices.{id}"),
                "has no positive price, so it is not traded",
            );
            continue;
        }
        let held = quantities.get(id).copied().unwrap_or_default();
        let weight = wanted
            .get(&HoldingKey::Instrument(id.clone()))
            .copied()
            .unwrap_or(0.0);
        let target = value_base * from_f64(weight);
        let mut quantity = ((target - held * unit.base) / unit.base).round_dp(10);
        if options.whole_units {
            quantity = quantity.trunc();
        }
        // Sell everything when the target is zero, whatever the rounding.
        if weight == 0.0 {
            quantity = -held;
        }
        let quantity = quantity.max(-held);
        if quantity.is_zero() {
            continue;
        }
        let value_base = (quantity.abs() * unit.base).round_dp(10);
        if value_base < options.minimum_trade {
            warnings.warning(
                format!("trades.{id}"),
                format!("a trade worth {value_base} is below the minimum and left out"),
            );
            continue;
        }
        let side = if quantity > Decimal::ZERO {
            Side::Buy
        } else {
            Side::Sell
        };
        let quantity = quantity.abs();
        let value_local = (quantity * unit.local).round_dp(10);
        let (lots, realised_gain_base) = match side {
            Side::Buy => (Vec::new(), None),
            Side::Sell => {
                let lots = holdings.lots_for_sale(account, id, quantity, options.lot_method)?;
                let cost: Decimal = lots.iter().map(|l| l.cost_base).sum();
                (lots, Some(value_base - cost))
            }
        };
        let trade = ProposedTrade {
            instrument: id.clone(),
            side,
            quantity,
            price: unit.price,
            currency: unit.currency,
            value_local,
            value_base,
            lots,
            realised_gain_base,
        };
        let balance = cash.entry(unit.currency).or_default();
        match side {
            Side::Buy => {
                *balance -= value_local;
                *quantities.entry(id.clone()).or_default() += quantity;
                purchases.push(trade);
            }
            Side::Sell => {
                *balance += value_local;
                *quantities.entry(id.clone()).or_default() -= quantity;
                sales.push(trade);
            }
        }
    }

    // Convert each foreign currency to its target balance through the base currency.
    let mut conversions = Vec::new();
    let foreign: Vec<Currency> = cash.keys().copied().filter(|c| *c != base).collect();
    for currency in foreign {
        let rate = rates[&currency];
        if rate <= Decimal::ZERO {
            continue;
        }
        let weight = wanted
            .get(&HoldingKey::Cash(currency))
            .copied()
            .unwrap_or(0.0);
        let target = (value_base * from_f64(weight) / rate).round_dp(currency.minor_units());
        let need = target - cash[&currency];
        let amount = Money::new(need.abs(), currency).round(Rounding::HalfEven);
        if amount.is_zero() {
            continue;
        }
        let in_base = Money::new(amount.amount() * rate, base).round(Rounding::HalfEven);
        let (sell, buy) = if need > Decimal::ZERO {
            (in_base, amount)
        } else {
            (amount, in_base)
        };
        *cash.entry(sell.currency()).or_default() -= sell.amount();
        *cash.entry(buy.currency()).or_default() += buy.amount();
        conversions.push(ProposedConversion { sell, buy });
    }
    if cash[&base] < Decimal::ZERO {
        warnings.warning(
            format!("cash.{base}"),
            format!(
                "the trades need {} {base} more than the account holds",
                -cash[&base]
            ),
        );
    }

    let traded: Decimal = sales.iter().chain(&purchases).map(|t| t.value_base).sum();
    let realised_gain_base = sales.iter().filter_map(|t| t.realised_gain_base).sum();
    let weights_after = weights(&quantities, &cash);
    sales.extend(purchases);
    Ok(Rebalance {
        account: account.clone(),
        date,
        value_base,
        trades: sales,
        conversions,
        weights_before,
        weights_after,
        turnover: to_f64(traded) / to_f64(value_base),
        realised_gain_base,
        warnings,
    })
}

impl Rebalance {
    /// The proposal as transactions on its date, with identifiers `{prefix}1`, `{prefix}2` and so
    /// on: sales naming their lots, then conversions, then purchases.
    pub fn transactions(&self, prefix: &str) -> Result<Vec<Transaction>> {
        let mut out = Vec::new();
        let mut next = |kind: TransactionKind| -> Result<()> {
            out.push(Transaction {
                id: TransactionId::new(format!("{prefix}{}", out.len() + 1)).ok_or_else(|| {
                    invalid(
                        "prefix",
                        "must make identifiers without surrounding whitespace".into(),
                    )
                })?,
                account: self.account.clone(),
                trade_date: self.date,
                settle_date: None,
                kind,
                note: None,
            });
            Ok(())
        };
        for t in self.trades.iter().filter(|t| t.side == Side::Sell) {
            next(TransactionKind::Sell {
                instrument: t.instrument.clone(),
                quantity: t.quantity,
                price: t.price,
                fees: Decimal::ZERO,
                lot_method: Some(LotMethod::SpecificId),
                lots: t
                    .lots
                    .iter()
                    .map(|l| LotPick {
                        lot: l.lot.clone(),
                        quantity: l.quantity,
                    })
                    .collect(),
            })?;
        }
        for c in &self.conversions {
            next(TransactionKind::FxConversion {
                sell: c.sell,
                buy: c.buy,
            })?;
        }
        for t in self.trades.iter().filter(|t| t.side == Side::Buy) {
            next(TransactionKind::Buy {
                instrument: t.instrument.clone(),
                quantity: t.quantity,
                price: t.price,
                fees: Decimal::ZERO,
            })?;
        }
        Ok(out)
    }
}

struct Unit {
    price: Decimal,
    currency: Currency,
    /// The value of one unit in the instrument's currency.
    local: Decimal,
    /// The value of one unit in the base currency.
    base: Decimal,
}

struct Look<'a> {
    market: &'a MarketData,
    date: Date,
    problems: &'a mut ValidationReport,
    warnings: &'a mut ValidationReport,
}

impl Look<'_> {
    fn fx(&mut self, from: Currency, to: Currency) -> Decimal {
        let path = format!("fx.{from}{to}");
        self.observe(self.market.fx_rate(from, to, self.date), path)
    }

    fn price(&mut self, id: &InstrumentId) -> Decimal {
        let path = format!("prices.{id}");
        self.observe(self.market.price(id, self.date), path)
    }

    fn observe(&mut self, observation: Observation<Decimal>, path: String) -> Decimal {
        match observation {
            Observation::Observed(value) => value,
            Observation::Carried { value, from } => {
                self.warnings
                    .warning(path, format!("value for {} carried from {from}", self.date));
                value
            }
            Observation::Missing => {
                self.problems
                    .error(path, format!("no value on or shortly before {}", self.date));
                Decimal::ZERO
            }
        }
    }
}

fn invalid(path: &str, message: String) -> Error {
    crate::invalid(path, message)
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}

fn from_f64(value: f64) -> Decimal {
    Decimal::try_from(value).unwrap_or_default()
}
