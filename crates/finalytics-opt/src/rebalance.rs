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
#[non_exhaustive]
pub struct RebalanceOptions {
    /// How sales choose lots: by default, each account's own method (first in, first out
    /// unless the account says otherwise). Highest cost first defers gains. Ignored when `tax`
    /// is given.
    pub lot_method: Option<LotMethod>,
    /// Trade whole units only, rounding each trade towards zero.
    pub whole_units: bool,
    /// Leave out trades worth less than this in the base currency.
    pub minimum_trade: Decimal,
    /// The fees each trade pays; none by default.
    pub fees: Fees,
    /// Tax rates on realised gains. When given, each sale takes the lots that cost the least tax,
    /// losses first, and the tax is estimated.
    pub tax: Option<TaxRates>,
}

impl Default for RebalanceOptions {
    fn default() -> Self {
        RebalanceOptions {
            lot_method: None,
            whole_units: false,
            minimum_trade: Decimal::ZERO,
            fees: Fees::default(),
            tax: None,
        }
    }
}

/// The fee a trade pays, in the instrument's currency: `max(minimum, per_trade + rate * value)`,
/// rounded to the currency's minor units. Purchases are sized so that the purchase and its fee
/// fit the target; sales pay the fee from their proceeds.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct Fees {
    /// A share of the trade's value, such as 0.001 for 10 basis points.
    pub rate: Decimal,
    /// A fixed amount per trade.
    pub per_trade: Decimal,
    /// The smallest fee a trade pays.
    pub minimum: Decimal,
}

impl Fees {
    fn of(&self, value: Decimal, currency: Currency) -> Decimal {
        let fee = (self.per_trade + self.rate * value).max(self.minimum);
        Money::new(fee, currency).round(Rounding::HalfEven).amount()
    }

    fn is_zero(&self) -> bool {
        self.rate.is_zero() && self.per_trade.is_zero() && self.minimum.is_zero()
    }
}

/// Tax rates on realised gains, for tax-aware sales. A lot held for more than
/// `long_term_after_days` days is long-term. The rates apply to gains in the base currency,
/// after fees; a loss saves tax at its rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TaxRates {
    /// The rate on gains from lots held for at most `long_term_after_days` days.
    pub short_term: Decimal,
    /// The rate on gains from lots held for longer.
    pub long_term: Decimal,
    /// The holding period, in days, beyond which a gain is long-term: 365 by default.
    #[serde(default = "a_year")]
    pub long_term_after_days: i32,
}

fn a_year() -> i32 {
    365
}

impl TaxRates {
    /// Short- and long-term rates, with gains long-term after 365 days.
    pub fn new(short_term: Decimal, long_term: Decimal) -> Self {
        TaxRates {
            short_term,
            long_term,
            long_term_after_days: a_year(),
        }
    }
}

impl TaxRates {
    fn is_long_term(&self, acquired: Date, sold: Date) -> bool {
        acquired.days_until(sold) > self.long_term_after_days
    }

    fn rate(&self, acquired: Date, sold: Date) -> Decimal {
        if self.is_long_term(acquired, sold) {
            self.long_term
        } else {
            self.short_term
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

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Side::Buy => "buy",
            Side::Sell => "sell",
        })
    }
}

/// A proposed trade in one instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
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
    /// The fee, in the instrument's currency.
    pub fees_local: Decimal,
    /// For a sale, the gain it would realise in the base currency, after fees, as the books
    /// would record it.
    pub realised_gain_base: Option<Decimal>,
    /// For a sale with tax rates, the estimated tax on its gains in the base currency; negative
    /// when its losses save tax. None in an account that is not taxable.
    pub tax_base: Option<Decimal>,
    /// For a sale with tax rates, the part of its realised gain from lots held long-term; the
    /// rest is short-term.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_term_gain_base: Option<Decimal>,
}

/// A proposed currency conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ProposedConversion {
    /// The amount sold.
    pub sell: Money,
    /// The amount bought.
    pub buy: Money,
}

