//! Keeping the books: applying transactions in order.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::str::FromStr;

use finalytics_core::{Currency, Date, Decimal, Error, Result, ValidationReport};
use serde::{Deserialize, Serialize};

use crate::lots::{Relief, ReliefError, derived, relieve};
use crate::{
    Account, AccountId, Instrument, InstrumentId, Lot, LotId, LotMethod, LotPick, MarketData,
    PortfolioData, Transaction, TransactionId, TransactionKind,
};

/// A change to the books made by one transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Effect {
    /// A lot was opened or changed; this is its new state.
    SetLot {
        /// The lot after the transaction.
        lot: Lot,
    },
    /// A lot was closed.
    CloseLot {
        /// The account that held it.
        account: AccountId,
        /// The instrument it held.
        instrument: InstrumentId,
        /// The lot.
        lot: LotId,
    },
    /// Cash in one currency changed by `amount` (negative for a payment).
    Cash {
        /// The account.
        account: AccountId,
        /// The currency.
        currency: Currency,
        /// The change.
        amount: Decimal,
    },
}

/// Why value entered or left the portfolio from outside: the flows that performance measurement
/// must separate from investment returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FlowKind {
    /// Cash deposited.
    Deposit,
    /// Cash withdrawn.
    Withdrawal,
    /// Units transferred in, at market value.
    TransferIn,
    /// Units transferred out, at market value.
    TransferOut,
}

/// Value entering (positive) or leaving (negative) the portfolio, in `currency`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Flow {
    /// The account.
    pub account: AccountId,
    /// The currency of `amount`.
    pub currency: Currency,
    /// The value, positive into the portfolio.
    pub amount: Decimal,
    /// What kind of flow it was.
    pub kind: FlowKind,
}

/// A holding: an instrument, or cash in one currency, added up across accounts.
///
/// In data, a holding is written as text: the instrument's identifier (`"SPY"`), or the
/// currency's code followed by ` cash` (`"USD cash"`). No instrument's identifier takes the
/// second form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HoldingKey {
    /// An instrument.
    Instrument(InstrumentId),
    /// Cash in one currency.
    Cash(Currency),
}

impl HoldingKey {
    /// The cash holding of `currency`, written `"USD cash"`.
    pub fn cash(currency: Currency) -> Self {
        HoldingKey::Cash(currency)
    }

    /// Whether `text` has the form of a cash holding, `"USD cash"`: an identifier of that form
    /// would be read as cash, so no instrument may have one.
    pub fn is_cash_form(text: &str) -> bool {
        text.strip_suffix(" cash")
            .is_some_and(|code| Currency::new(code).is_ok())
    }
}

impl fmt::Display for HoldingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HoldingKey::Instrument(id) => f.write_str(id.as_str()),
            HoldingKey::Cash(currency) => write!(f, "{currency} cash"),
        }
    }
}

impl FromStr for HoldingKey {
    type Err = Error;

    /// Reads `"USD cash"` as cash in US dollars and anything else as an instrument's identifier.
    fn from_str(text: &str) -> Result<Self> {
        if let Some(code) = text.strip_suffix(" cash")
            && let Ok(currency) = Currency::new(code)
        {
            return Ok(HoldingKey::Cash(currency));
        }
        InstrumentId::new(text)
            .map(HoldingKey::Instrument)
            .ok_or_else(|| {
                let mut report = ValidationReport::new();
                report.error(
                    "holding",
                    format!(
                        "{text:?} is not a holding: write an instrument's identifier, or a \
                         currency followed by \" cash\", such as \"USD cash\""
                    ),
                );
                Error::Validation(report)
            })
    }
}

impl From<InstrumentId> for HoldingKey {
    fn from(id: InstrumentId) -> Self {
        HoldingKey::Instrument(id)
    }
}

impl Serialize for HoldingKey {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for HoldingKey {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for HoldingKey {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "HoldingKey".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "minLength": 1,
            "description": "A holding: an instrument's identifier, such as \"SPY\", or a currency's code followed by \" cash\", such as \"USD cash\"."
        })
    }
}

