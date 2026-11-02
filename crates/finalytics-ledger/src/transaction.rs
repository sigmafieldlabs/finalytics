//! Transactions and corporate actions.

use finalytics_core::{Date, Decimal, Money};
use serde::{Deserialize, Serialize};

use crate::{AccountId, InstrumentId, LotId, LotMethod, TransactionId};

/// One event in the life of a portfolio.
///
/// Transactions are applied in trade-date order; transactions on the same date are applied in the
/// order given. Positions are booked on the trade date; the settlement date is recorded for cash
/// reporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Transaction {
    /// The caller's identifier, unique within the portfolio.
    pub id: TransactionId,
    /// The account the transaction belongs to.
    pub account: AccountId,
    /// The date the transaction takes effect for positions and performance.
    pub trade_date: Date,
    /// The date cash settles, if different from the trade date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_date: Option<Date>,
    /// What happened.
    #[serde(flatten)]
    pub kind: TransactionKind,
    /// A free-text note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Transaction {
    /// A transaction that settles on its trade date and has no note.
    pub fn new(
        id: TransactionId,
        account: AccountId,
        trade_date: Date,
        kind: TransactionKind,
    ) -> Transaction {
        Transaction {
            id,
            account,
            trade_date,
            settle_date: None,
            kind,
            note: None,
        }
    }
}

/// What a transaction does. Amounts of instruments are in the instrument's currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TransactionKind {
    /// Buy `quantity` units at `price` per unit, paying `fees` on top.
    Buy {
        /// The instrument bought.
        instrument: InstrumentId,
        /// The number of units, greater than zero.
        quantity: Decimal,
        /// The price per unit, zero or more.
        price: Decimal,
        /// Commissions and other costs of the trade, added to the cost basis.
        #[serde(default)]
        fees: Decimal,
    },
    /// Sell `quantity` units at `price` per unit, with `fees` deducted from the proceeds.
    Sell {
        /// The instrument sold.
        instrument: InstrumentId,
        /// The number of units, greater than zero.
        quantity: Decimal,
        /// The price per unit, zero or more.
        price: Decimal,
        /// Commissions and other costs, deducted from the proceeds.
        #[serde(default)]
        fees: Decimal,
        /// Overrides the account's lot method for this sale.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lot_method: Option<LotMethod>,
        /// The lots to close, for specific identification.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lots: Vec<LotPick>,
    },
    /// Cash paid into the account from outside the portfolio.
    Deposit {
        /// The amount, greater than zero.
        amount: Money,
    },
    /// Cash taken out of the portfolio.
    Withdrawal {
        /// The amount, greater than zero.
        amount: Money,
    },
    /// Units received from outside the portfolio.
    TransferIn {
        /// The instrument received.
        instrument: InstrumentId,
        /// The number of units, greater than zero.
        quantity: Decimal,
        /// The total cost basis carried over, in the instrument's currency. When absent, the
        /// market value on the trade date is used.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cost_basis: Option<Decimal>,
        /// The original acquisition date, when the holding period carries over.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        acquired: Option<Date>,
    },
    /// Units sent out of the portfolio. No gain is realised.
    TransferOut {
        /// The instrument sent.
        instrument: InstrumentId,
        /// The number of units, greater than zero.
        quantity: Decimal,
        /// Overrides the account's lot method for this transfer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lot_method: Option<LotMethod>,
        /// The lots to send, for specific identification.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lots: Vec<LotPick>,
    },
    /// A dividend received in cash.
    Dividend {
        /// The instrument that paid it.
        instrument: InstrumentId,
        /// The gross amount.
        amount: Money,
        /// Tax withheld at source, in the same currency.
        #[serde(default)]
        withholding_tax: Decimal,
    },
    /// Interest received in cash, from an instrument or on the cash balance.
    Interest {
        /// The instrument that paid it, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instrument: Option<InstrumentId>,
        /// The gross amount.
        amount: Money,
        /// Tax withheld at source, in the same currency.
        #[serde(default)]
        withholding_tax: Decimal,
    },
    /// A fee not tied to a trade, such as a custody or management fee.
    Fee {
        /// The amount, greater than zero.
        amount: Money,
    },
    /// A tax not tied to a trade.
    Tax {
        /// The amount, greater than zero.
        amount: Money,
    },
    /// Cash converted from one currency to another.
    FxConversion {
        /// The amount given up.
        sell: Money,
        /// The amount received.
        buy: Money,
    },
    /// A share split: every `from` units become `to` units, with cost basis unchanged. A reverse
    /// split has `to` below `from`.
    Split {
        /// The instrument split.
        instrument: InstrumentId,
        /// Units before.
        from: Decimal,
        /// Units after.
        to: Decimal,
    },
    /// A spin-off: holders receive `ratio` units of a new instrument per unit held, and a share
    /// `cost_allocation` (between 0 and 1) of the cost basis moves to the new units.
    SpinOff {
        /// The instrument held.
        instrument: InstrumentId,
        /// The new instrument received.
        new_instrument: InstrumentId,
        /// New units per unit held.
        ratio: Decimal,
        /// The share of cost basis moved to the new units.
        cost_allocation: Decimal,
    },
    /// A merger or acquisition: each unit held becomes `ratio` units of `new_instrument`, plus
    /// `cash_per_unit` in cash, which reduces the cost basis (gains beyond the basis are
    /// realised).
    Merger {
        /// The instrument held.
        instrument: InstrumentId,
        /// The instrument received.
        new_instrument: InstrumentId,
        /// New units per unit held; zero for an all-cash takeover.
        ratio: Decimal,
        /// Cash per unit held, in the held instrument's currency.
        #[serde(default)]
        cash_per_unit: Decimal,
    },
    /// The instrument's identifier changes; lots carry over unchanged.
    SymbolChange {
        /// The old identifier.
        instrument: InstrumentId,
        /// The new identifier.
        new_instrument: InstrumentId,
    },
    /// A return of capital paid in cash, which reduces the cost basis (amounts beyond the basis
    /// are realised as gains).
    ReturnOfCapital {
        /// The instrument that paid it.
        instrument: InstrumentId,
        /// The total amount, in the instrument's currency.
        amount: Money,
    },
}