/// The trades that move an account to target weights, and their effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
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
    pub weights_before: BTreeMap<HoldingKey, f64>,
    /// Each holding's weight after the trades.
    pub weights_after: BTreeMap<HoldingKey, f64>,
    /// The one-way turnover: half the value traded, sales and purchases, as a share of the
    /// account's value.
    pub turnover: f64,
    /// The gain the sales would realise in the base currency, after fees.
    pub realised_gain_base: Decimal,
    /// The fees of all the trades, in the base currency.
    pub fees_base: Decimal,
    /// With tax rates, the estimated tax on the sales' gains in the base currency; None in an
    /// account that is not taxable.
    pub tax_base: Option<Decimal>,
    /// Whether the account's gains are taxed when they are realised.
    #[serde(default = "yes")]
    #[cfg_attr(feature = "schema", schemars(extend("default" = true)))]
    pub taxable: bool,
    /// Carried prices, a shortfall of cash, and trades left out by rounding.
    pub warnings: ValidationReport,
}

fn yes() -> bool {
    true
}

/// Checks the target weights as given, at `targets.<holding>`: each a known holding, listed
/// once, with a weight of zero or more, and together no more than one.
fn check_targets(books: &Books, targets: &[(HoldingKey, f64)], problems: &mut ValidationReport) {
    let mut seen = std::collections::BTreeSet::new();
    for (holding, weight) in targets {
        let path = format!("targets.{holding}");
        if !seen.insert(holding) {
            problems.error(&path, "is listed a second time");
        }
        if !(weight.is_finite() && *weight >= 0.0) {
            problems.error(
                &path,
                format!("must be a weight of zero or more, not {weight}"),
            );
        }
        if let HoldingKey::Instrument(id) = holding
            && books.instrument(id).is_none()
        {
            problems.error(&path, "is not an instrument of the portfolio");
        }
    }
    let total: f64 = targets.iter().map(|(_, w)| w).sum();
    if total > 1.0 + 1e-9 {
        problems.error(
            "targets",
            format!("the weights add up to {total}, more than 1"),
        );
    }
}