/// How a transaction moved value between holdings, used to measure each holding's
/// contribution. Value moved between holdings nets to zero; only external flows and expenses
/// change the portfolio's total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Allocation {
    /// `amount` (in `currency`) moved into `holding` (negative: out of it).
    Trade {
        /// The holding.
        holding: HoldingKey,
        /// The currency of `amount`.
        currency: Currency,
        /// The value moved in.
        amount: Decimal,
    },
    /// `amount` (in `currency`) earned by `holding` and paid out to cash, such as a dividend.
    Income {
        /// The holding that earned it.
        holding: HoldingKey,
        /// The currency of `amount`.
        currency: Currency,
        /// The amount, net of tax withheld.
        amount: Decimal,
    },
    /// `amount` (in `currency`) paid out of cash for a fee or a tax charged to the account
    /// rather than to a holding: a cost of the portfolio, not a loss on the cash.
    Expense {
        /// The currency of `amount`.
        currency: Currency,
        /// The amount paid.
        amount: Decimal,
    },
    /// `quantity` units of `to` replaced part of `from` (a spin-off, merger or symbol change);
    /// the value moved is the market value of the new units on the date.
    Exchange {
        /// The instrument given up, wholly or in part.
        from: InstrumentId,
        /// The instrument received.
        to: InstrumentId,
        /// The units of `to` received.
        quantity: Decimal,
    },
}

/// One transaction as applied to the books.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Event {
    /// The trade date.
    pub date: Date,
    /// The transaction.
    pub transaction: TransactionId,
    /// Its position in the portfolio's transaction list.
    pub index: usize,
    /// What it changed.
    pub effects: Vec<Effect>,
    /// The external flows it caused.
    pub flows: Vec<Flow>,
    /// How it moved value between holdings.
    pub allocations: Vec<Allocation>,
}

/// A gain or loss realised by a sale, a return of capital or a cash merger, for one lot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RealisedGain {
    /// The transaction that realised it.
    pub transaction: TransactionId,
    /// The account.
    pub account: AccountId,
    /// The instrument.
    pub instrument: InstrumentId,
    /// The lot.
    pub lot: LotId,
    /// The units disposed of (zero for a return of capital beyond the cost basis).
    pub quantity: Decimal,
    /// When the lot was acquired.
    pub acquired: Date,
    /// When it was disposed of.
    pub disposed: Date,
    /// Proceeds net of fees, in the instrument's currency.
    pub proceeds_local: Decimal,
    /// Cost basis in the instrument's currency.
    pub cost_local: Decimal,
    /// `proceeds_local - cost_local`.
    pub gain_local: Decimal,
    /// Proceeds in the base currency, at the disposal date's rate.
    pub proceeds_base: Decimal,
    /// Cost basis in the base currency, at the acquisition rates.
    pub cost_base: Decimal,
    /// `proceeds_base - cost_base`, which includes the currency effect.
    pub gain_base: Decimal,
}

impl RealisedGain {
    /// The number of days the lot was held.
    pub fn holding_days(&self) -> i32 {
        self.acquired.days_until(self.disposed)
    }
}

/// A position: the open lots of one instrument in one account, added up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Position {
    /// The account.
    pub account: AccountId,
    /// The instrument.
    pub instrument: InstrumentId,
    /// The total quantity.
    pub quantity: Decimal,
    /// The total cost basis in the instrument's currency.
    pub cost_local: Decimal,
    /// The total cost basis in the base currency.
    pub cost_base: Decimal,
    /// The number of open lots.
    pub lots: usize,
}

/// The part of a lot that a sale would close.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct LotSale {
    /// The lot.
    pub lot: LotId,
    /// When the lot was acquired.
    pub acquired: Date,
    /// The units the sale would take from it.
    pub quantity: Decimal,
    /// The cost basis of those units in the instrument's currency.
    pub cost_local: Decimal,
    /// The cost basis of those units in the base currency.
    pub cost_base: Decimal,
}

/// A cash balance in one account and currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CashBalance {
    /// The account.
    pub account: AccountId,
    /// The currency.
    pub currency: Currency,
    /// The balance, negative if overdrawn.
    pub amount: Decimal,
}

/// Open lots and cash balances at a point in time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Holdings {
    lots: BTreeMap<(AccountId, InstrumentId), Vec<Lot>>,
    cash: BTreeMap<(AccountId, Currency), Decimal>,
}

impl Holdings {
    /// Every open lot, by account and instrument, in the order they were opened.
    pub fn lots(&self) -> impl Iterator<Item = &Lot> {
        self.lots.values().flatten()
    }

    /// The open lots of `instrument` in `account`.
    pub fn lots_of(&self, account: &AccountId, instrument: &InstrumentId) -> &[Lot] {
        self.lots
            .get(&(account.clone(), instrument.clone()))
            .map_or(&[], Vec::as_slice)
    }

    /// The positions, by account and then instrument.
    pub fn positions(&self) -> Vec<Position> {
        self.lots
            .iter()
            .filter(|(_, lots)| !lots.is_empty())
            .map(|((account, instrument), lots)| Position {
                account: account.clone(),
                instrument: instrument.clone(),
                quantity: lots.iter().map(|l| l.quantity).sum(),
                cost_local: lots.iter().map(|l| l.cost_local).sum(),
                cost_base: lots.iter().map(|l| l.cost_base).sum(),
                lots: lots.len(),
            })
            .collect()
    }

