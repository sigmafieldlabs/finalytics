//! Reading instruments, transactions, prices and FX rates from CSV.
//!
//! Every reader checks the whole file and reports every problem at once, located by line and
//! column, for example `line 15 (quantity)`. Column names are matched without regard to case or
//! surrounding spaces. Dates are ISO 8601 (`2026-11-02`); numbers use a dot for decimals and no
//! thousands separators.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use finalytics_core::{
    Currency, Date, Decimal, Error, Money, Result, TimeSeries, ValidationReport,
};
use finalytics_ledger::{
    Account, AccountId, Instrument, InstrumentId, InstrumentKind, LotId, LotMethod, LotPick,
    Portfolio, Transaction, TransactionId, TransactionKind,
};

/// Options for reading CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvOptions {
    /// The field delimiter, a comma by default.
    pub delimiter: u8,
}

impl Default for CsvOptions {
    fn default() -> Self {
        CsvOptions { delimiter: b',' }
    }
}

/// An FX series read from CSV: units of `quote` per unit of `base`.
#[derive(Debug, Clone, PartialEq)]
pub struct FxSeries {
    /// The base currency.
    pub base: Currency,
    /// The quote currency.
    pub quote: Currency,
    /// The rates.
    pub rates: TimeSeries<Decimal>,
}

/// Points read for one series, before sorting: date, value and the line they came from.
type Points = Vec<(Date, Decimal, u64)>;

/// A parsed table: column positions and rows with their line numbers.
struct Table {
    columns: HashMap<String, usize>,
    names: Vec<String>,
    rows: Vec<(u64, Vec<String>)>,
}

impl Table {
    fn read(text: &str, options: CsvOptions, report: &mut ValidationReport) -> Option<Table> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(options.delimiter)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());
        let names: Vec<String> = match reader.headers() {
            Ok(headers) => headers.iter().map(|h| h.to_ascii_lowercase()).collect(),
            Err(e) => {
                report.error("line 1", format!("cannot read the header: {e}"));
                return None;
            }
        };
        let mut columns = HashMap::new();
        for (i, name) in names.iter().enumerate() {
            if columns.insert(name.clone(), i).is_some() {
                report.error("line 1", format!("column {name:?} appears twice"));
            }
        }
        let mut rows = Vec::new();
        for record in reader.records() {
            match record {
                Ok(record) => {
                    let line = record.position().map_or(0, |p| p.line());
                    if record.iter().all(str::is_empty) {
                        continue;
                    }
                    rows.push((line, record.iter().map(str::to_owned).collect()));
                }
                Err(e) => {
                    let line = e.position().map_or(0, |p| p.line());
                    report.error(format!("line {line}"), format!("cannot read the row: {e}"));
                }
            }
        }
        Some(Table {
            columns,
            names,
            rows,
        })
    }

    fn require(&self, names: &[&str], report: &mut ValidationReport) -> bool {
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !self.columns.contains_key(*n))
            .collect();
        for name in &missing {
            report.error("line 1", format!("missing column {name:?}"));
        }
        missing.is_empty()
    }
}

/// One row, with typed accessors that record problems in the report.
struct Row<'a> {
    table: &'a Table,
    line: u64,
    values: &'a [String],
    report: &'a mut ValidationReport,
}

