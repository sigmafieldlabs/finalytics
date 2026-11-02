//! The holdings report: positions, lots, cash, allocation, concentration, realised gains and
//! income.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Decimal, Result, ValidationReport};
use finalytics_ledger::{
    AccountId, Allocation, Books, CashValue, HoldingKey, InstrumentId, InstrumentKind, LotId,
    MarketData, PositionValue,
};

use crate::context::{Context, Provenance, ReportKind, ReportOptions, Source, label, to_f64};
use crate::shapes::{Breakdown, Slice};

/// One holding, added up across accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingRow {
    /// The holding.
    pub holding: HoldingKey,
    /// The instrument's name, if it has one.
    pub name: Option<String>,
    /// The instrument's kind; `None` for cash.
    pub kind: Option<InstrumentKind>,
    /// The holding's currency.
    pub currency: Currency,
    /// The quantity held, or the cash balance.
    pub quantity: Decimal,
    /// The price per unit, in the instrument's currency; `None` for cash.
    pub price: Option<Decimal>,
    /// The value in the holding's currency.
    pub value_local: Decimal,
    /// The value in the base currency.
    pub value_base: Decimal,
    /// The cost basis in the instrument's currency; `None` for cash.
    pub cost_local: Option<Decimal>,
    /// The cost basis in the base currency; `None` for cash.
    pub cost_base: Option<Decimal>,
    /// `value_local - cost_local`.
    pub unrealised_local: Option<Decimal>,
    /// `value_base - cost_base`, which includes the currency effect.
    pub unrealised_base: Option<Decimal>,
    /// `unrealised_base / cost_base`, when the cost is above zero.
    pub unrealised_return: Option<f64>,
    /// The share of the portfolio's value.
    pub weight: f64,
    /// The instrument's classifications.
    pub classifications: BTreeMap<String, String>,
}

/// One account's holdings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountHoldings {
    /// The account.
    pub account: AccountId,
    /// The account's name, if it has one.
    pub name: Option<String>,
    /// The account's classifications.
    pub classifications: BTreeMap<String, String>,
    /// The account's value in the base currency.
    pub value_base: Decimal,
    /// The share of the portfolio's value.
    pub weight: f64,
    /// Its positions.
    pub positions: Vec<PositionValue>,
    /// Its cash balances.
    pub cash: Vec<CashValue>,
}

/// One open tax lot, valued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LotRow {
    /// The account.
    pub account: AccountId,
    /// The instrument.
    pub instrument: InstrumentId,
    /// The lot.
    pub lot: LotId,
    /// The acquisition date.
    pub acquired: Date,
    /// The quantity.
    pub quantity: Decimal,
    /// The cost basis in the base currency.
    pub cost_base: Decimal,
    /// The value in the base currency.
    pub value_base: Decimal,
    /// `value_base - cost_base`.
    pub unrealised_base: Decimal,
    /// `unrealised_base / cost_base`, when the cost is above zero.
    pub unrealised_return: Option<f64>,
    /// Calendar days held to the report's date.
    pub holding_days: i32,
    /// Whether it has been held for the long-term period or longer.
    pub long_term: bool,
}

/// How concentrated the portfolio's instruments are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Concentration {
    /// The largest instrument holdings, largest first, with their share of the portfolio.
    pub largest: Vec<Slice>,
    /// The share of the portfolio in its five largest instruments.
    pub top_five: f64,
    /// The share of the portfolio in its ten largest instruments.
    pub top_ten: f64,
    /// The Herfindahl–Hirschman index of the instruments' shares of the value invested in
    /// instruments, from 1/n (equal) to 1 (one instrument).
    pub herfindahl: Option<f64>,
    /// `1 / herfindahl`: the number of equal holdings that would be as concentrated.
    pub effective_holdings: Option<f64>,
}