    /// The cash balances by account and currency, including zero balances of currencies used.
    pub fn cash(&self) -> impl Iterator<Item = (&AccountId, Currency, Decimal)> {
        self.cash
            .iter()
            .map(|((account, currency), amount)| (account, *currency, *amount))
    }

    /// The lots a sale of `quantity` units of `instrument` from `account` would close, chosen by
    /// `method`, without changing the holdings. Each lot's cost is the part the sale would take.
    ///
    /// [`LotMethod::SpecificId`] needs the lots named, so it is refused here; use
    /// [`Holdings::lots_for_picks`] for named lots.
    pub fn lots_for_sale(
        &self,
        account: &AccountId,
        instrument: &InstrumentId,
        quantity: Decimal,
        method: LotMethod,
    ) -> Result<Vec<LotSale>> {
        let mut report = ValidationReport::new();
        if quantity <= Decimal::ZERO {
            report.error("quantity", "must be greater than zero");
        }
        if method == LotMethod::SpecificId {
            report.error(
                "method",
                "choose a method that selects lots without naming them",
            );
        }
        if report.has_errors() {
            return Err(Error::Validation(report));
        }
        self.relieved(account, instrument, quantity, method, &[])
    }

    /// The lots a sale of the named `picks` from `account` would close, as the books would relieve
    /// them by specific identification, without changing the holdings.
    pub fn lots_for_picks(
        &self,
        account: &AccountId,
        instrument: &InstrumentId,
        picks: &[LotPick],
    ) -> Result<Vec<LotSale>> {
        let quantity: Decimal = picks.iter().map(|p| p.quantity).sum();
        if picks.is_empty() || picks.iter().any(|p| p.quantity <= Decimal::ZERO) {
            let mut report = ValidationReport::new();
            report.error(
                "picks",
                "must name lots, each with a quantity greater than zero",
            );
            return Err(Error::Validation(report));
        }
        self.relieved(account, instrument, quantity, LotMethod::SpecificId, picks)
    }

    fn relieved(
        &self,
        account: &AccountId,
        instrument: &InstrumentId,
        quantity: Decimal,
        method: LotMethod,
        picks: &[LotPick],
    ) -> Result<Vec<LotSale>> {
        let mut report = ValidationReport::new();
        let mut lots = self.lots_of(account, instrument).to_vec();
        match relieve(&mut lots, quantity, method, picks) {
            Ok(relief) => Ok(relief
                .into_iter()
                .map(|r| LotSale {
                    lot: r.lot.id,
                    acquired: r.lot.acquired,
                    quantity: r.quantity,
                    cost_local: r.cost_local,
                    cost_base: r.cost_base,
                })
                .collect()),
            Err(e) => {
                let message = match e {
                    ReliefError::Insufficient { held } => format!(
                        "needs {quantity} units of {instrument}, but account {account} holds {held}"
                    ),
                    ReliefError::UnknownLot(lot) | ReliefError::LotTooSmall { lot, .. } => {
                        format!("lot {lot} cannot be relieved")
                    }
                };
                report.error("quantity", message);
                Err(Error::Validation(report))
            }
        }
    }

    /// The cash balances as records, by account and currency.
    pub fn cash_balances(&self) -> Vec<CashBalance> {
        self.cash()
            .map(|(account, currency, amount)| CashBalance {
                account: account.clone(),
                currency,
                amount,
            })
            .collect()
    }

    /// The quantity of `instrument` held in all accounts.
    pub fn quantity(&self, instrument: &InstrumentId) -> Decimal {
        self.lots
            .iter()
            .filter(|((_, i), _)| i == instrument)
            .flat_map(|(_, lots)| lots.iter().map(|l| l.quantity))
            .sum()
    }

    fn apply(&mut self, effect: &Effect) {
        match effect {
            Effect::SetLot { lot } => {
                let key = (lot.account.clone(), lot.instrument.clone());
                let lots = self.lots.entry(key).or_default();
                match lots.iter_mut().find(|l| l.id == lot.id) {
                    Some(existing) => *existing = lot.clone(),
                    None => {
                        lots.push(lot.clone());
                        lots.sort_by_key(|l| l.sequence);
                    }
                }
            }
            Effect::CloseLot {
                account,
                instrument,
                lot,
            } => {
                let key = (account.clone(), instrument.clone());
                if let Some(lots) = self.lots.get_mut(&key) {
                    lots.retain(|l| &l.id != lot);
                    if lots.is_empty() {
                        self.lots.remove(&key);
                    }
                }
            }
            Effect::Cash {
                account,
                currency,
                amount,
            } => {
                *self.cash.entry((account.clone(), *currency)).or_default() += *amount;
            }
        }
    }
}