impl<'a> Row<'a> {
    fn raw(&self, column: &str) -> Option<&'a str> {
        let (table, values): (&'a Table, &'a [String]) = (self.table, self.values);
        let i = *table.columns.get(column)?;
        values.get(i).map(String::as_str).filter(|v| !v.is_empty())
    }

    fn error(&mut self, column: &str, message: impl Into<String>) {
        let path = if column.is_empty() {
            format!("line {}", self.line)
        } else {
            format!("line {} ({column})", self.line)
        };
        self.report.error(path, message);
    }

    fn required<T>(
        &mut self,
        column: &str,
        parse: impl FnOnce(&str) -> std::result::Result<T, String>,
    ) -> Option<T> {
        match self.raw(column) {
            None => {
                self.error(column, "is required");
                None
            }
            Some(text) => self.parse(column, text, parse),
        }
    }

    fn optional<T>(
        &mut self,
        column: &str,
        parse: impl FnOnce(&str) -> std::result::Result<T, String>,
    ) -> Option<Option<T>> {
        match self.raw(column) {
            None => Some(None),
            Some(text) => self.parse(column, text, parse).map(Some),
        }
    }

    fn parse<T>(
        &mut self,
        column: &str,
        text: &str,
        parse: impl FnOnce(&str) -> std::result::Result<T, String>,
    ) -> Option<T> {
        match parse(text) {
            Ok(value) => Some(value),
            Err(message) => {
                self.error(column, message);
                None
            }
        }
    }

    fn decimal(&mut self, column: &str) -> Option<Decimal> {
        self.required(column, parse_decimal)
    }

    fn decimal_or_zero(&mut self, column: &str) -> Option<Decimal> {
        self.optional(column, parse_decimal)
            .map(Option::unwrap_or_default)
    }

    fn date(&mut self, column: &str) -> Option<Date> {
        self.required(column, parse_date)
    }

    fn instrument(&mut self, column: &str) -> Option<InstrumentId> {
        self.required(column, |t| {
            InstrumentId::new(t).ok_or_else(|| format!("{t:?} is not a valid identifier"))
        })
    }

    fn currency(&mut self, column: &str) -> Option<Currency> {
        self.required(column, |t| Currency::new(t).map_err(|e| e.to_string()))
    }

    fn money(&mut self, amount: &str, currency: &str) -> Option<Money> {
        let a = self.decimal(amount);
        let c = self.currency(currency);
        Some(Money::new(a?, c?))
    }
}

fn parse_decimal(text: &str) -> std::result::Result<Decimal, String> {
    let well_formed = !text.contains(',') && !text.contains('_') && !text.contains(' ');
    match text.parse::<Decimal>() {
        Ok(value) if well_formed => Ok(value),
        _ => Decimal::from_scientific(text)
            .ok()
            .filter(|_| well_formed)
            .ok_or_else(|| {
                format!(
                    "{text:?} is not a number (use a dot for decimals and no thousands separators)"
                )
            }),
    }
}

fn parse_date(text: &str) -> std::result::Result<Date, String> {
    text.parse::<Date>().map_err(|e| e.to_string())
}

fn parse_kind(text: &str) -> std::result::Result<InstrumentKind, String> {
    let kinds = [
        ("equity", InstrumentKind::Equity),
        ("fund", InstrumentKind::Fund),
        ("etf", InstrumentKind::Fund),
        ("bond", InstrumentKind::Bond),
        ("future", InstrumentKind::Future),
        ("option", InstrumentKind::Option),
        ("fx_forward", InstrumentKind::FxForward),
        ("other", InstrumentKind::Other),
    ];
    let lower = text.to_ascii_lowercase();
    kinds
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, kind)| *kind)
        .ok_or_else(|| format!("{text:?} is not a kind; use one of equity, fund, etf, bond, future, option, fx_forward, other"))
}

fn parse_lot_method(text: &str) -> std::result::Result<LotMethod, String> {
    serde_json::from_value(serde_json::Value::String(text.to_ascii_lowercase())).map_err(|_| {
        format!("{text:?} is not a lot method; use fifo, lifo, highest_cost, lowest_cost, average_cost or specific_id")
    })
}

/// Lots for specific identification, written `lot:quantity` and separated by semicolons
/// (`t1:5; t3:2.5`).
fn parse_lots(text: &str) -> std::result::Result<Vec<LotPick>, String> {
    text.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|item| {
            let (lot, quantity) = item
                .split_once(':')
                .ok_or_else(|| format!("{item:?} should be lot:quantity"))?;
            Ok(LotPick {
                lot: LotId::new(lot.trim()).ok_or_else(|| format!("{lot:?} is not a valid lot"))?,
                quantity: parse_decimal(quantity.trim())?,
            })
        })
        .collect()
}

fn finish<T>(report: ValidationReport, value: T) -> Result<T> {
    report.into_result(value)
}