/// Proposes the trades that move `account` to `targets`: a weight for each holding, as a share of
/// the account's value at `date`. Instruments held but not listed are sold; cash in the base
/// currency takes whatever weight the targets leave, and cash in other currencies is converted
/// to its target (zero if not listed). Only long positions are proposed.
///
/// Sales choose lots by [`RebalanceOptions::lot_method`] (each account's own by default), or,
/// with tax rates, the lots that cost the least tax. [`Rebalance::transactions`] returns the
/// trades as transactions that name those lots and carry their fees, ready to apply to the
/// books.
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
    check_targets(books, targets, &mut problems);
    if options.minimum_trade < Decimal::ZERO {
        problems.error("options.minimum_trade", "must not be negative");
    }
    let f = &options.fees;
    for (value, name) in [
        (f.rate, "rate"),
        (f.per_trade, "per_trade"),
        (f.minimum, "minimum"),
    ] {
        if value < Decimal::ZERO {
            problems.error(format!("options.fees.{name}"), "must not be negative");
        }
    }
    if let Some(tax) = &options.tax {
        for (value, name) in [(tax.short_term, "short_term"), (tax.long_term, "long_term")] {
            if value < Decimal::ZERO || value > Decimal::ONE {
                problems.error(format!("options.tax.{name}"), "must be between 0 and 1");
            }
        }
        if tax.long_term_after_days < 0 {
            problems.error("options.tax.long_term_after_days", "must not be negative");
        }
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
        let out: BTreeMap<HoldingKey, f64> = quantities
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
        out
    };
    let weights_before = weights(&quantities, &cash);

    let held_in = books.accounts().iter().find(|a| &a.id == account);
    let taxable = held_in.is_none_or(|a| a.taxable);
    // Without a method of its own, a sale takes the lots as the account's sales do; naming
    // lots is for one sale, so an account set to name them sells first in, first out here.
    let lot_method = options
        .lot_method
        .or_else(|| held_in.map(|a| a.lot_method))
        .filter(|m| *m != LotMethod::SpecificId)
        .unwrap_or_default();
    let taxed = options.tax.as_ref().filter(|_| taxable);
    // The trades with purchases sized on `investable`, the value the targets share. Sales are
    // sized on the account's value; when the fees would take the base-currency cash below
    // zero, purchases are sized again on a value less the shortfall, so that the trades fit the
    // cash the account has.
    let attempt = |investable: Decimal,
                   trims: &BTreeMap<InstrumentId, Decimal>|
     -> Result<Planned> {
        let mut quantities = quantities.clone();
        let mut cash = cash.clone();
        let mut warnings = ValidationReport::new();
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
            let size = |value: Decimal| {
                ((value * from_f64(weight) - held * unit.base) / unit.base).round_dp(10)
            };
            let mut quantity = size(value_base);
            if quantity > Decimal::ZERO {
                quantity = size(investable).max(Decimal::ZERO);
                if !options.fees.is_zero() {
                    quantity = fee_adjusted(quantity, unit, &options.fees);
                }
                if let Some(trim) = trims.get(id) {
                    quantity = (quantity - trim).max(Decimal::ZERO);
                }
            }
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
            let fees = if options.fees.is_zero() {
                Decimal::ZERO
            } else {
                options.fees.of(value_local, unit.currency)
            };
            let rate = rates[&unit.currency];
            let (lots, realised_gain_base, tax_base, long_term_gain_base) = match side {
                Side::Buy => (Vec::new(), None, None, None),
                Side::Sell => {
                    let lots = match taxed {
                        Some(tax) => {
                            let picks = cheapest_lots(
                                &holdings, account, id, quantity, unit, rate, tax, date,
                            );
                            holdings.lots_for_picks(account, id, &picks)?
                        }
                        None => holdings.lots_for_sale(account, id, quantity, lot_method)?,
                    };
                    // Proceeds after fees, split across the lots as the books split them.
                    let proceeds = quantity * unit.local - fees;
                    let gains = lot_gains(&lots, quantity, proceeds, rate);
                    let gain: Decimal = gains.iter().sum();
                    let tax = taxed.map(|tax| {
                        lots.iter()
                            .zip(&gains)
                            .map(|(l, g)| tax.rate(l.acquired, date) * g)
                            .sum::<Decimal>()
                            .round_dp(10)
                    });
                    let long_term = options.tax.as_ref().map(|tax| {
                        lots.iter()
                            .zip(&gains)
                            .filter(|(l, _)| tax.is_long_term(l.acquired, date))
                            .map(|(_, g)| *g)
                            .sum::<Decimal>()
                    });
                    (lots, Some(gain), tax, long_term)
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
                fees_local: fees,
                realised_gain_base,
                tax_base,
                long_term_gain_base,
            };
            let balance = cash.entry(unit.currency).or_default();
            match side {
                Side::Buy => {
                    *balance -= value_local + fees;
                    *quantities.entry(id.clone()).or_default() += quantity;
                    purchases.push(trade);
                }
                Side::Sell => {
                    *balance += value_local - fees;
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
        Ok(Planned {
            sales,
            purchases,
            conversions,
            quantities,
            cash,
            warnings,
        })
    };
    // First the value the purchases share is reduced by the shortfall, which makes room for
    // fees in proportion; a shortfall that rounding leaves is then taken from the largest
    // purchase.
    let mut investable = value_base;
    let mut trims: BTreeMap<InstrumentId, Decimal> = BTreeMap::new();
    let mut planned = attempt(investable, &trims)?;
    for round in 0..8 {
        let short = -planned.cash[&base];
        if short <= Decimal::ZERO {
            break;
        }
        if round < 2 {
            investable -= short;
        } else {
            let Some(largest) = planned
                .purchases
                .iter()
                .max_by(|a, b| a.value_base.cmp(&b.value_base))
            else {
                break;
            };
            let unit = &units[&largest.instrument];
            let mut cut = short / unit.base;
            cut = if options.whole_units {
                cut.ceil()
            } else {
                cut.round_dp(10) + Decimal::new(1, 10)
            };
            *trims.entry(largest.instrument.clone()).or_default() += cut;
        }
        planned = attempt(investable, &trims)?;
    }
    let Planned {
        mut sales,
        purchases,
        conversions,
        quantities,
        cash,
        warnings: found,
    } = planned;
    warnings.merge("", found);
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
    let fees_base = sales
        .iter()
        .chain(&purchases)
        .map(|t| t.fees_local * rates[&t.currency])
        .sum::<Decimal>()
        .round_dp(10);
    let tax_base = taxed.map(|_| sales.iter().filter_map(|t| t.tax_base).sum());
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
        turnover: to_f64(traded) / to_f64(value_base) / 2.0,
        realised_gain_base,
        fees_base,
        tax_base,
        taxable,
        warnings,
    })
}