/// The books of a portfolio: every transaction applied in order, with the lots, cash and gains
/// that result.
///
/// Building the books checks the portfolio's structure first ([`PortfolioData::validate`]), then
/// applies the transactions in trade-date order. Applying stops at the first transaction that
/// cannot be applied, such as a sale of more units than are held, and reports it with its
/// position in the transaction list.
#[derive(Debug, Clone)]
pub struct Books {
    base_currency: Currency,
    accounts: Vec<Account>,
    instruments: BTreeMap<InstrumentId, Instrument>,
    events: Vec<Event>,
    realised: Vec<RealisedGain>,
    holdings: Holdings,
}

impl Books {
    /// Applies the portfolio's transactions, using `market` for FX rates (to record cost in the
    /// base currency) and for prices where a transfer needs a market value.
    pub fn build(portfolio: &PortfolioData, market: &MarketData) -> Result<Books> {
        portfolio.validate().into_result(())?;
        let mut order: Vec<(usize, &Transaction)> =
            portfolio.transactions.iter().enumerate().collect();
        order.sort_by_key(|(i, tx)| (tx.trade_date, *i));

        let mut engine = Engine {
            portfolio,
            market,
            instruments: portfolio
                .instruments
                .iter()
                .map(|i| (i.id.clone(), i))
                .collect(),
            holdings: Holdings::default(),
            realised: Vec::new(),
            sequence: 0,
        };
        let mut events = Vec::with_capacity(order.len());
        for (index, tx) in order {
            let event = engine.apply(index, tx).map_err(|message| {
                let mut report = ValidationReport::new();
                report.error(
                    format!("transactions[{index}]"),
                    format!(
                        "{} {} on {}: {message}",
                        tx.kind.name(),
                        tx.id,
                        tx.trade_date
                    ),
                );
                Error::Validation(report)
            })?;
            events.push(event);
        }
        Ok(Books {
            base_currency: portfolio.base_currency,
            accounts: portfolio.accounts.clone(),
            instruments: portfolio
                .instruments
                .iter()
                .map(|i| (i.id.clone(), i.clone()))
                .collect(),
            events,
            realised: engine.realised,
            holdings: engine.holdings,
        })
    }

    /// The base currency.
    pub fn base_currency(&self) -> Currency {
        self.base_currency
    }

    /// The accounts, in the portfolio's order.
    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    /// The instrument with identifier `id`.
    pub fn instrument(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instruments.get(id)
    }

    /// The applied transactions, in the order they were applied.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Every realised gain, in the order realised.
    pub fn realised_gains(&self) -> &[RealisedGain] {
        &self.realised
    }

    /// The holdings after the last transaction.
    pub fn holdings(&self) -> &Holdings {
        &self.holdings
    }

    /// The holdings at the end of `date`.
    pub fn holdings_on(&self, date: Date) -> Holdings {
        let mut holdings = Holdings::default();
        for event in self.events.iter().take_while(|e| e.date <= date) {
            for effect in &event.effects {
                holdings.apply(effect);
            }
        }
        holdings
    }
}

struct Engine<'a> {
    portfolio: &'a PortfolioData,
    market: &'a MarketData,
    instruments: HashMap<InstrumentId, &'a Instrument>,
    holdings: Holdings,
    realised: Vec<RealisedGain>,
    sequence: u64,
}

type Applied = std::result::Result<Event, String>;

impl<'a> Engine<'a> {
    fn apply(&mut self, index: usize, tx: &Transaction) -> Applied {
        let touched = touched_keys(tx);
        let before: Vec<((AccountId, InstrumentId), Vec<Lot>)> = touched
            .iter()
            .map(|key| {
                (
                    key.clone(),
                    self.holdings.lots.get(key).cloned().unwrap_or_default(),
                )
            })
            .collect();
        let mut cash = Vec::new();
        let mut flows = Vec::new();
        self.apply_kind(tx, &mut cash, &mut flows)?;
        let received = match &tx.kind {
            TransactionKind::SpinOff { new_instrument, .. }
            | TransactionKind::Merger { new_instrument, .. }
            | TransactionKind::SymbolChange { new_instrument, .. } => {
                let key = (tx.account.clone(), new_instrument.clone());
                let total = |lots: &[Lot]| -> Decimal { lots.iter().map(|x| x.quantity).sum() };
                let before_qty = before
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(Decimal::ZERO, |(_, l)| total(l));
                self.holdings
                    .lots
                    .get(&key)
                    .map_or(Decimal::ZERO, |l| total(l))
                    - before_qty
            }
            _ => Decimal::ZERO,
        };
        let allocations = allocations(tx, &cash, &flows, received);

        let mut effects = Vec::new();
        for (key, old) in before {
            let new = self.holdings.lots.get(&key).cloned().unwrap_or_default();
            for lot in &new {
                if old.iter().find(|l| l.id == lot.id) != Some(lot) {
                    effects.push(Effect::SetLot { lot: lot.clone() });
                }
            }
            for lot in &old {
                if !new.iter().any(|l| l.id == lot.id) {
                    effects.push(Effect::CloseLot {
                        account: key.0.clone(),
                        instrument: key.1.clone(),
                        lot: lot.id.clone(),
                    });
                }
            }
            if new.is_empty() {
                self.holdings.lots.remove(&key);
            }
        }
        for (currency, amount) in cash {
            let effect = Effect::Cash {
                account: tx.account.clone(),
                currency,
                amount,
            };
            self.holdings.apply(&effect);
            effects.push(effect);
        }
        Ok(Event {
            date: tx.trade_date,
            transaction: tx.id.clone(),
            index,
            effects,
            flows,
            allocations,
        })
    }