/// Reads instruments. Columns: `id`, `kind` and `currency` (required); `name` and `multiplier`
/// (optional); `class.<scheme>` for classifications (for example `class.asset_class`) and
/// `id.<scheme>` for other identifiers (for example `id.isin`).
pub fn instruments_from_csv(text: &str, options: CsvOptions) -> Result<Vec<Instrument>> {
    let mut report = ValidationReport::new();
    let Some(table) = Table::read(text, options, &mut report) else {
        return Err(Error::Validation(report));
    };
    if !table.require(&["id", "kind", "currency"], &mut report) {
        return Err(Error::Validation(report));
    }
    let mut instruments = Vec::new();
    for (line, values) in &table.rows {
        let mut row = Row {
            table: &table,
            line: *line,
            values,
            report: &mut report,
        };
        let id = row.instrument("id");
        let kind = row.required("kind", parse_kind);
        let currency = row.currency("currency");
        let multiplier = row.optional("multiplier", parse_decimal);
        let (Some(id), Some(kind), Some(currency), Some(multiplier)) =
            (id, kind, currency, multiplier)
        else {
            continue;
        };
        let mut instrument = Instrument::new(id, kind, currency);
        instrument.name = row.raw("name").map(str::to_owned);
        instrument.multiplier = multiplier.unwrap_or(Decimal::ONE);
        for (i, name) in table.names.iter().enumerate() {
            let Some(value) = values.get(i).filter(|v| !v.is_empty()) else {
                continue;
            };
            if let Some(scheme) = name.strip_prefix("class.") {
                instrument
                    .classifications
                    .insert(scheme.to_owned(), value.clone());
            } else if let Some(scheme) = name.strip_prefix("id.") {
                instrument
                    .identifiers
                    .insert(scheme.to_owned(), value.clone());
            }
        }
        instruments.push(instrument);
    }
    finish(report, instruments)
}

/// Reads transactions. Every row needs `id`, `account`, `trade_date` and `type`; the other
/// columns depend on the type:
///
/// | type | columns |
/// | --- | --- |
/// | `buy` | `instrument`, `quantity`, `price`, optional `fees` |
/// | `sell` | `instrument`, `quantity`, `price`, optional `fees`, `lot_method`, `lots` |
/// | `deposit`, `withdrawal`, `fee`, `tax` | `amount`, `currency` |
/// | `transfer_in` | `instrument`, `quantity`, optional `cost_basis`, `acquired` |
/// | `transfer_out` | `instrument`, `quantity`, optional `lot_method`, `lots` |
/// | `dividend` | `instrument`, `amount`, `currency`, optional `withholding_tax` |
/// | `interest` | optional `instrument`, `amount`, `currency`, optional `withholding_tax` |
/// | `fx_conversion` | `sell_amount`, `sell_currency`, `buy_amount`, `buy_currency` |
/// | `split` | `instrument`, `from`, `to` |
/// | `spin_off` | `instrument`, `new_instrument`, `ratio`, `cost_allocation` |
/// | `merger` | `instrument`, `new_instrument` (unless all cash), `ratio`, optional `cash_per_unit` |
/// | `symbol_change` | `instrument`, `new_instrument` |
/// | `return_of_capital` | `instrument`, `amount`, `currency` |
///
/// Optional on every row: `settle_date` and `note`. `lots` names lots for specific
/// identification as `lot:quantity` pairs separated by semicolons.
pub fn transactions_from_csv(text: &str, options: CsvOptions) -> Result<Vec<Transaction>> {
    let mut report = ValidationReport::new();
    let Some(table) = Table::read(text, options, &mut report) else {
        return Err(Error::Validation(report));
    };
    if !table.require(&["id", "account", "trade_date", "type"], &mut report) {
        return Err(Error::Validation(report));
    }
    let mut transactions = Vec::new();
    for (line, values) in &table.rows {
        let mut row = Row {
            table: &table,
            line: *line,
            values,
            report: &mut report,
        };
        let id = row.required("id", |t| {
            TransactionId::new(t).ok_or_else(|| format!("{t:?} is not a valid identifier"))
        });
        let account = row.required("account", |t| {
            AccountId::new(t).ok_or_else(|| format!("{t:?} is not a valid identifier"))
        });
        let trade_date = row.date("trade_date");
        let settle_date = row.optional("settle_date", parse_date);
        let kind_name = row.raw("type").map(str::to_ascii_lowercase);
        let kind = match kind_name.as_deref() {
            None => {
                row.error("type", "is required");
                None
            }
            Some(name) => transaction_kind(&mut row, name),
        };
        let (Some(id), Some(account), Some(trade_date), Some(settle_date), Some(kind)) =
            (id, account, trade_date, settle_date, kind)
        else {
            continue;
        };
        transactions.push(Transaction {
            id,
            account,
            trade_date,
            settle_date,
            kind,
            note: row.raw("note").map(str::to_owned),
        });
    }
    finish(report, transactions)
}