/// The trades of one sizing of a rebalance, with the holdings and cash they leave.
struct Planned {
    sales: Vec<ProposedTrade>,
    purchases: Vec<ProposedTrade>,
    conversions: Vec<ProposedConversion>,
    quantities: BTreeMap<InstrumentId, Decimal>,
    cash: BTreeMap<Currency, Decimal>,
    warnings: ValidationReport,
}

/// The trades that move several accounts together to target weights of their combined value,
/// one proposal per account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RebalanceResults {
    /// The proposal for each account with value, in the order given.
    pub accounts: Vec<Rebalance>,
    /// The base currency, of the values below.
    pub base_currency: Currency,
    /// The accounts' combined value in the base currency.
    pub value_base: Decimal,
    /// Each holding's weight in the combined value before the trades.
    pub weights_before: BTreeMap<HoldingKey, f64>,
    /// Each holding's weight in the combined value after the trades.
    pub weights_after: BTreeMap<HoldingKey, f64>,
    /// The one-way turnover of all the accounts: half the value traded, as a share of their
    /// combined value.
    pub turnover: f64,
    /// The gain the sales would realise in the base currency, after fees.
    pub realised_gain_base: Decimal,
    /// The fees of all the trades, in the base currency.
    pub fees_base: Decimal,
    /// With tax rates, the estimated tax on the sales' gains in the base currency.
    pub tax_base: Option<Decimal>,
}