    fn apply_kind(
        &mut self,
        tx: &Transaction,
        cash: &mut Vec<(Currency, Decimal)>,
        flows: &mut Vec<Flow>,
    ) -> std::result::Result<(), String> {
        use TransactionKind::*;
        let date = tx.trade_date;
        let account = &tx.account;
        match &tx.kind {
            Buy {
                instrument,
                quantity,
                price,
                fees,
            } => {
                let inst = self.instrument(instrument)?;
                let cost_local = mul3(*quantity, *price, inst.multiplier)? + *fees;
                let cost_base = derived(cost_local * self.to_base(inst.currency, date)?);
                self.open_lot(tx, instrument, *quantity, cost_local, cost_base, date);
                cash.push((inst.currency, -cost_local));
            }
            Sell {
                instrument,
                quantity,
                price,
                fees,
                lot_method,
                lots,
            } => {
                let inst = self.instrument(instrument)?;
                let proceeds = mul3(*quantity, *price, inst.multiplier)? - *fees;
                let relief = self.relieve(account, instrument, *quantity, *lot_method, lots)?;
                let rate = self.to_base(inst.currency, date)?;
                self.realise(tx, instrument, &relief, proceeds, rate);
                cash.push((inst.currency, proceeds));
            }
            TransferIn {
                instrument,
                quantity,
                cost_basis,
                acquired,
            } => {
                let inst = self.instrument(instrument)?;
                let value = mul3(*quantity, self.price(instrument, date)?, inst.multiplier)?;
                let cost_local = cost_basis.unwrap_or(value);
                let cost_base = derived(cost_local * self.to_base(inst.currency, date)?);
                self.open_lot(
                    tx,
                    instrument,
                    *quantity,
                    cost_local,
                    cost_base,
                    acquired.unwrap_or(date),
                );
                flows.push(Flow {
                    account: account.clone(),
                    currency: inst.currency,
                    amount: value,
                    kind: FlowKind::TransferIn,
                });
            }
            TransferOut {
                instrument,
                quantity,
                lot_method,
                lots,
            } => {
                let inst = self.instrument(instrument)?;
                let value = mul3(*quantity, self.price(instrument, date)?, inst.multiplier)?;
                self.relieve(account, instrument, *quantity, *lot_method, lots)?;
                flows.push(Flow {
                    account: account.clone(),
                    currency: inst.currency,
                    amount: -value,
                    kind: FlowKind::TransferOut,
                });
            }
            Deposit { amount } => {
                cash.push((amount.currency(), amount.amount()));
                flows.push(Flow {
                    account: account.clone(),
                    currency: amount.currency(),
                    amount: amount.amount(),
                    kind: FlowKind::Deposit,
                });
            }
            Withdrawal { amount } => {
                cash.push((amount.currency(), -amount.amount()));
                flows.push(Flow {
                    account: account.clone(),
                    currency: amount.currency(),
                    amount: -amount.amount(),
                    kind: FlowKind::Withdrawal,
                });
            }
            Dividend {
                amount,
                withholding_tax,
                ..
            }
            | Interest {
                amount,
                withholding_tax,
                ..
            } => {
                cash.push((amount.currency(), amount.amount() - *withholding_tax));
            }
            Fee { amount } | Tax { amount } => cash.push((amount.currency(), -amount.amount())),
            FxConversion { sell, buy } => {
                cash.push((sell.currency(), -sell.amount()));
                cash.push((buy.currency(), buy.amount()));
            }
            Split {
                instrument,
                from,
                to,
            } => {
                let lots = self.held_lots(account, instrument)?;
                for lot in lots.iter_mut() {
                    lot.quantity = derived(lot.quantity * *to / *from);
                }
            }
            SpinOff {
                instrument,
                new_instrument,
                ratio,
                cost_allocation,
            } => {
                let old_currency = self.instrument(instrument)?.currency;
                let new_currency = self.instrument(new_instrument)?.currency;
                let convert = self.cross(old_currency, new_currency, date)?;
                let mut spun = Vec::new();
                for lot in self.held_lots(account, instrument)?.iter_mut() {
                    let moved_local = derived(lot.cost_local * *cost_allocation);
                    let moved_base = derived(lot.cost_base * *cost_allocation);
                    lot.cost_local -= moved_local;
                    lot.cost_base -= moved_base;
                    spun.push(Lot {
                        id: lot.id.derived(&tx.id),
                        account: account.clone(),
                        instrument: new_instrument.clone(),
                        quantity: derived(lot.quantity * *ratio),
                        cost_local: derived(moved_local * convert),
                        cost_base: moved_base,
                        acquired: lot.acquired,
                        opened_by: lot.opened_by.clone(),
                        sequence: 0,
                    });
                }
                for mut lot in spun {
                    lot.sequence = self.next_sequence();
                    self.holdings
                        .lots
                        .entry((account.clone(), new_instrument.clone()))
                        .or_default()
                        .push(lot);
                }
            }
            Merger {
                instrument,
                new_instrument,
                ratio,
                cash_per_unit,
            } => {
                let inst_currency = self.instrument(instrument)?.currency;
                let rate = self.to_base(inst_currency, date)?;
                let held: Decimal = self
                    .held_lots(account, instrument)?
                    .iter()
                    .map(|l| l.quantity)
                    .sum();
                let cash_total = derived(held * *cash_per_unit);
                if ratio.is_zero() {
                    let relief =
                        self.relieve(account, instrument, held, Some(LotMethod::Fifo), &[])?;
                    self.realise(tx, instrument, &relief, cash_total, rate);
                } else {
                    let new_currency = self.instrument(new_instrument)?.currency;
                    let convert = self.cross(inst_currency, new_currency, date)?;
                    if !cash_total.is_zero() {
                        self.return_capital(tx, instrument, cash_total, rate)?;
                    }
                    let key = (account.clone(), instrument.clone());
                    let mut lots = self.holdings.lots.remove(&key).unwrap_or_default();
                    for lot in &mut lots {
                        lot.instrument = new_instrument.clone();
                        lot.quantity = derived(lot.quantity * *ratio);
                        lot.cost_local = derived(lot.cost_local * convert);
                    }
                    self.holdings
                        .lots
                        .entry((account.clone(), new_instrument.clone()))
                        .or_default()
                        .extend(lots);
                }
                if !cash_total.is_zero() {
                    cash.push((inst_currency, cash_total));
                }
            }
            SymbolChange {
                instrument,
                new_instrument,
            } => {
                if self.instrument(instrument)?.currency
                    != self.instrument(new_instrument)?.currency
                {
                    return Err(format!(
                        "{new_instrument} is priced in another currency than {instrument}; record a merger instead"
                    ));
                }
                self.held_lots(account, instrument)?;
                let mut lots = self
                    .holdings
                    .lots
                    .remove(&(account.clone(), instrument.clone()))
                    .unwrap_or_default();
                for lot in &mut lots {
                    lot.instrument = new_instrument.clone();
                }
                self.holdings
                    .lots
                    .entry((account.clone(), new_instrument.clone()))
                    .or_default()
                    .extend(lots);
            }
            ReturnOfCapital { instrument, amount } => {
                let rate = self.to_base(amount.currency(), date)?;
                self.return_capital(tx, instrument, amount.amount(), rate)?;
                cash.push((amount.currency(), amount.amount()));
            }
        }
        Ok(())
    }