/// Gains realised up to the report's date, in the base currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RealisedSummary {
    /// Every realised gain.
    pub total: Decimal,
    /// Gains on lots held for less than the long-term period.
    pub short_term: Decimal,
    /// Gains on lots held for the long-term period or longer.
    pub long_term: Decimal,
    /// By calendar year of disposal.
    pub by_year: Vec<(i32, Decimal)>,
    /// By instrument.
    pub by_instrument: Vec<(InstrumentId, Decimal)>,
}

/// Income received up to the report's date, net of tax withheld, in the base currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IncomeSummary {
    /// All income.
    pub total: Decimal,
    /// By calendar year.
    pub by_year: Vec<(i32, Decimal)>,
    /// By holding: the instrument that paid it, or cash for interest on cash.
    pub by_holding: Vec<(HoldingKey, Decimal)>,
}

/// The holdings report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HoldingsReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The portfolio's value in the base currency.
    pub value: Decimal,
    /// Each holding, added up across accounts, largest first.
    pub holdings: Vec<HoldingRow>,
    /// Each account's holdings, in the portfolio's order.
    pub accounts: Vec<AccountHoldings>,
    /// The open tax lots, by account and instrument.
    pub lots: Vec<LotRow>,
    /// The value split by holding, currency, instrument kind, account and each scheme of the
    /// options.
    pub breakdowns: Vec<Breakdown>,
    /// How concentrated the instruments are.
    pub concentration: Concentration,
    /// Gains realised to date.
    pub realised: RealisedSummary,
    /// Income received to date.
    pub income: IncomeSummary,
}