/// A lot named by a sale or transfer, for specific identification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LotPick {
    /// The lot.
    pub lot: LotId,
    /// The number of units to take from it.
    pub quantity: Decimal,
}

impl TransactionKind {
    /// The instrument the transaction concerns, if any.
    pub fn instrument(&self) -> Option<&InstrumentId> {
        use TransactionKind::*;
        match self {
            Buy { instrument, .. }
            | Sell { instrument, .. }
            | TransferIn { instrument, .. }
            | TransferOut { instrument, .. }
            | Dividend { instrument, .. }
            | Split { instrument, .. }
            | SpinOff { instrument, .. }
            | Merger { instrument, .. }
            | SymbolChange { instrument, .. }
            | ReturnOfCapital { instrument, .. } => Some(instrument),
            Interest { instrument, .. } => instrument.as_ref(),
            Deposit { .. } | Withdrawal { .. } | Fee { .. } | Tax { .. } | FxConversion { .. } => {
                None
            }
        }
    }

    /// The name used for this kind in serialised form, for example `"buy"`.
    pub fn name(&self) -> &'static str {
        use TransactionKind::*;
        match self {
            Buy { .. } => "buy",
            Sell { .. } => "sell",
            Deposit { .. } => "deposit",
            Withdrawal { .. } => "withdrawal",
            TransferIn { .. } => "transfer_in",
            TransferOut { .. } => "transfer_out",
            Dividend { .. } => "dividend",
            Interest { .. } => "interest",
            Fee { .. } => "fee",
            Tax { .. } => "tax",
            FxConversion { .. } => "fx_conversion",
            Split { .. } => "split",
            SpinOff { .. } => "spin_off",
            Merger { .. } => "merger",
            SymbolChange { .. } => "symbol_change",
            ReturnOfCapital { .. } => "return_of_capital",
        }
    }
}