    fn instrument(&self, id: &InstrumentId) -> std::result::Result<&'a Instrument, String> {
        self.instruments
            .get(id)
            .copied()
            .ok_or_else(|| format!("unknown instrument {id}"))
    }

    fn to_base(&self, currency: Currency, date: Date) -> std::result::Result<Decimal, String> {
        self.cross(currency, self.portfolio.base_currency, date)
    }

    fn cross(
        &self,
        from: Currency,
        to: Currency,
        date: Date,
    ) -> std::result::Result<Decimal, String> {
        self.market
            .fx_rate(from, to, date)
            .value()
            .copied()
            .ok_or_else(|| format!("no {from}/{to} rate on or shortly before {date}"))
    }

    fn price(&self, instrument: &InstrumentId, date: Date) -> std::result::Result<Decimal, String> {
        self.market
            .price(instrument, date)
            .value()
            .copied()
            .ok_or_else(|| format!("no price for {instrument} on or shortly before {date}, needed for the market value"))
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    fn open_lot(
        &mut self,
        tx: &Transaction,
        instrument: &InstrumentId,
        quantity: Decimal,
        cost_local: Decimal,
        cost_base: Decimal,
        acquired: Date,
    ) {
        let lot = Lot {
            id: LotId::opened_by(&tx.id),
            account: tx.account.clone(),
            instrument: instrument.clone(),
            quantity,
            cost_local,
            cost_base,
            acquired,
            opened_by: tx.id.clone(),
            sequence: self.next_sequence(),
        };
        self.holdings
            .lots
            .entry((tx.account.clone(), instrument.clone()))
            .or_default()
            .push(lot);
    }

    fn held_lots(
        &mut self,
        account: &AccountId,
        instrument: &InstrumentId,
    ) -> std::result::Result<&mut Vec<Lot>, String> {
        match self
            .holdings
            .lots
            .get_mut(&(account.clone(), instrument.clone()))
        {
            Some(lots) if !lots.is_empty() => Ok(lots),
            _ => Err(format!(
                "no units of {instrument} are held in account {account}"
            )),
        }
    }

    fn relieve(
        &mut self,
        account: &AccountId,
        instrument: &InstrumentId,
        quantity: Decimal,
        method: Option<LotMethod>,
        picks: &[LotPick],
    ) -> std::result::Result<Vec<Relief>, String> {
        let method = match (method, picks.is_empty()) {
            (_, false) => LotMethod::SpecificId,
            (Some(m), true) => m,
            (None, true) => self
                .portfolio
                .account(account)
                .map(|a| a.lot_method)
                .unwrap_or_default(),
        };
        let lots = self
            .holdings
            .lots
            .entry((account.clone(), instrument.clone()))
            .or_default();
        relieve(lots, quantity, method, picks).map_err(|e| match e {
            ReliefError::Insufficient { held } => format!(
                "needs {quantity} units of {instrument}, but account {account} holds {held}"
            ),
            ReliefError::UnknownLot(lot) => {
                format!("lot {lot} is not an open lot of {instrument} in account {account}")
            }
            ReliefError::LotTooSmall { lot, held } => format!("lot {lot} holds only {held} units"),
        })
    }

    fn realise(
        &mut self,
        tx: &Transaction,
        instrument: &InstrumentId,
        relief: &[Relief],
        proceeds: Decimal,
        rate: Decimal,
    ) {
        let total: Decimal = relief.iter().map(|r| r.quantity).sum();
        let mut remaining = proceeds;
        for (i, r) in relief.iter().enumerate() {
            let share = if i + 1 == relief.len() || total.is_zero() {
                remaining
            } else {
                derived(proceeds * r.quantity / total)
            };
            remaining -= share;
            let proceeds_base = derived(share * rate);
            self.realised.push(RealisedGain {
                transaction: tx.id.clone(),
                account: tx.account.clone(),
                instrument: instrument.clone(),
                lot: r.lot.id.clone(),
                quantity: r.quantity,
                acquired: r.lot.acquired,
                disposed: tx.trade_date,
                proceeds_local: share,
                cost_local: r.cost_local,
                gain_local: share - r.cost_local,
                proceeds_base,
                cost_base: r.cost_base,
                gain_base: proceeds_base - r.cost_base,
            });
        }
    }

    /// Reduces the cost basis of the lots of `instrument` by `amount` (in proportion to their
    /// quantities). Any part beyond a lot's basis is realised as a gain.
    fn return_capital(
        &mut self,
        tx: &Transaction,
        instrument: &InstrumentId,
        amount: Decimal,
        rate: Decimal,
    ) -> std::result::Result<(), String> {
        let account = tx.account.clone();
        let lots = self.held_lots(&account, instrument)?;
        let held: Decimal = lots.iter().map(|l| l.quantity).sum();
        let mut remaining = amount;
        let mut gains = Vec::new();
        let count = lots.len();
        for (i, lot) in lots.iter_mut().enumerate() {
            let share = if i + 1 == count {
                remaining
            } else {
                derived(amount * lot.quantity / held)
            };
            remaining -= share;
            let reduction = share.min(lot.cost_local);
            let base_reduction = if lot.cost_local.is_zero() {
                Decimal::ZERO
            } else {
                derived(lot.cost_base * reduction / lot.cost_local)
            };
            lot.cost_local -= reduction;
            lot.cost_base -= base_reduction;
            let excess = share - reduction;
            if excess > Decimal::ZERO {
                gains.push(RealisedGain {
                    transaction: tx.id.clone(),
                    account: account.clone(),
                    instrument: instrument.clone(),
                    lot: lot.id.clone(),
                    quantity: Decimal::ZERO,
                    acquired: lot.acquired,
                    disposed: tx.trade_date,
                    proceeds_local: excess,
                    cost_local: Decimal::ZERO,
                    gain_local: excess,
                    proceeds_base: derived(excess * rate),
                    cost_base: Decimal::ZERO,
                    gain_base: derived(excess * rate),
                });
            }
        }
        self.realised.extend(gains);
        Ok(())
    }
}