fn transaction_kind(row: &mut Row<'_>, name: &str) -> Option<TransactionKind> {
    use TransactionKind::*;
    Some(match name {
        "buy" => {
            let (instrument, quantity, price, fees) = (
                row.instrument("instrument"),
                row.decimal("quantity"),
                row.decimal("price"),
                row.decimal_or_zero("fees"),
            );
            Buy {
                instrument: instrument?,
                quantity: quantity?,
                price: price?,
                fees: fees?,
            }
        }
        "sell" => {
            let (instrument, quantity, price, fees) = (
                row.instrument("instrument"),
                row.decimal("quantity"),
                row.decimal("price"),
                row.decimal_or_zero("fees"),
            );
            let lot_method = row.optional("lot_method", parse_lot_method);
            let lots = row.optional("lots", parse_lots);
            Sell {
                instrument: instrument?,
                quantity: quantity?,
                price: price?,
                fees: fees?,
                lot_method: lot_method?,
                lots: lots?.unwrap_or_default(),
            }
        }
        "deposit" => Deposit {
            amount: row.money("amount", "currency")?,
        },
        "withdrawal" => Withdrawal {
            amount: row.money("amount", "currency")?,
        },
        "fee" => Fee {
            amount: row.money("amount", "currency")?,
        },
        "tax" => Tax {
            amount: row.money("amount", "currency")?,
        },
        "transfer_in" => {
            let (instrument, quantity) = (row.instrument("instrument"), row.decimal("quantity"));
            let cost_basis = row.optional("cost_basis", parse_decimal);
            let acquired = row.optional("acquired", parse_date);
            TransferIn {
                instrument: instrument?,
                quantity: quantity?,
                cost_basis: cost_basis?,
                acquired: acquired?,
            }
        }
        "transfer_out" => {
            let (instrument, quantity) = (row.instrument("instrument"), row.decimal("quantity"));
            let lot_method = row.optional("lot_method", parse_lot_method);
            let lots = row.optional("lots", parse_lots);
            TransferOut {
                instrument: instrument?,
                quantity: quantity?,
                lot_method: lot_method?,
                lots: lots?.unwrap_or_default(),
            }
        }
        "dividend" => {
            let (instrument, amount, tax) = (
                row.instrument("instrument"),
                row.money("amount", "currency"),
                row.decimal_or_zero("withholding_tax"),
            );
            Dividend {
                instrument: instrument?,
                amount: amount?,
                withholding_tax: tax?,
            }
        }
        "interest" => {
            let instrument = row.optional("instrument", |t| {
                InstrumentId::new(t).ok_or_else(|| format!("{t:?} is not a valid identifier"))
            });
            let (amount, tax) = (
                row.money("amount", "currency"),
                row.decimal_or_zero("withholding_tax"),
            );
            Interest {
                instrument: instrument?,
                amount: amount?,
                withholding_tax: tax?,
            }
        }
        "fx_conversion" => {
            let (sell, buy) = (
                row.money("sell_amount", "sell_currency"),
                row.money("buy_amount", "buy_currency"),
            );
            FxConversion {
                sell: sell?,
                buy: buy?,
            }
        }
        "split" => {
            let (instrument, from, to) = (
                row.instrument("instrument"),
                row.decimal("from"),
                row.decimal("to"),
            );
            Split {
                instrument: instrument?,
                from: from?,
                to: to?,
            }
        }
        "spin_off" => {
            let (instrument, new_instrument) = (
                row.instrument("instrument"),
                row.instrument("new_instrument"),
            );
            let (ratio, cost_allocation) = (row.decimal("ratio"), row.decimal("cost_allocation"));
            SpinOff {
                instrument: instrument?,
                new_instrument: new_instrument?,
                ratio: ratio?,
                cost_allocation: cost_allocation?,
            }
        }
        "merger" => {
            let instrument = row.instrument("instrument");
            let (ratio, cash_per_unit) =
                (row.decimal("ratio"), row.decimal_or_zero("cash_per_unit"));
            let new_instrument = match (ratio, row.raw("new_instrument")) {
                (Some(r), None) if r.is_zero() => instrument.clone(),
                _ => row.instrument("new_instrument"),
            };
            Merger {
                instrument: instrument?,
                new_instrument: new_instrument?,
                ratio: ratio?,
                cash_per_unit: cash_per_unit?,
            }
        }
        "symbol_change" => {
            let (instrument, new_instrument) = (
                row.instrument("instrument"),
                row.instrument("new_instrument"),
            );
            SymbolChange {
                instrument: instrument?,
                new_instrument: new_instrument?,
            }
        }
        "return_of_capital" => {
            let (instrument, amount) = (
                row.instrument("instrument"),
                row.money("amount", "currency"),
            );
            ReturnOfCapital {
                instrument: instrument?,
                amount: amount?,
            }
        }
        other => {
            row.error("type", format!("{other:?} is not a transaction type"));
            return None;
        }
    })
}

