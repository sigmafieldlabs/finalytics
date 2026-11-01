//! A portfolio and its structural validation.

use std::collections::{BTreeMap, BTreeSet};

use finalytics_core::{Currency, Decimal, Money, ValidationReport};
use serde::{Deserialize, Serialize};

use crate::{
    Account, AccountId, Instrument, InstrumentId, LotMethod, LotPick, Transaction, TransactionKind,
};

/// Everything Finalytics needs to keep a portfolio's books: its base currency, accounts,
/// instruments and transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portfolio {
    /// The currency results are reported in.
    pub base_currency: Currency,
    /// The accounts.
    pub accounts: Vec<Account>,
    /// The instruments the transactions refer to.
    pub instruments: Vec<Instrument>,
    /// The transactions, in any order; they are applied by trade date, and in the order given
    /// within a date.
    pub transactions: Vec<Transaction>,
}

impl Portfolio {
    /// An empty portfolio reporting in `base_currency`.
    pub fn new(base_currency: Currency) -> Portfolio {
        Portfolio {
            base_currency,
            accounts: Vec::new(),
            instruments: Vec::new(),
            transactions: Vec::new(),
        }
    }

    /// The instrument with identifier `id`.
    pub fn instrument(&self, id: &InstrumentId) -> Option<&Instrument> {
        self.instruments.iter().find(|i| &i.id == id)
    }

    /// The account with identifier `id`.
    pub fn account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.iter().find(|a| &a.id == id)
    }

    /// Checks the portfolio's structure without applying its transactions: unique identifiers,
    /// references that resolve, and amounts with sensible signs and currencies. Problems found
    /// only by applying the transactions in order, such as selling more than is held, are
    /// reported when the books are built.
    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        let mut accounts = BTreeSet::new();
        for (i, account) in self.accounts.iter().enumerate() {
            if !accounts.insert(&account.id) {
                report.error(
                    format!("accounts[{i}].id"),
                    format!("duplicate account {}", account.id),
                );
            }
        }

        let mut instruments: BTreeMap<&InstrumentId, &Instrument> = BTreeMap::new();
        for (i, instrument) in self.instruments.iter().enumerate() {
            if instruments.insert(&instrument.id, instrument).is_some() {
                report.error(
                    format!("instruments[{i}].id"),
                    format!("duplicate instrument {}", instrument.id),
                );
            }
            if instrument.multiplier <= Decimal::ZERO {
                report.error(
                    format!("instruments[{i}].multiplier"),
                    "must be greater than zero",
                );
            }
        }

        let mut transactions = BTreeSet::new();
        for (i, tx) in self.transactions.iter().enumerate() {
            let at = |field: &str| format!("transactions[{i}].{field}");
            if !transactions.insert(&tx.id) {
                report.error(at("id"), format!("duplicate transaction {}", tx.id));
            }
            if !accounts.contains(&tx.account) {
                report.error(at("account"), format!("unknown account {}", tx.account));
            }
            if let Some(settle) = tx.settle_date
                && settle < tx.trade_date
            {
                report.error(at("settle_date"), "is before the trade date");
            }
            let mut check = Checker {
                report: &mut report,
                at: &at,
                instruments: &instruments,
            };
            check.kind(&tx.kind);
        }
        report
    }
}

struct Checker<'a, F: Fn(&str) -> String> {
    report: &'a mut ValidationReport,
    at: &'a F,
    instruments: &'a BTreeMap<&'a InstrumentId, &'a Instrument>,
}