/// The holdings on the report's date: positions, lots and cash, by holding and by account;
/// allocation by every dimension; concentration; and realised gains and income to date.
pub fn holdings_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<HoldingsReport> {
    let cx = Context::new(books, market, source, options)?;
    let valuation = books.value_on(cx.as_of, market)?;
    let mut warnings = valuation.warnings.clone();
    let total = valuation.total_base;
    let total_f = to_f64(total);
    let weight = |v: Decimal| {
        if total_f > 0.0 {
            to_f64(v) / total_f
        } else {
            0.0
        }
    };

    let mut rows: BTreeMap<HoldingKey, HoldingRow> = BTreeMap::new();
    for p in &valuation.positions {
        let instrument = books.instrument(&p.instrument);
        let key = HoldingKey::Instrument(p.instrument.clone());
        let row = rows.entry(key.clone()).or_insert_with(|| HoldingRow {
            holding: key,
            name: instrument.and_then(|i| i.name.clone()),
            kind: instrument.map(|i| i.kind),
            currency: instrument.map_or(cx.base, |i| i.currency),
            quantity: Decimal::ZERO,
            price: Some(p.price),
            value_local: Decimal::ZERO,
            value_base: Decimal::ZERO,
            cost_local: Some(Decimal::ZERO),
            cost_base: Some(Decimal::ZERO),
            unrealised_local: Some(Decimal::ZERO),
            unrealised_base: Some(Decimal::ZERO),
            unrealised_return: None,
            weight: 0.0,
            classifications: instrument
                .map(|i| i.classifications.clone())
                .unwrap_or_default(),
        });
        row.quantity += p.quantity;
        row.value_local += p.market_value_local;
        row.value_base += p.market_value_base;
        let add = |x: &mut Option<Decimal>, v: Decimal| *x = x.map(|a| a + v);
        add(&mut row.cost_local, p.cost_local);
        add(&mut row.cost_base, p.cost_base);
        add(&mut row.unrealised_local, p.unrealised_local);
        add(&mut row.unrealised_base, p.unrealised_base);
    }
    // Accounts with no cash in a currency are left out; a holding of nothing is clutter.
    for c in valuation.cash.iter().filter(|c| !c.amount.is_zero()) {
        let key = HoldingKey::Cash(c.currency);
        let row = rows.entry(key.clone()).or_insert_with(|| HoldingRow {
            holding: key,
            name: None,
            kind: None,
            currency: c.currency,
            quantity: Decimal::ZERO,
            price: None,
            value_local: Decimal::ZERO,
            value_base: Decimal::ZERO,
            cost_local: None,
            cost_base: None,
            unrealised_local: None,
            unrealised_base: None,
            unrealised_return: None,
            weight: 0.0,
            classifications: BTreeMap::new(),
        });
        row.quantity += c.amount;
        row.value_local += c.amount;
        row.value_base += c.value_base;
    }
    let mut holdings: Vec<HoldingRow> = rows
        .into_values()
        .map(|mut r| {
            r.weight = weight(r.value_base);
            r.unrealised_return = match (r.unrealised_base, r.cost_base) {
                (Some(u), Some(c)) if c > Decimal::ZERO => Some(to_f64(u) / to_f64(c)),
                _ => None,
            };
            r
        })
        .collect();
    holdings.sort_by_key(|h| std::cmp::Reverse(h.value_base));

    let accounts: Vec<AccountHoldings> = books
        .accounts()
        .iter()
        .map(|a| {
            let positions: Vec<PositionValue> = valuation
                .positions
                .iter()
                .filter(|p| p.account == a.id)
                .cloned()
                .collect();
            let cash: Vec<CashValue> = valuation
                .cash
                .iter()
                .filter(|c| c.account == a.id && !c.amount.is_zero())
                .cloned()
                .collect();
            let value: Decimal = positions
                .iter()
                .map(|p| p.market_value_base)
                .sum::<Decimal>()
                + cash.iter().map(|c| c.value_base).sum::<Decimal>();
            AccountHoldings {
                account: a.id.clone(),
                name: a.name.clone(),
                classifications: a.classifications.clone(),
                value_base: value,
                weight: weight(value),
                positions,
                cash,
            }
        })
        .collect();

    let held = books.holdings_on(cx.as_of);
    let lots: Vec<LotRow> = held
        .lots()
        .map(|lot| {
            let position = valuation
                .positions
                .iter()
                .find(|p| p.account == lot.account && p.instrument == lot.instrument);
            let unit_base = position.map_or(Decimal::ZERO, |p| {
                if p.quantity.is_zero() {
                    Decimal::ZERO
                } else {
                    p.market_value_base / p.quantity
                }
            });
            let value_base = unit_base * lot.quantity;
            let unrealised = value_base - lot.cost_base;
            let holding_days = lot.acquired.days_until(cx.as_of);
            LotRow {
                account: lot.account.clone(),
                instrument: lot.instrument.clone(),
                lot: lot.id.clone(),
                acquired: lot.acquired,
                quantity: lot.quantity,
                cost_base: lot.cost_base,
                value_base,
                unrealised_base: unrealised,
                unrealised_return: (lot.cost_base > Decimal::ZERO)
                    .then(|| to_f64(unrealised) / to_f64(lot.cost_base)),
                holding_days,
                long_term: holding_days >= options.long_term_after_days,
            }
        })
        .collect();

    let breakdown = |by: &str, name_of: &dyn Fn(&HoldingRow) -> String| -> Breakdown {
        let mut values: BTreeMap<String, Decimal> = BTreeMap::new();
        for h in &holdings {
            *values.entry(name_of(h)).or_default() += h.value_base;
        }
        let mut slices: Vec<Slice> = values
            .into_iter()
            .map(|(name, v)| Slice {
                name,
                value: to_f64(v),
                weight: weight(v),
            })
            .collect();
        slices.sort_by(|a, b| {
            b.value
                .total_cmp(&a.value)
                .then_with(|| a.name.cmp(&b.name))
        });
        Breakdown {
            by: by.to_owned(),
            slices,
        }
    };
    let mut breakdowns = vec![
        breakdown("holding", &|h| label(&h.holding)),
        breakdown("currency", &|h| h.currency.to_string()),
        breakdown("kind", &|h| match h.kind {
            None => "cash".to_owned(),
            Some(kind) => serde_json::to_value(kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
        }),
    ];
    let mut by_account = Breakdown {
        by: "account".to_owned(),
        slices: accounts
            .iter()
            .map(|a| Slice {
                name: a.account.to_string(),
                value: to_f64(a.value_base),
                weight: a.weight,
            })
            .collect(),
    };
    by_account.slices.sort_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then_with(|| a.name.cmp(&b.name))
    });
    breakdowns.push(by_account);
    for scheme in &options.group_by {
        breakdowns.push(breakdown(scheme, &|h| {
            crate::context::group_of(books, &h.holding, scheme)
        }));
    }

    let instruments: Vec<&HoldingRow> = holdings
        .iter()
        .filter(|h| matches!(h.holding, HoldingKey::Instrument(_)))
        .collect();
    let invested: f64 = instruments.iter().map(|h| to_f64(h.value_base)).sum();
    let share_of_top = |n: usize| instruments.iter().take(n).map(|h| h.weight).sum::<f64>();
    let herfindahl = (invested > 0.0).then(|| {
        instruments
            .iter()
            .map(|h| (to_f64(h.value_base) / invested).powi(2))
            .sum::<f64>()
    });
    let concentration = Concentration {
        largest: instruments
            .iter()
            .take(options.top)
            .map(|h| Slice {
                name: label(&h.holding),
                value: to_f64(h.value_base),
                weight: h.weight,
            })
            .collect(),
        top_five: share_of_top(5),
        top_ten: share_of_top(10),
        herfindahl,
        effective_holdings: herfindahl.filter(|h| *h > 0.0).map(|h| 1.0 / h),
    };

    let mut realised = RealisedSummary {
        total: Decimal::ZERO,
        short_term: Decimal::ZERO,
        long_term: Decimal::ZERO,
        by_year: Vec::new(),
        by_instrument: Vec::new(),
    };
    let mut gains_by_year: BTreeMap<i32, Decimal> = BTreeMap::new();
    let mut gains_by_instrument: BTreeMap<InstrumentId, Decimal> = BTreeMap::new();
    for g in books
        .realised_gains()
        .iter()
        .filter(|g| g.disposed <= cx.as_of)
    {
        realised.total += g.gain_base;
        if g.acquired.days_until(g.disposed) >= options.long_term_after_days {
            realised.long_term += g.gain_base;
        } else {
            realised.short_term += g.gain_base;
        }
        *gains_by_year.entry(g.disposed.year()).or_default() += g.gain_base;
        *gains_by_instrument.entry(g.instrument.clone()).or_default() += g.gain_base;
    }
    realised.by_year = gains_by_year.into_iter().collect();
    realised.by_instrument = gains_by_instrument.into_iter().collect();

    let income = income(books, market, cx.as_of, &mut warnings);
    Ok(HoldingsReport {
        provenance: cx.provenance(ReportKind::Holdings, warnings),
        value: total,
        holdings,
        accounts,
        lots,
        breakdowns,
        concentration,
        realised,
        income,
    })
}