/// Proposes the trades that move `accounts` together to `targets`: a weight for each holding, as
/// a share of the accounts' combined value at `date`, as for [`rebalance`].
///
/// Each holding is spread across the accounts by a simple rule: a holding above its target is
/// sold from the accounts that hold it in proportion to their holdings of it, and a holding below
/// its target is bought in proportion to the value each account has free, its value less what it
/// keeps. Each account is then rebalanced to its share with [`rebalance`] and `options`, so sales
/// choose lots, pay fees and estimate tax as they would in one account, and purchases are made
/// with each account's own cash. Trades do not move cash between accounts. With one account,
/// the proposal is [`rebalance`]'s.
pub fn rebalance_accounts(
    books: &Books,
    market: &MarketData,
    date: Date,
    accounts: &[AccountId],
    targets: &[(HoldingKey, f64)],
    options: &RebalanceOptions,
) -> Result<RebalanceResults> {
    let base = books.base_currency();
    let mut report = ValidationReport::new();
    if accounts.is_empty() {
        report.error("accounts", "needs at least one account");
    }
    let mut seen = std::collections::BTreeSet::new();
    for (k, a) in accounts.iter().enumerate() {
        if !seen.insert(a) {
            report.error(format!("accounts[{k}]"), format!("{a} is listed twice"));
        }
    }
    // The targets as given: spread across the accounts, they would be clipped to each one's
    // share and their problems hidden.
    check_targets(books, targets, &mut report);
    report.into_result(())?;
    if let [account] = accounts {
        let proposal = rebalance(books, market, date, account, targets, options)?;
        return Ok(combine(vec![proposal], base, options));
    }
    let valuation = books.value_on(date, market)?;
    // Each account's value of each holding other than base cash, and its total value.
    let mut held: BTreeMap<&AccountId, BTreeMap<HoldingKey, f64>> = BTreeMap::new();
    let mut totals: BTreeMap<&AccountId, f64> = accounts.iter().map(|a| (a, 0.0)).collect();
    for p in &valuation.positions {
        if let Some(total) = totals.get_mut(&p.account) {
            let value = to_f64(p.market_value_base);
            *total += value;
            *held
                .entry(&p.account)
                .or_default()
                .entry(HoldingKey::Instrument(p.instrument.clone()))
                .or_default() += value;
        }
    }
    for c in &valuation.cash {
        if let Some(total) = totals.get_mut(&c.account) {
            let value = to_f64(c.value_base);
            *total += value;
            if c.currency != base {
                *held
                    .entry(&c.account)
                    .or_default()
                    .entry(HoldingKey::Cash(c.currency))
                    .or_default() += value;
            }
        }
    }
    let combined: f64 = totals.values().sum();
    if combined.is_nan() || combined <= 0.0 {
        return Err(crate::invalid("accounts", "have no value to rebalance"));
    }
    let weight_of: BTreeMap<&HoldingKey, f64> = targets.iter().map(|(k, w)| (k, *w)).collect();
    let keys: std::collections::BTreeSet<HoldingKey> = held
        .values()
        .flat_map(|h| h.keys().cloned())
        .chain(
            targets
                .iter()
                .map(|(k, _)| k.clone())
                .filter(|k| *k != HoldingKey::Cash(base)),
        )
        .collect();
    // What each account keeps of each holding, and what remains to buy.
    let mut kept: BTreeMap<&AccountId, BTreeMap<HoldingKey, f64>> = BTreeMap::new();
    let mut to_buy: BTreeMap<HoldingKey, f64> = BTreeMap::new();
    for key in &keys {
        let target = weight_of.get(key).copied().unwrap_or(0.0) * combined;
        let total: f64 = held.values().filter_map(|h| h.get(key)).sum();
        let keep = if total > target && total > 0.0 {
            target / total
        } else {
            1.0
        };
        for (account, holdings) in &held {
            if let Some(value) = holdings.get(key) {
                kept.entry(account)
                    .or_default()
                    .insert(key.clone(), value * keep);
            }
        }
        if target > total {
            to_buy.insert(key.clone(), target - total);
        }
    }
    let free: BTreeMap<&AccountId, f64> = totals
        .iter()
        .map(|(a, v)| {
            let keeps: f64 = kept.get(a).map_or(0.0, |k| k.values().sum());
            (*a, (v - keeps).max(0.0))
        })
        .collect();
    let all_free: f64 = free.values().sum();

    let mut proposals = Vec::new();
    for account in accounts {
        let value = totals[account];
        if value <= 0.0 {
            continue;
        }
        let share = if all_free > 0.0 {
            free[account] / all_free
        } else {
            0.0
        };
        let mut wanted: BTreeMap<HoldingKey, f64> = kept.get(account).cloned().unwrap_or_default();
        for (key, amount) in &to_buy {
            *wanted.entry(key.clone()).or_default() += amount * share;
        }
        let account_targets: Vec<(HoldingKey, f64)> = wanted
            .into_iter()
            .map(|(k, v)| (k, (v / value).clamp(0.0, 1.0)))
            .collect();
        proposals.push(rebalance(
            books,
            market,
            date,
            account,
            &account_targets,
            options,
        )?);
    }

    Ok(combine(proposals, base, options))
}