/// How `tx` moved value between holdings, from the cash it moved, the external flows it caused
/// and the units of a new instrument it delivered.
fn allocations(
    tx: &Transaction,
    cash: &[(Currency, Decimal)],
    flows: &[Flow],
    received: Decimal,
) -> Vec<Allocation> {
    use TransactionKind::*;
    let trade = |holding: HoldingKey, currency: Currency, amount: Decimal| Allocation::Trade {
        holding,
        currency,
        amount,
    };
    let cash_trades = || cash.iter().map(|&(c, a)| trade(HoldingKey::Cash(c), c, a));
    match &tx.kind {
        Buy { instrument, .. } | Sell { instrument, .. } => {
            let mut out: Vec<Allocation> = cash_trades().collect();
            out.extend(
                cash.iter()
                    .map(|&(c, a)| trade(HoldingKey::Instrument(instrument.clone()), c, -a)),
            );
            out
        }
        Deposit { .. } | Withdrawal { .. } => cash_trades().collect(),
        TransferIn { instrument, .. } | TransferOut { instrument, .. } => flows
            .iter()
            .map(|f| {
                trade(
                    HoldingKey::Instrument(instrument.clone()),
                    f.currency,
                    f.amount,
                )
            })
            .collect(),
        Dividend { instrument, .. } | ReturnOfCapital { instrument, .. } => {
            income(instrument, cash)
        }
        Interest {
            instrument: Some(instrument),
            ..
        } => income(instrument, cash),
        Fee { amount } | Tax { amount } => vec![Allocation::Expense {
            currency: amount.currency(),
            amount: amount.amount(),
        }],
        Interest {
            instrument: None, ..
        }
        | Split { .. } => Vec::new(),
        FxConversion { sell, buy } => vec![
            trade(
                HoldingKey::Cash(sell.currency()),
                sell.currency(),
                -sell.amount(),
            ),
            // The currency bought receives the market value of what was sold; any difference
            // from the amount actually received is that holding's gain or loss.
            trade(
                HoldingKey::Cash(buy.currency()),
                sell.currency(),
                sell.amount(),
            ),
        ],
        SpinOff {
            instrument,
            new_instrument,
            ..
        }
        | SymbolChange {
            instrument,
            new_instrument,
        } => {
            vec![Allocation::Exchange {
                from: instrument.clone(),
                to: new_instrument.clone(),
                quantity: received,
            }]
        }
        Merger {
            instrument,
            new_instrument,
            ratio,
            ..
        } => {
            let mut out = Vec::new();
            if !ratio.is_zero() {
                out.push(Allocation::Exchange {
                    from: instrument.clone(),
                    to: new_instrument.clone(),
                    quantity: received,
                });
            }
            for &(c, a) in cash {
                out.push(trade(HoldingKey::Instrument(instrument.clone()), c, -a));
                out.push(trade(HoldingKey::Cash(c), c, a));
            }
            out
        }
    }
}