/// Income received up to `as_of`, converted at each payment's date.
fn income(
    books: &Books,
    market: &MarketData,
    as_of: Date,
    warnings: &mut ValidationReport,
) -> IncomeSummary {
    let base = books.base_currency();
    let mut total = Decimal::ZERO;
    let mut by_year: BTreeMap<i32, Decimal> = BTreeMap::new();
    let mut by_holding: BTreeMap<HoldingKey, Decimal> = BTreeMap::new();
    for event in books.events().iter().filter(|e| e.date <= as_of) {
        for allocation in &event.allocations {
            let Allocation::Income {
                holding,
                currency,
                amount,
            } = allocation
            else {
                continue;
            };
            let Some(rate) = market.fx_rate(*currency, base, event.date).value().copied() else {
                warnings.warning(
                    format!("fx.{currency}{base}"),
                    format!(
                        "no rate on or shortly before {} for income of {amount} {currency}; \
                         left out of the income summary",
                        event.date
                    ),
                );
                continue;
            };
            let value = *amount * rate;
            total += value;
            *by_year.entry(event.date.year()).or_default() += value;
            *by_holding.entry(holding.clone()).or_default() += value;
        }
    }
    IncomeSummary {
        total,
        by_year: by_year.into_iter().collect(),
        by_holding: by_holding.into_iter().collect(),
    }
}