/// The proposals of several accounts as one.
fn combine(
    proposals: Vec<Rebalance>,
    base_currency: Currency,
    options: &RebalanceOptions,
) -> RebalanceResults {
    let value_base: Decimal = proposals.iter().map(|p| p.value_base).sum();
    // Each account's weights are of its own value, before the trades or after them (less the
    // fees), so they are combined in proportion to those values.
    let combine = |pick: fn(&Rebalance) -> &BTreeMap<HoldingKey, f64>,
                   value: fn(&Rebalance) -> Decimal| {
        let mut out: BTreeMap<HoldingKey, f64> = BTreeMap::new();
        let total: f64 = proposals.iter().map(|p| to_f64(value(p))).sum();
        for p in &proposals {
            let scale = to_f64(value(p)) / total;
            for (k, w) in pick(p) {
                *out.entry(k.clone()).or_default() += w * scale;
            }
        }
        out
    };
    let traded: f64 = proposals
        .iter()
        .map(|p| p.turnover * to_f64(p.value_base))
        .sum();
    RebalanceResults {
        weights_before: combine(|p| &p.weights_before, |p| p.value_base),
        weights_after: combine(|p| &p.weights_after, |p| p.value_base - p.fees_base),
        turnover: traded / to_f64(value_base),
        realised_gain_base: proposals.iter().map(|p| p.realised_gain_base).sum(),
        fees_base: proposals.iter().map(|p| p.fees_base).sum(),
        tax_base: options
            .tax
            .as_ref()
            .map(|_| proposals.iter().filter_map(|p| p.tax_base).sum()),
        value_base,
        base_currency,
        accounts: proposals,
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `value` to `places` decimals, with thousands separated by commas and a true minus sign.
fn grouped(value: Decimal, places: u32) -> String {
    let text = format!("{:.*}", places as usize, value.abs().round_dp(places));
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text.as_str(), None), |(w, f)| (w, Some(f)));
    let mut out = String::new();
    for (k, c) in whole.chars().enumerate() {
        if k > 0 && (whole.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(fraction) = fraction {
        out.push('.');
        out.push_str(fraction);
    }
    if value < Decimal::ZERO {
        out.insert(0, '−');
    }
    out
}

impl std::fmt::Display for RebalanceResults {
    /// The trades of every account, one line each, with the turnover, fees, tax and realised
    /// gain.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trades: usize = self.accounts.iter().map(|a| a.trades.len()).sum();
        let date = self.accounts.first().map(|a| a.date);
        write!(
            f,
            "RebalanceResults · {} · {}",
            plural(self.accounts.len(), "account"),
            plural(trades, "trade")
        )?;
        if let Some(d) = date {
            write!(
                f,
                " · prices of {} {} {}",
                d.day(),
                MONTHS[(d.month().clamp(1, 12) - 1) as usize],
                d.year()
            )?;
        }
        writeln!(f)?;
        writeln!(f)?;
        let width = self
            .accounts
            .iter()
            .map(|a| a.account.as_str().chars().count())
            .chain([7])
            .max()
            .unwrap_or(7)
            .min(16);
        let cell = |text: &str| {
            let count = text.chars().count();
            if count > width {
                let mut out: String = text.chars().take(width - 1).collect();
                out.push('…');
                out
            } else {
                format!("{text}{}", " ".repeat(width - count))
            }
        };
        writeln!(
            f,
            "  {}  {:<4} {:>9}  {:<10} {:>14} {:>18}",
            cell("Account"),
            "Side",
            "Units",
            "Instrument",
            "Price",
            "Value"
        )?;
        for a in &self.accounts {
            for t in &a.trades {
                writeln!(
                    f,
                    "  {}  {:<4} {:>9}  {:<10} {:>14} {:>18}",
                    cell(a.account.as_str()),
                    t.side.to_string(),
                    grouped(t.quantity, t.quantity.normalize().scale().min(4)),
                    t.instrument.as_str(),
                    format!("{} {}", grouped(t.price, 2), t.currency),
                    format!("{} {}", grouped(t.value_local, 2), t.currency)
                )?;
            }
            for c in &a.conversions {
                writeln!(
                    f,
                    "  {}  Convert {} {} to {} {}",
                    cell(a.account.as_str()),
                    grouped(c.sell.amount(), 2),
                    c.sell.currency(),
                    grouped(c.buy.amount(), 2),
                    c.buy.currency()
                )?;
            }
        }
        let base = self.base_currency;
        writeln!(f)?;
        write!(
            f,
            "  Turnover {:.1}% · fees {} {base}",
            self.turnover * 100.0,
            grouped(self.fees_base, 2)
        )?;
        if let Some(tax) = self.tax_base {
            write!(f, " · tax {} {base}", grouped(tax, 2))?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "  Realised gain {} {base}.",
            grouped(self.realised_gain_base, 2)
        )?;
        write!(
            f,
            "  Each account's trades, conversions and weights are in results.accounts[k]."
        )
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

impl Rebalance {
    /// The proposal as transactions on its date, with identifiers `{prefix}1`, `{prefix}2` and so
    /// on: sales naming their lots, then conversions, then purchases.
    pub fn transactions(&self, prefix: &str) -> Result<Vec<Transaction>> {
        let mut out = Vec::new();
        let mut next = |kind: TransactionKind| -> Result<()> {
            let id = TransactionId::new(format!("{prefix}{}", out.len() + 1)).ok_or_else(|| {
                invalid(
                    "prefix",
                    "must make identifiers without surrounding whitespace".into(),
                )
            })?;
            out.push(Transaction::new(id, self.account.clone(), self.date, kind));
            Ok(())
        };
        for t in self.trades.iter().filter(|t| t.side == Side::Sell) {
            next(TransactionKind::Sell {
                instrument: t.instrument.clone(),
                quantity: t.quantity,
                price: t.price,
                fees: t.fees_local,
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
                fees: t.fees_local,
            })?;
        }
        Ok(out)
    }
}

/// The largest purchase, below `quantity`, whose value and fee together fit the value of
/// `quantity` units.
fn fee_adjusted(quantity: Decimal, unit: &Unit, fees: &Fees) -> Decimal {
    if unit.local <= Decimal::ZERO {
        return quantity;
    }
    let budget = quantity * unit.local;
    let proportional = (budget - fees.per_trade) / (unit.local * (Decimal::ONE + fees.rate));
    let fixed = (budget - fees.minimum) / unit.local;
    // The fee is the larger of the two schedules, so the purchase is the smaller.
    let fitted = if fees.per_trade + fees.rate * proportional * unit.local >= fees.minimum {
        proportional
    } else {
        fixed
    };
    // Rounding the fee up to minor units can take a hair more; keep the purchase within budget.
    fitted.max(Decimal::ZERO).trunc_with_scale(10)
}

/// Lots to sell for `quantity` units, cheapest in tax first: by the tax on each unit's gain at the
/// sale's price, losses (which save tax) first, then the lowest taxed gains; ties in the order
/// the lots were opened.
#[allow(clippy::too_many_arguments)]
fn cheapest_lots(
    holdings: &finalytics_ledger::Holdings,
    account: &AccountId,
    instrument: &InstrumentId,
    quantity: Decimal,
    unit: &Unit,
    rate: Decimal,
    tax: &TaxRates,
    date: Date,
) -> Vec<LotPick> {
    let price_base = unit.local * rate;
    let mut lots: Vec<(Decimal, u64, &finalytics_ledger::Lot)> = holdings
        .lots_of(account, instrument)
        .iter()
        .filter(|l| l.quantity > Decimal::ZERO)
        .map(|l| {
            let gain = price_base - l.cost_base / l.quantity;
            (tax.rate(l.acquired, date) * gain, l.sequence, l)
        })
        .collect();
    lots.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut remaining = quantity;
    let mut picks = Vec::new();
    for (_, _, lot) in lots {
        if remaining.is_zero() {
            break;
        }
        let take = remaining.min(lot.quantity);
        picks.push(LotPick {
            lot: lot.id.clone(),
            quantity: take,
        });
        remaining -= take;
    }
    picks
}

/// Each lot's gain in the base currency, with the proceeds split across lots in proportion to
/// their quantities and rounded as the books round them, the last lot taking the remainder.
fn lot_gains(
    lots: &[LotSale],
    quantity: Decimal,
    proceeds: Decimal,
    rate: Decimal,
) -> Vec<Decimal> {
    let mut remaining = proceeds;
    lots.iter()
        .enumerate()
        .map(|(i, l)| {
            let share = if i + 1 == lots.len() || quantity.is_zero() {
                remaining
            } else {
                (proceeds * l.quantity / quantity).round_dp(10)
            };
            remaining -= share;
            (share * rate).round_dp(10) - l.cost_base
        })
        .collect()
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