/// Reads prices, in either layout:
///
/// - long: columns `date`, `instrument` and `price` (or `close`), one row per price;
/// - wide: a `date` column and one column per instrument, named by its identifier.
pub fn prices_from_csv(
    text: &str,
    options: CsvOptions,
) -> Result<BTreeMap<InstrumentId, TimeSeries<Decimal>>> {
    let mut report = ValidationReport::new();
    let Some(table) = Table::read(text, options, &mut report) else {
        return Err(Error::Validation(report));
    };
    let mut points: BTreeMap<InstrumentId, Points> = BTreeMap::new();
    let price_column = ["price", "close"]
        .into_iter()
        .find(|c| table.columns.contains_key(*c));
    if table.columns.contains_key("instrument") {
        let Some(price_column) = price_column else {
            report.error("line 1", "missing column \"price\"");
            return Err(Error::Validation(report));
        };
        if !table.require(&["date"], &mut report) {
            return Err(Error::Validation(report));
        }
        for (line, values) in &table.rows {
            let mut row = Row {
                table: &table,
                line: *line,
                values,
                report: &mut report,
            };
            let (date, instrument, price) = (
                row.date("date"),
                row.instrument("instrument"),
                row.decimal(price_column),
            );
            if let (Some(date), Some(instrument), Some(price)) = (date, instrument, price) {
                points
                    .entry(instrument)
                    .or_default()
                    .push((date, price, *line));
            }
        }
    } else {
        if !table.require(&["date"], &mut report) {
            return Err(Error::Validation(report));
        }
        let mut columns: Vec<(usize, InstrumentId)> = Vec::new();
        for (i, name) in table
            .names
            .iter()
            .enumerate()
            .filter(|(_, n)| n.as_str() != "date")
        {
            // Use the header as written: identifiers are case-sensitive.
            match original_header(text, options, i).and_then(InstrumentId::new) {
                Some(id) => columns.push((i, id)),
                None => report.error(
                    "line 1",
                    format!("column {name:?} is not a valid instrument identifier"),
                ),
            }
        }
        for (line, values) in &table.rows {
            let mut row = Row {
                table: &table,
                line: *line,
                values,
                report: &mut report,
            };
            let Some(date) = row.date("date") else {
                continue;
            };
            for (i, id) in &columns {
                let Some(text) = values.get(*i).filter(|v| !v.is_empty()) else {
                    continue;
                };
                let column = id.as_str().to_owned();
                if let Some(price) = row.parse(&column, &text.clone(), parse_decimal) {
                    points
                        .entry(id.clone())
                        .or_default()
                        .push((date, price, *line));
                }
            }
        }
    }
    let series = build_series(points, &mut report, "price", |id| id.to_string());
    finish(report, series)
}