fn income(instrument: &InstrumentId, cash: &[(Currency, Decimal)]) -> Vec<Allocation> {
    cash.iter()
        .flat_map(|&(c, a)| {
            [
                Allocation::Income {
                    holding: HoldingKey::Instrument(instrument.clone()),
                    currency: c,
                    amount: a,
                },
                Allocation::Trade {
                    holding: HoldingKey::Cash(c),
                    currency: c,
                    amount: a,
                },
            ]
        })
        .collect()
}

fn touched_keys(tx: &Transaction) -> Vec<(AccountId, InstrumentId)> {
    use TransactionKind::*;
    let account = tx.account.clone();
    match &tx.kind {
        SpinOff {
            instrument,
            new_instrument,
            ..
        }
        | Merger {
            instrument,
            new_instrument,
            ..
        }
        | SymbolChange {
            instrument,
            new_instrument,
        } => vec![
            (account.clone(), instrument.clone()),
            (account, new_instrument.clone()),
        ],
        kind => kind
            .instrument()
            .map(|i| vec![(account, i.clone())])
            .unwrap_or_default(),
    }
}

fn mul3(a: Decimal, b: Decimal, c: Decimal) -> std::result::Result<Decimal, String> {
    a.checked_mul(b)
        .and_then(|x| x.checked_mul(c))
        .ok_or_else(|| "the amount is too large".to_owned())
}