impl<F: Fn(&str) -> String> Checker<'_, F> {
    fn kind(&mut self, kind: &TransactionKind) {
        use TransactionKind::*;
        match kind {
            Buy {
                instrument,
                quantity,
                price,
                fees,
            } => {
                self.instrument("instrument", instrument);
                self.positive("quantity", *quantity);
                self.non_negative("price", *price);
                self.non_negative("fees", *fees);
            }
            Sell {
                instrument,
                quantity,
                price,
                fees,
                lot_method,
                lots,
            } => {
                self.instrument("instrument", instrument);
                self.positive("quantity", *quantity);
                self.non_negative("price", *price);
                self.non_negative("fees", *fees);
                self.lot_picks(*quantity, *lot_method, lots);
            }
            TransferOut {
                instrument,
                quantity,
                lot_method,
                lots,
            } => {
                self.instrument("instrument", instrument);
                self.positive("quantity", *quantity);
                self.lot_picks(*quantity, *lot_method, lots);
            }
            TransferIn {
                instrument,
                quantity,
                cost_basis,
                ..
            } => {
                self.instrument("instrument", instrument);
                self.positive("quantity", *quantity);
                if let Some(basis) = cost_basis {
                    self.non_negative("cost_basis", *basis);
                }
            }
            Deposit { amount } | Withdrawal { amount } | Fee { amount } | Tax { amount } => {
                self.positive("amount", amount.amount());
            }
            Dividend {
                instrument,
                amount,
                withholding_tax,
            } => {
                self.instrument("instrument", instrument);
                self.income(amount, *withholding_tax);
            }
            Interest {
                instrument,
                amount,
                withholding_tax,
            } => {
                if let Some(instrument) = instrument {
                    self.instrument("instrument", instrument);
                }
                self.income(amount, *withholding_tax);
            }
            FxConversion { sell, buy } => {
                self.positive("sell.amount", sell.amount());
                self.positive("buy.amount", buy.amount());
                if sell.currency() == buy.currency() {
                    self.error("buy.currency", "must differ from the currency sold");
                }
            }
            Split {
                instrument,
                from,
                to,
            } => {
                self.instrument("instrument", instrument);
                self.positive("from", *from);
                self.positive("to", *to);
            }
            SpinOff {
                instrument,
                new_instrument,
                ratio,
                cost_allocation,
            } => {
                self.instrument("instrument", instrument);
                self.new_instrument(instrument, new_instrument);
                self.positive("ratio", *ratio);
                if *cost_allocation < Decimal::ZERO || *cost_allocation > Decimal::ONE {
                    self.error("cost_allocation", "must be between 0 and 1");
                }
            }
            Merger {
                instrument,
                new_instrument,
                ratio,
                cash_per_unit,
            } => {
                self.instrument("instrument", instrument);
                self.non_negative("ratio", *ratio);
                self.non_negative("cash_per_unit", *cash_per_unit);
                if ratio.is_zero() && cash_per_unit.is_zero() {
                    self.error("ratio", "a merger needs new units, cash, or both");
                }
                if !ratio.is_zero() {
                    self.new_instrument(instrument, new_instrument);
                }
            }
            SymbolChange {
                instrument,
                new_instrument,
            } => {
                self.instrument("instrument", instrument);
                self.new_instrument(instrument, new_instrument);
            }
            ReturnOfCapital { instrument, amount } => {
                self.instrument("instrument", instrument);
                self.positive("amount", amount.amount());
                if let Some(found) = self.instruments.get(instrument)
                    && found.currency != amount.currency()
                {
                    self.error(
                        "amount.currency",
                        format!("must be the instrument's currency, {}", found.currency),
                    );
                }
            }
        }
    }

    fn error(&mut self, field: &str, message: impl Into<String>) {
        self.report.error((self.at)(field), message);
    }

    fn instrument(&mut self, field: &str, id: &InstrumentId) {
        if !self.instruments.contains_key(id) {
            self.error(field, format!("unknown instrument {id}"));
        }
    }

    fn new_instrument(&mut self, old: &InstrumentId, new: &InstrumentId) {
        if old == new {
            self.error("new_instrument", "must differ from the instrument held");
        } else {
            self.instrument("new_instrument", new);
        }
    }

    fn positive(&mut self, field: &str, value: Decimal) {
        if value <= Decimal::ZERO {
            self.error(field, "must be greater than zero");
        }
    }

    fn non_negative(&mut self, field: &str, value: Decimal) {
        if value < Decimal::ZERO {
            self.error(field, "must not be negative");
        }
    }

    fn income(&mut self, amount: &Money, withholding_tax: Decimal) {
        self.positive("amount", amount.amount());
        self.non_negative("withholding_tax", withholding_tax);
        if withholding_tax > amount.amount() {
            self.error("withholding_tax", "is larger than the gross amount");
        }
    }

    fn lot_picks(&mut self, quantity: Decimal, method: Option<LotMethod>, lots: &[LotPick]) {
        if method == Some(LotMethod::SpecificId) && lots.is_empty() {
            self.error("lots", "specific identification needs the lots to close");
        }
        if !lots.is_empty() {
            if method.is_some_and(|m| m != LotMethod::SpecificId) {
                self.error("lots", "lots are named only with specific identification");
            }
            let mut total = Decimal::ZERO;
            for (k, pick) in lots.iter().enumerate() {
                if pick.quantity <= Decimal::ZERO {
                    self.error(&format!("lots[{k}].quantity"), "must be greater than zero");
                }
                total += pick.quantity;
            }
            if total != quantity {
                self.error(
                    "lots",
                    format!("the named lots add up to {total}, not {quantity}"),
                );
            }
        }
    }
}