/// Reads FX rates, in either layout:
///
/// - long: columns `date`, `base`, `quote` and `rate`;
/// - wide: a `date` column and one column per pair, named `EURUSD`, `EUR/USD` or `EUR.USD`
///   (units of the second currency per unit of the first).
pub fn fx_from_csv(text: &str, options: CsvOptions) -> Result<Vec<FxSeries>> {
    let mut report = ValidationReport::new();
    let Some(table) = Table::read(text, options, &mut report) else {
        return Err(Error::Validation(report));
    };
    if !table.require(&["date"], &mut report) {
        return Err(Error::Validation(report));
    }
    let mut points: BTreeMap<(Currency, Currency), Points> = BTreeMap::new();
    if table.columns.contains_key("base") {
        if !table.require(&["quote", "rate"], &mut report) {
            return Err(Error::Validation(report));
        }
        for (line, values) in &table.rows {
            let mut row = Row {
                table: &table,
                line: *line,
                values,
                report: &mut report,
            };
            let (date, base, quote, rate) = (
                row.date("date"),
                row.currency("base"),
                row.currency("quote"),
                row.decimal("rate"),
            );
            if let (Some(date), Some(base), Some(quote), Some(rate)) = (date, base, quote, rate) {
                points
                    .entry((base, quote))
                    .or_default()
                    .push((date, rate, *line));
            }
        }
    } else {
        let mut columns = Vec::new();
        for (i, name) in table
            .names
            .iter()
            .enumerate()
            .filter(|(_, n)| n.as_str() != "date")
        {
            let letters: String = name
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .collect::<String>()
                .to_ascii_uppercase();
            match (
                letters.len(),
                Currency::new(letters.get(..3).unwrap_or("")),
                Currency::new(letters.get(3..).unwrap_or("")),
            ) {
                (6, Ok(base), Ok(quote)) => columns.push((i, base, quote)),
                _ => report.error(
                    "line 1",
                    format!("column {name:?} is not a currency pair such as EURUSD"),
                ),
            }
        }
        for (line, values) in &table.rows {
            let mut row = Row {
                table: &table,
                line: *line,
                values,
                report: &mut report,
            };
            let Some(date) = row.date("date") else {
                continue;
            };
            for (i, base, quote) in &columns {
                let Some(text) = values.get(*i).filter(|v| !v.is_empty()) else {
                    continue;
                };
                let column = format!("{base}{quote}");
                if let Some(rate) = row.parse(&column, &text.clone(), parse_decimal) {
                    points
                        .entry((*base, *quote))
                        .or_default()
                        .push((date, rate, *line));
                }
            }
        }
    }
    let series = build_series(points, &mut report, "rate", |(b, q)| format!("{b}/{q}"))
        .into_iter()
        .map(|((base, quote), rates)| FxSeries { base, quote, rates })
        .collect();
    finish(report, series)
}

fn build_series<K: Ord + Clone>(
    points: BTreeMap<K, Points>,
    report: &mut ValidationReport,
    what: &str,
    label: impl Fn(&K) -> String,
) -> BTreeMap<K, TimeSeries<Decimal>> {
    let mut out = BTreeMap::new();
    for (key, mut pts) in points {
        let name = label(&key);
        pts.sort_by_key(|p| (p.0, p.2));
        let mut seen = BTreeSet::new();
        let mut clean = Vec::with_capacity(pts.len());
        for (date, value, line) in pts {
            if seen.insert(date) {
                clean.push((date, value));
            } else {
                report.error(
                    format!("line {line}"),
                    format!("a second {what} for {name} on {date}"),
                );
            }
            if value <= Decimal::ZERO {
                report.error(
                    format!("line {line}"),
                    format!("the {what} for {name} on {date} must be greater than zero"),
                );
            }
        }
        if let Ok(series) = TimeSeries::from_unsorted(clean) {
            out.insert(key, series);
        }
    }
    out
}

/// The header of column `index` as written in the file, before lower-casing.
fn original_header(text: &str, options: CsvOptions, index: usize) -> Option<String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(options.delimiter)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    reader
        .headers()
        .ok()
        .and_then(|h| h.get(index).map(str::to_owned))
}

/// Builds a portfolio from instruments and transactions read from CSV. Accounts are created for
/// every account the transactions name, with first-in, first-out lot relief.
///
/// Problems in either file and in the portfolio's structure are reported together, with paths
/// under `instruments`, `transactions` and `portfolio`.
pub fn portfolio_from_csv(
    base_currency: Currency,
    instruments_csv: &str,
    transactions_csv: &str,
    options: CsvOptions,
) -> Result<Portfolio> {
    let mut report = ValidationReport::new();
    let instruments = match instruments_from_csv(instruments_csv, options) {
        Ok(v) => v,
        Err(Error::Validation(r)) => {
            report.merge("instruments", r);
            Vec::new()
        }
        Err(e) => return Err(e),
    };
    let transactions = match transactions_from_csv(transactions_csv, options) {
        Ok(v) => v,
        Err(Error::Validation(r)) => {
            report.merge("transactions", r);
            Vec::new()
        }
        Err(e) => return Err(e),
    };
    if report.has_errors() {
        return Err(Error::Validation(report));
    }
    let accounts: BTreeSet<AccountId> = transactions.iter().map(|t| t.account.clone()).collect();
    let portfolio = Portfolio {
        base_currency,
        accounts: accounts.into_iter().map(Account::new).collect(),
        instruments,
        transactions,
    };
    report.merge("portfolio", portfolio.validate());
    report.into_result(portfolio)
}
