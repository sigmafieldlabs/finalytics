//! The WebAssembly module behind the `finalytics` npm package.
//!
//! This layer only converts data between JavaScript and Rust; the analytics live in the
//! `finalytics` crate. Structured values cross as plain objects in the formats of
//! `schema/finalytics.schema.json`, whose TypeScript declarations are generated into
//! `types.d.ts`, by way of JSON text, which is far faster for large values than reading objects
//! property by property; series of numbers cross as `Float64Array`s and dates as ISO 8601
//! strings.
//!
//! Errors are thrown as `Error` objects. A validation error has the name `ValidationError` and an
//! `issues` property listing every problem, each with a `path` and a `message`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

use finalytics::formats::{
    CsvOptions, MarketDocument, PortfolioDocument, fx_from_csv, portfolio_from_csv, prices_from_csv,
};
use finalytics::opt::{Objective, Problem, Rebalance, RebalanceOptions};
use finalytics::risk::{
    self, CovarianceMethod, Matrix, ReturnsTable, StatisticsOptions, TailMethod,
};
use finalytics::{
    AccountId, Currency, Date, Decimal, Error, FlowTiming, HoldingKey, HoldingSeries, InstrumentId,
    LotMethod, MarketData as EngineMarket, Observation, Period, Portfolio, TimeSeries,
    ValidationReport,
};

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = include_str!("../types.d.ts");

#[wasm_bindgen(typescript_custom_section)]
const EXTRA_TYPES: &'static str = r#"
/** A price or rate found for a date. */
export interface Observed {
  /** The value. */
  value: Decimal;
  /** The date it was observed, earlier than the date asked for if it was carried forward. */
  date: Date;
  /** Whether it was carried forward from an earlier date. */
  carried: boolean;
}

/** Open positions, tax lots and cash balances at a point in time. */
export interface Holdings {
  positions: Position[];
  lots: Lot[];
  cash: CashBalance[];
}

/** A portfolio document: a portfolio and, optionally, its market data. */
export interface Document {
  portfolio: Portfolio;
  market?: MarketData;
}

/** Choices for `Books.rebalance`, all optional. */
export interface RebalanceChoices extends Partial<RebalanceOptions> {
  /** The date of the prices used; the last price date by default. */
  date?: Date;
}

/** A validation error: every problem found, with where it is. */
export interface ValidationError extends Error {
  name: "ValidationError";
  issues: Issue[];
}
"#;

fn error(e: Error) -> JsValue {
    let js = js_sys::Error::new(&e.to_string());
    if let Error::Validation(report) = &e {
        js.set_name("ValidationError");
        if let Ok(issues) = to_js(&report.issues()) {
            let _ = js_sys::Reflect::set(&js, &"issues".into(), &issues);
        }
    }
    js.into()
}

fn invalid(path: &str, message: impl Into<String>) -> JsValue {
    let mut report = ValidationReport::new();
    report.error(path, message);
    error(Error::Validation(report))
}

/// A Rust value as a JavaScript value, through JSON text: one call to the JavaScript engine's
/// JSON parser is much faster than building a large object property by property across the
/// boundary.
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    let text = serde_json::to_string(value)
        .map_err(|e| invalid("", format!("could not encode a result: {e}")))?;
    js_sys::JSON::parse(&text)
}

/// A JavaScript value as a Rust value, through JSON text (see `to_js`), reporting where it is
/// malformed.
fn from_js<T: DeserializeOwned>(value: JsValue, name: &str) -> Result<T, JsValue> {
    if value.is_undefined() {
        return Err(invalid(name, "is missing"));
    }
    let text: String = js_sys::JSON::stringify(&value)
        .map_err(|_| invalid(name, "cannot be written as JSON"))?
        .into();
    // Reading without tracking paths is faster; only a value that fails is read again to find
    // where it goes wrong.
    if let Ok(found) = serde_json::from_str(&text) {
        return Ok(found);
    }
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|e| {
        let path = e.path().to_string();
        let path = if path == "." {
            name.to_owned()
        } else {
            format!("{name}.{path}")
        };
        invalid(&path, e.inner().to_string())
    })
}

fn optional<T: DeserializeOwned + Default>(value: JsValue, name: &str) -> Result<T, JsValue> {
    if value.is_undefined() || value.is_null() {
        Ok(T::default())
    } else {
        from_js(value, name)
    }
}

fn named<T: DeserializeOwned>(text: &str, name: &str) -> Result<T, JsValue> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|e| invalid(name, e.to_string()))
}

fn parse_date(text: &str) -> Result<Date, JsValue> {
    text.parse()
        .map_err(|_| invalid("date", format!("{text:?} is not a date in YYYY-MM-DD form")))
}

fn optional_date(text: Option<String>) -> Result<Option<Date>, JsValue> {
    text.as_deref().map(parse_date).transpose()
}

fn parse_dates(values: Vec<String>, name: &str) -> Result<Vec<Date>, JsValue> {
    values
        .iter()
        .enumerate()
        .map(|(k, d)| {
            d.parse().map_err(|_| {
                invalid(
                    &format!("{name}[{k}]"),
                    format!("{d:?} is not a date in YYYY-MM-DD form"),
                )
            })
        })
        .collect()
}

fn date_strings(values: &[Date]) -> Vec<String> {
    values.iter().map(Date::to_string).collect()
}

fn currency(text: &str) -> Result<Currency, JsValue> {
    text.parse().map_err(error)
}

fn instrument(text: &str) -> Result<InstrumentId, JsValue> {
    InstrumentId::new(text)
        .ok_or_else(|| invalid("instrument", format!("{text:?} is not a valid identifier")))
}

fn parse_account(text: &str) -> Result<AccountId, JsValue> {
    AccountId::new(text)
        .ok_or_else(|| invalid("account", format!("{text:?} is not a valid identifier")))
}

fn csv_options(delimiter: Option<String>) -> Result<CsvOptions, JsValue> {
    match delimiter.as_deref().map(str::as_bytes) {
        None => Ok(CsvOptions::default()),
        Some([byte]) => Ok(CsvOptions { delimiter: *byte }),
        Some(_) => Err(invalid("delimiter", "must be a single character")),
    }
}

fn series(dates: Vec<String>, values: JsValue, name: &str) -> Result<TimeSeries<Decimal>, JsValue> {
    let values: Vec<Decimal> = from_js(values, name)?;
    TimeSeries::new(parse_dates(dates, "dates")?, values).map_err(error)
}

fn observation(found: Observation<Decimal>, on: Date) -> Result<JsValue, JsValue> {
    let (value, from, carried) = match found {
        Observation::Observed(v) => (v, on, false),
        Observation::Carried { value, from } => (value, from, true),
        Observation::Missing => return Ok(JsValue::UNDEFINED),
    };
    to_js(&serde_json::json!({"value": value, "date": from, "carried": carried}))
}

/// Prices and FX rates for valuing a portfolio. Each method that adds data returns a new
/// `MarketData`.
///
/// Prices are in each instrument's currency; an FX rate is the number of units of `quote` per
/// unit of `base`. A rate can be found directly, inverted, or crossed through the pivot currency
/// (USD by default). A missing value may be carried forward from an earlier date for at most
/// `maxCarryDays` days (5 by default).
#[wasm_bindgen]
#[derive(Clone)]
pub struct MarketData {
    inner: EngineMarket,
}

#[wasm_bindgen]
impl MarketData {
    /// Empty market data.
    #[wasm_bindgen(constructor)]
    pub fn new() -> MarketData {
        MarketData {
            inner: EngineMarket::new(),
        }
    }

    /// Market data in the portfolio document's format: `{prices, fx, pivot, max_carry_days}`.
    #[wasm_bindgen(js_name = fromDocument)]
    pub fn from_document(
        #[wasm_bindgen(unchecked_param_type = "DocumentMarketData")] value: JsValue,
    ) -> Result<MarketData, JsValue> {
        let MarketDocument(inner) = from_js(value, "market")?;
        Ok(MarketData { inner })
    }

    /// Market data from CSV text of prices and FX rates, in long or wide layout.
    #[wasm_bindgen(js_name = fromCsv)]
    pub fn from_csv(
        prices: Option<String>,
        fx: Option<String>,
        delimiter: Option<String>,
    ) -> Result<MarketData, JsValue> {
        let options = csv_options(delimiter)?;
        let mut inner = EngineMarket::new();
        if let Some(text) = prices {
            for (id, series) in prices_from_csv(&text, options).map_err(error)? {
                inner = inner.with_prices(id, series);
            }
        }
        if let Some(text) = fx {
            for s in fx_from_csv(&text, options).map_err(error)? {
                inner = inner.with_fx(s.base, s.quote, s.rates);
            }
        }
        Ok(MarketData { inner })
    }

    /// These market data in the portfolio document's format.
    #[wasm_bindgen(js_name = toDocument, unchecked_return_type = "DocumentMarketData")]
    pub fn to_document(&self) -> Result<JsValue, JsValue> {
        to_js(&MarketDocument(self.inner.clone()))
    }

    /// These market data with prices for `id` on `dates`.
    #[wasm_bindgen(js_name = withPrices)]
    pub fn with_prices(
        &self,
        id: &str,
        dates: Vec<String>,
        #[wasm_bindgen(unchecked_param_type = "(Decimal | number)[]")] values: JsValue,
    ) -> Result<MarketData, JsValue> {
        let prices = series(dates, values, "values")?;
        Ok(MarketData {
            inner: self.inner.clone().with_prices(instrument(id)?, prices),
        })
    }

    /// These market data with rates for `base` priced in `quote`.
    #[wasm_bindgen(js_name = withFx)]
    pub fn with_fx(
        &self,
        base: &str,
        quote: &str,
        dates: Vec<String>,
        #[wasm_bindgen(unchecked_param_type = "(Decimal | number)[]")] rates: JsValue,
    ) -> Result<MarketData, JsValue> {
        let rates = series(dates, rates, "rates")?;
        Ok(MarketData {
            inner: self
                .inner
                .clone()
                .with_fx(currency(base)?, currency(quote)?, rates),
        })
    }

    /// These market data crossing rates through `pivot`.
    #[wasm_bindgen(js_name = withPivot)]
    pub fn with_pivot(&self, pivot: &str) -> Result<MarketData, JsValue> {
        Ok(MarketData {
            inner: self.inner.clone().with_pivot(currency(pivot)?),
        })
    }

    /// These market data carrying values forward for at most `days` days.
    #[wasm_bindgen(js_name = withMaxCarryDays)]
    pub fn with_max_carry_days(&self, days: u32) -> MarketData {
        MarketData {
            inner: self.inner.clone().with_max_carry_days(days),
        }
    }

    /// The currency through which rates are crossed.
    #[wasm_bindgen(getter)]
    pub fn pivot(&self) -> String {
        self.inner.pivot().to_string()
    }

    /// How many days a price or rate may be carried forward.
    #[wasm_bindgen(getter, js_name = maxCarryDays)]
    pub fn max_carry_days(&self) -> u32 {
        self.inner.max_carry_days()
    }

    /// Every date with a price for any instrument, in order.
    #[wasm_bindgen(getter, js_name = priceDates)]
    pub fn price_dates(&self) -> Vec<String> {
        let mut all: Vec<Date> = self
            .inner
            .all_prices()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .collect();
        all.sort();
        all.dedup();
        date_strings(&all)
    }

    /// The price of `id` on `date`, or `undefined` if there is none within the carry limit.
    #[wasm_bindgen(unchecked_return_type = "Observed | undefined")]
    pub fn price(&self, id: &str, date: &str) -> Result<JsValue, JsValue> {
        let on = parse_date(date)?;
        observation(self.inner.price(&instrument(id)?, on), on)
    }

    /// Units of `quote` per unit of `base` on `date`, or `undefined` if no rate is found.
    #[wasm_bindgen(js_name = fxRate, unchecked_return_type = "Observed | undefined")]
    pub fn fx_rate(&self, base: &str, quote: &str, date: &str) -> Result<JsValue, JsValue> {
        let on = parse_date(date)?;
        observation(
            self.inner.fx_rate(currency(base)?, currency(quote)?, on),
            on,
        )
    }
}

impl Default for MarketData {
    fn default() -> Self {
        MarketData::new()
    }
}

/// The books of a portfolio: its transactions applied in order, with the market data used to
/// value them.
#[wasm_bindgen]
pub struct Books {
    books: finalytics::Books,
    market: EngineMarket,
    first_trade: Option<Date>,
}

#[wasm_bindgen]
impl Books {
    /// Keeps the books of `portfolio`, using `market` for FX rates and for the prices that
    /// transfers without a cost basis need (pass `new MarketData()` for none).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Portfolio")] portfolio: JsValue,
        market: &MarketData,
    ) -> Result<Books, JsValue> {
        let portfolio: Portfolio = from_js(portfolio, "portfolio")?;
        let market = market.inner.clone();
        let books = finalytics::Books::build(&portfolio, &market).map_err(error)?;
        Ok(Books {
            books,
            first_trade: portfolio.transactions.iter().map(|t| t.trade_date).min(),
            market,
        })
    }

    /// The currency results are reported in.
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> String {
        self.books.base_currency().to_string()
    }

    /// The dates performance is measured on when none are given: every date with a price, from
    /// the first transaction to the last price.
    #[wasm_bindgen(js_name = defaultDates)]
    pub fn default_dates(&self) -> Vec<String> {
        let mut all: Vec<Date> = self
            .market
            .all_prices()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .filter(|d| self.first_trade.is_none_or(|first| *d >= first))
            .collect();
        all.sort();
        all.dedup();
        date_strings(&all)
    }

    fn dates_or_default(&self, given: Option<Vec<String>>) -> Result<Vec<Date>, JsValue> {
        match given {
            Some(d) => parse_dates(d, "dates"),
            None => parse_dates(self.default_dates(), "dates"),
        }
    }

    /// Positions, lots and cash at the end of `date`, or after the last transaction.
    #[wasm_bindgen(unchecked_return_type = "Holdings")]
    pub fn holdings(&self, date: Option<String>) -> Result<JsValue, JsValue> {
        let holdings = match optional_date(date)? {
            Some(d) => self.books.holdings_on(d),
            None => self.books.holdings().clone(),
        };
        let lots: Vec<_> = holdings.lots().collect();
        to_js(&serde_json::json!({
            "positions": holdings.positions(),
            "lots": lots,
            "cash": holdings.cash_balances(),
        }))
    }

    /// The holdings valued at the end of `date`.
    #[wasm_bindgen(unchecked_return_type = "Valuation")]
    pub fn valuation(&self, date: &str) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .books
                .value_on(parse_date(date)?, &self.market)
                .map_err(error)?,
        )
    }

    /// End-of-day values and external flows on `dates` (by default `defaultDates()`).
    #[wasm_bindgen(js_name = valueSeries, unchecked_return_type = "ValueSeries")]
    pub fn value_series(&self, dates: Option<Vec<String>>) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(dates)?;
        to_js(
            &self
                .books
                .value_series(&dates, &self.market)
                .map_err(error)?,
        )
    }

    /// Each holding's daily value, trade cash and income on `dates`.
    #[wasm_bindgen(js_name = holdingSeries, unchecked_return_type = "HoldingSeries")]
    pub fn holding_series(&self, dates: Option<Vec<String>>) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(dates)?;
        to_js(
            &self
                .books
                .holding_series(&dates, &self.market)
                .map_err(error)?,
        )
    }

    /// Every realised gain, by lot.
    #[wasm_bindgen(js_name = realisedGains, unchecked_return_type = "RealisedGain[]")]
    pub fn realised_gains(&self) -> Result<JsValue, JsValue> {
        to_js(self.books.realised_gains())
    }

    /// What each transaction did to the books, in the order applied.
    #[wasm_bindgen(unchecked_return_type = "Event[]")]
    pub fn events(&self) -> Result<JsValue, JsValue> {
        to_js(self.books.events())
    }

    /// Time-weighted performance on `dates` (by default `defaultDates()`), with flows at the
    /// start (default) or end of the day.
    pub fn performance(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Date[]")] dates: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "FlowTiming")] timing: Option<String>,
    ) -> Result<Performance, JsValue> {
        let dates = self.dates_or_default(optional(dates, "dates")?)?;
        let timing: FlowTiming = named(timing.as_deref().unwrap_or("start_of_day"), "timing")?;
        let values = self
            .books
            .value_series(&dates, &self.market)
            .map_err(error)?;
        let holdings = self
            .books
            .holding_series(&dates, &self.market)
            .map_err(error)?;
        Ok(Performance {
            inner: finalytics::Performance::from_values(&values, timing).map_err(error)?,
            holdings: Some(holdings),
            warnings: values.warnings,
        })
    }

    /// Each holding's share of the portfolio's value on `date`.
    #[wasm_bindgen(unchecked_return_type = "[HoldingKey, number][]")]
    pub fn weights(&self, date: &str) -> Result<JsValue, JsValue> {
        let valuation = self
            .books
            .value_on(parse_date(date)?, &self.market)
            .map_err(error)?;
        to_js(&risk::holding_weights(&valuation).map_err(error)?)
    }

    /// Price returns of `holdings` in the base currency between consecutive `dates`.
    #[wasm_bindgen(js_name = holdingReturns, unchecked_return_type = "HoldingReturns")]
    pub fn holding_returns(
        &self,
        #[wasm_bindgen(unchecked_param_type = "HoldingKey[]")] holdings: JsValue,
        dates: Option<Vec<String>>,
    ) -> Result<JsValue, JsValue> {
        let holdings: Vec<HoldingKey> = from_js(holdings, "holdings")?;
        let dates = self.dates_or_default(dates)?;
        to_js(&risk::holding_returns(&self.books, &holdings, &dates, &self.market).map_err(error)?)
    }

    /// The lots a sale would close, chosen by `method` (first in, first out by default).
    #[wasm_bindgen(js_name = lotsForSale, unchecked_return_type = "LotSale[]")]
    pub fn lots_for_sale(
        &self,
        date: &str,
        account: &str,
        id: &str,
        #[wasm_bindgen(unchecked_param_type = "Decimal | number")] quantity: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "LotMethod")] method: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let quantity: Decimal = from_js(quantity, "quantity")?;
        let method: LotMethod = named(method.as_deref().unwrap_or("fifo"), "method")?;
        let sale = self
            .books
            .holdings_on(parse_date(date)?)
            .lots_for_sale(&parse_account(account)?, &instrument(id)?, quantity, method)
            .map_err(error)?;
        to_js(&sale)
    }

    /// The trades that move `account` to `targets`, weights by holding, as shares of the
    /// account's value.
    #[wasm_bindgen(unchecked_return_type = "Rebalance")]
    pub fn rebalance(
        &self,
        account: &str,
        #[wasm_bindgen(unchecked_param_type = "[HoldingKey, number][]")] targets: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "RebalanceChoices")] choices: JsValue,
    ) -> Result<JsValue, JsValue> {
        let targets: Vec<(HoldingKey, f64)> = from_js(targets, "targets")?;
        let mut choices: serde_json::Map<String, serde_json::Value> = optional(choices, "choices")?;
        let on = match choices.remove("date") {
            Some(serde_json::Value::String(d)) => parse_date(&d)?,
            _ => match self.default_dates().last() {
                Some(last) => parse_date(last)?,
                None => {
                    return Err(invalid(
                        "date",
                        "give a date: the market data has no prices",
                    ));
                }
            },
        };
        let options: RebalanceOptions = serde_json::from_value(serde_json::Value::Object(choices))
            .map_err(|e| invalid("choices", e.to_string()))?;
        let plan = finalytics::opt::rebalance(
            &self.books,
            &self.market,
            on,
            &parse_account(account)?,
            &targets,
            &options,
        )
        .map_err(error)?;
        to_js(&plan)
    }
}

/// A portfolio's daily time-weighted returns and the measures built on them.
#[wasm_bindgen]
pub struct Performance {
    inner: finalytics::Performance,
    holdings: Option<HoldingSeries>,
    warnings: ValidationReport,
}

#[wasm_bindgen]
impl Performance {
    /// Performance from end-of-day values and the external flows since the previous date
    /// (positive into the portfolio; none by default).
    #[wasm_bindgen(js_name = fromValues)]
    pub fn from_values(
        dates: Vec<String>,
        values: &[f64],
        flows: Option<Vec<f64>>,
        #[wasm_bindgen(unchecked_optional_param_type = "FlowTiming")] timing: Option<String>,
    ) -> Result<Performance, JsValue> {
        let timing: FlowTiming = named(timing.as_deref().unwrap_or("start_of_day"), "timing")?;
        let flows = flows.unwrap_or_else(|| vec![0.0; values.len()]);
        let inner = finalytics::Performance::from_parts(
            parse_dates(dates, "dates")?,
            values.to_vec(),
            flows,
            timing,
        )
        .map_err(error)?;
        Ok(Performance {
            inner,
            holdings: None,
            warnings: ValidationReport::new(),
        })
    }

    /// The valuation dates.
    #[wasm_bindgen(getter)]
    pub fn parse_dates(&self) -> Vec<String> {
        date_strings(self.inner.dates())
    }

    /// The daily returns, one per date after the first.
    #[wasm_bindgen(getter)]
    pub fn returns(&self) -> Vec<f64> {
        self.inner.returns().values().to_vec()
    }

    /// The growth of 1 invested on the first date, one value per date.
    #[wasm_bindgen(getter)]
    pub fn growth(&self) -> Vec<f64> {
        self.inner.growth_of(1.0).values().to_vec()
    }

    /// The portfolio's value on each date, in the base currency.
    #[wasm_bindgen(getter)]
    pub fn values(&self) -> Vec<f64> {
        self.inner.values().values().to_vec()
    }

    /// Prices and rates carried forward while valuing the portfolio.
    #[wasm_bindgen(getter, unchecked_return_type = "Issue[]")]
    pub fn warnings(&self) -> Result<JsValue, JsValue> {
        to_js(self.warnings.issues())
    }

    fn range(&self, start: Option<String>, end: Option<String>) -> Result<(Date, Date), JsValue> {
        let all = self.inner.dates();
        let (Some(first), Some(last)) = (all.first(), all.last()) else {
            return Err(invalid("performance", "has no dates"));
        };
        Ok((
            optional_date(start)?.unwrap_or(*first),
            optional_date(end)?.unwrap_or(*last),
        ))
    }

    /// The time-weighted return from `start` to `end` (by default the whole history).
    #[wasm_bindgen(js_name = timeWeighted)]
    pub fn time_weighted(
        &self,
        start: Option<String>,
        end: Option<String>,
    ) -> Result<Option<f64>, JsValue> {
        let (s, e) = self.range(start, end)?;
        Ok(self.inner.time_weighted(s, e))
    }

    /// The return over a reporting period ending on `asOf`, annualised for a year or longer.
    #[wasm_bindgen(unchecked_return_type = "PeriodReturn | undefined")]
    pub fn period(
        &self,
        #[wasm_bindgen(
            unchecked_param_type = "Period | \"MTD\" | \"QTD\" | \"YTD\" | \"ITD\" | `${number}Y`"
        )]
        which: JsValue,
        #[wasm_bindgen(js_name = asOf)] as_of: Option<String>,
    ) -> Result<JsValue, JsValue> {
        match self.inner.period(period(which)?, optional_date(as_of)?) {
            Some(p) => to_js(&p),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    /// The money-weighted return (internal rate of return, annualised).
    #[wasm_bindgen(js_name = moneyWeighted)]
    pub fn money_weighted(
        &self,
        start: Option<String>,
        end: Option<String>,
    ) -> Result<Option<f64>, JsValue> {
        let (s, e) = self.range(start, end)?;
        Ok(self.inner.money_weighted(s, e))
    }

    /// The Modified Dietz return.
    #[wasm_bindgen(js_name = modifiedDietz)]
    pub fn modified_dietz(
        &self,
        start: Option<String>,
        end: Option<String>,
    ) -> Result<Option<f64>, JsValue> {
        let (s, e) = self.range(start, end)?;
        Ok(self.inner.modified_dietz(s, e))
    }

    /// The return over a period against a benchmark's, given as levels on dates.
    #[wasm_bindgen(js_name = excessReturn, unchecked_return_type = "ExcessReturn | undefined")]
    pub fn excess_return(
        &self,
        #[wasm_bindgen(js_name = benchmarkDates)] benchmark_dates: Vec<String>,
        #[wasm_bindgen(js_name = benchmarkLevels)] benchmark_levels: &[f64],
        #[wasm_bindgen(unchecked_optional_param_type = "Period | string")] which: JsValue,
        #[wasm_bindgen(js_name = asOf, unchecked_optional_param_type = "Date")] as_of: JsValue,
        #[wasm_bindgen(js_name = maxCarryDays, unchecked_optional_param_type = "number")]
        max_carry_days: JsValue,
    ) -> Result<JsValue, JsValue> {
        let as_of = as_of.as_string();
        let max_carry_days = max_carry_days.as_f64().map(|d| d as u32);
        let which = if which.is_undefined() {
            JsValue::from_str("ITD")
        } else {
            which
        };
        let benchmark = TimeSeries::new(
            parse_dates(benchmark_dates, "benchmark.dates")?,
            benchmark_levels.to_vec(),
        )
        .map_err(error)?;
        match self.inner.excess_return(
            period(which)?,
            optional_date(as_of)?,
            &benchmark,
            max_carry_days.unwrap_or(5),
        ) {
            Some(e) => to_js(&e),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    /// Each holding's contribution to the time-weighted return; they add up to the return.
    #[wasm_bindgen(unchecked_return_type = "Contribution[]")]
    pub fn contributions(
        &self,
        start: Option<String>,
        end: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let Some(holdings) = &self.holdings else {
            return Err(invalid(
                "performance",
                "contributions need performance measured from the books",
            ));
        };
        let (s, e) = self.range(start, end)?;
        to_js(&self.inner.contributions(holdings, s, e).map_err(error)?)
    }
}

/// A period from its short label (`"MTD"`, `"QTD"`, `"YTD"`, `"ITD"`, `"3Y"`) or its object form.
fn period(value: JsValue) -> Result<Period, JsValue> {
    let Some(label) = value.as_string() else {
        return from_js(value, "period");
    };
    let label = label.trim().to_uppercase();
    let found = match label.as_str() {
        "MTD" => Some(Period::MonthToDate),
        "QTD" => Some(Period::QuarterToDate),
        "YTD" => Some(Period::YearToDate),
        "ITD" => Some(Period::SinceInception),
        _ => label
            .strip_suffix('Y')
            .and_then(|n| n.parse().ok())
            .map(|years| Period::Years { years }),
    };
    found.ok_or_else(|| {
        invalid(
            "period",
            format!(
                "unknown period {label:?}: use MTD, QTD, YTD, ITD or a number of years such as 3Y"
            ),
        )
    })
}

/// The version of this release of Finalytics.
#[wasm_bindgen]
pub fn version() -> String {
    finalytics::version().to_owned()
}

/// Reads a portfolio document, reporting where a malformed one goes wrong.
#[wasm_bindgen(js_name = readDocument, unchecked_return_type = "Document")]
pub fn read_document(text: &str) -> Result<JsValue, JsValue> {
    let document = PortfolioDocument::from_json(text).map_err(error)?;
    let object = js_sys::Object::new();
    js_sys::Reflect::set(&object, &"portfolio".into(), &to_js(&document.portfolio)?)?;
    if let Some(inner) = document.market {
        js_sys::Reflect::set(
            &object,
            &"market".into(),
            &JsValue::from(MarketData { inner }),
        )?;
    }
    Ok(object.into())
}

/// A portfolio document holding `portfolio` and, optionally, market data in the document format
/// (`market.toDocument()`), as JSON text.
#[wasm_bindgen(js_name = writeDocument)]
pub fn write_document(
    #[wasm_bindgen(unchecked_param_type = "Portfolio")] portfolio: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "DocumentMarketData")] market: JsValue,
) -> Result<String, JsValue> {
    let mut document = PortfolioDocument::new(from_js(portfolio, "portfolio")?);
    let market: Option<MarketDocument> = optional(market, "market")?;
    document.market = market.map(|MarketDocument(m)| m);
    Ok(document.to_json())
}

/// Every structural problem in a portfolio, without applying its transactions.
#[wasm_bindgen(js_name = validatePortfolio, unchecked_return_type = "Issue[]")]
pub fn validate_portfolio(
    #[wasm_bindgen(unchecked_param_type = "Portfolio")] portfolio: JsValue,
) -> Result<JsValue, JsValue> {
    let portfolio: Portfolio = from_js(portfolio, "portfolio")?;
    to_js(portfolio.validate().issues())
}

/// A portfolio from CSV text of instruments and transactions.
#[wasm_bindgen(js_name = portfolioFromCsv, unchecked_return_type = "Portfolio")]
pub fn portfolio_from_csv_text(
    instruments: &str,
    transactions: &str,
    #[wasm_bindgen(js_name = baseCurrency)] base_currency: &str,
    delimiter: Option<String>,
) -> Result<JsValue, JsValue> {
    let portfolio = portfolio_from_csv(
        currency(base_currency)?,
        instruments,
        transactions,
        csv_options(delimiter)?,
    )
    .map_err(error)?;
    to_js(&portfolio)
}

fn statistics_options(value: JsValue) -> Result<StatisticsOptions, JsValue> {
    optional(value, "options")
}

/// Statistics of a returns series of at least two periods. Options default to 252 periods a
/// year and zero risk-free and target rates.
#[wasm_bindgen(unchecked_return_type = "Statistics")]
pub fn statistics(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    #[wasm_bindgen(unchecked_optional_param_type = "Partial<StatisticsOptions>")] options: JsValue,
) -> Result<JsValue, JsValue> {
    to_js(&risk::statistics(returns, &statistics_options(options)?).map_err(error)?)
}

/// Statistics of a returns series against a benchmark's over the same periods.
#[wasm_bindgen(js_name = relativeStatistics, unchecked_return_type = "RelativeStatistics")]
pub fn relative_statistics(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] benchmark: &[f64],
    #[wasm_bindgen(unchecked_optional_param_type = "Partial<StatisticsOptions>")] options: JsValue,
) -> Result<JsValue, JsValue> {
    to_js(
        &risk::relative_statistics(returns, benchmark, &statistics_options(options)?)
            .map_err(error)?,
    )
}

/// Value at risk and expected shortfall over one period, as positive losses.
#[wasm_bindgen(js_name = tailRisk, unchecked_return_type = "TailRisk")]
pub fn tail_risk(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    #[wasm_bindgen(unchecked_optional_param_type = "number")] confidence: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "TailMethod")] method: Option<String>,
) -> Result<JsValue, JsValue> {
    let method: TailMethod = named(method.as_deref().unwrap_or("historical"), "method")?;
    to_js(&risk::tail_risk(returns, confidence.as_f64().unwrap_or(0.95), method).map_err(error)?)
}

/// `statistics` over every window of `window` returns.
#[wasm_bindgen(js_name = rollingStatistics, unchecked_return_type = "Statistics[]")]
pub fn rolling_statistics(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    window: usize,
    #[wasm_bindgen(unchecked_optional_param_type = "Partial<StatisticsOptions>")] options: JsValue,
) -> Result<JsValue, JsValue> {
    to_js(&risk::rolling_statistics(returns, window, &statistics_options(options)?).map_err(error)?)
}

/// `relativeStatistics` over every window of `window` returns.
#[wasm_bindgen(js_name = rollingRelativeStatistics, unchecked_return_type = "RelativeStatistics[]")]
pub fn rolling_relative_statistics(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] benchmark: &[f64],
    window: usize,
    #[wasm_bindgen(unchecked_optional_param_type = "Partial<StatisticsOptions>")] options: JsValue,
) -> Result<JsValue, JsValue> {
    let found = risk::rolling_relative_statistics(
        returns,
        benchmark,
        window,
        &statistics_options(options)?,
    )
    .map_err(error)?;
    to_js(&found)
}

/// `tailRisk` over every window of `window` returns.
#[wasm_bindgen(js_name = rollingTailRisk, unchecked_return_type = "TailRisk[]")]
pub fn rolling_tail_risk(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    window: usize,
    #[wasm_bindgen(unchecked_optional_param_type = "number")] confidence: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "TailMethod")] method: Option<String>,
) -> Result<JsValue, JsValue> {
    let method: TailMethod = named(method.as_deref().unwrap_or("historical"), "method")?;
    to_js(
        &risk::rolling_tail_risk(returns, window, confidence.as_f64().unwrap_or(0.95), method)
            .map_err(error)?,
    )
}

fn levels(dates: Vec<String>, values: &[f64]) -> Result<TimeSeries<f64>, JsValue> {
    TimeSeries::new(parse_dates(dates, "dates")?, values.to_vec()).map_err(error)
}

/// Every drawdown in a series of levels, in date order.
#[wasm_bindgen(unchecked_return_type = "Drawdown[]")]
pub fn drawdowns(
    dates: Vec<String>,
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] values: &[f64],
) -> Result<JsValue, JsValue> {
    to_js(&risk::drawdowns(&levels(dates, values)?).map_err(error)?)
}

/// The fall below the running peak on each date, as a negative return.
#[wasm_bindgen(js_name = drawdownSeries)]
pub fn drawdown_series(
    dates: Vec<String>,
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] values: &[f64],
) -> Result<Vec<f64>, JsValue> {
    Ok(risk::drawdown_series(&levels(dates, values)?)
        .map_err(error)?
        .values()
        .to_vec())
}

/// Estimates the covariance matrix of a table of returns (one column per asset). The method is
/// `{method: "sample"}` by default; others are `"ewma"` (with `decay`), `"ledoit_wolf"` and
/// `"oas"`.
#[wasm_bindgen(unchecked_return_type = "CovarianceEstimate")]
pub fn covariance(
    #[wasm_bindgen(unchecked_param_type = "ReturnsTable")] returns: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "CovarianceMethod")] method: JsValue,
) -> Result<JsValue, JsValue> {
    let table: ReturnsTable = from_js(returns, "returns")?;
    let method: CovarianceMethod = if method.is_undefined() {
        CovarianceMethod::Sample
    } else {
        from_js(method, "method")?
    };
    to_js(&risk::covariance(&table, method).map_err(error)?)
}

/// The correlation matrix of a covariance matrix.
#[wasm_bindgen(unchecked_return_type = "Matrix")]
pub fn correlation(
    #[wasm_bindgen(unchecked_param_type = "Matrix")] matrix: JsValue,
) -> Result<JsValue, JsValue> {
    let matrix: Matrix = from_js(matrix, "matrix")?;
    to_js(&matrix.correlation().map_err(error)?)
}

/// Splits a portfolio's volatility into each asset's marginal and component risk.
#[wasm_bindgen(js_name = riskDecomposition, unchecked_return_type = "RiskDecomposition")]
pub fn risk_decomposition(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] weights: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Matrix")] matrix: JsValue,
) -> Result<JsValue, JsValue> {
    let matrix: Matrix = from_js(matrix, "covariance")?;
    to_js(&risk::risk_decomposition(weights, &matrix).map_err(error)?)
}

/// The ex-ante tracking error against `benchmark` weights, split into each asset's contribution.
#[wasm_bindgen(js_name = trackingErrorDecomposition, unchecked_return_type = "RiskDecomposition")]
pub fn tracking_error_decomposition(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] weights: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] benchmark: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Matrix")] matrix: JsValue,
) -> Result<JsValue, JsValue> {
    let matrix: Matrix = from_js(matrix, "covariance")?;
    to_js(&risk::tracking_error_decomposition(weights, benchmark, &matrix).map_err(error)?)
}

/// The standard normal distribution function.
#[wasm_bindgen(js_name = normalCdf)]
pub fn normal_cdf(x: f64) -> f64 {
    risk::normal_cdf(x)
}

/// The standard normal density.
#[wasm_bindgen(js_name = normalPdf)]
pub fn normal_pdf(x: f64) -> f64 {
    risk::normal_pdf(x)
}

/// The standard normal quantile.
#[wasm_bindgen(js_name = normalQuantile)]
pub fn normal_quantile(p: f64) -> f64 {
    risk::normal_quantile(p)
}

/// The optimal portfolio for `objective`. A problem with no solution returns a solution whose
/// `status` says why and whose `weights` are null.
#[wasm_bindgen(unchecked_return_type = "Solution")]
pub fn solve(
    #[wasm_bindgen(unchecked_param_type = "Problem")] problem: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Objective")] objective: JsValue,
) -> Result<JsValue, JsValue> {
    let problem: Problem = from_js(problem, "problem")?;
    let objective: Objective = from_js(objective, "objective")?;
    to_js(&problem.solve(&objective).map_err(error)?)
}

/// The efficient frontier: `points` portfolios (20 by default) from the minimum-variance
/// portfolio to the largest attainable expected return.
#[wasm_bindgen(unchecked_return_type = "FrontierPoint[]")]
pub fn frontier(
    #[wasm_bindgen(unchecked_param_type = "Problem")] problem: JsValue,
    points: Option<usize>,
) -> Result<JsValue, JsValue> {
    let problem: Problem = from_js(problem, "problem")?;
    to_js(&problem.frontier(points.unwrap_or(20)).map_err(error)?)
}

/// A rebalancing proposal as transactions, ready to add to the portfolio.
#[wasm_bindgen(js_name = rebalanceTransactions, unchecked_return_type = "Transaction[]")]
pub fn rebalance_transactions(
    #[wasm_bindgen(unchecked_param_type = "Rebalance")] plan: JsValue,
    prefix: Option<String>,
) -> Result<JsValue, JsValue> {
    let plan: Rebalance = from_js(plan, "rebalance")?;
    to_js(
        &plan
            .transactions(prefix.as_deref().unwrap_or("rebalance-"))
            .map_err(error)?,
    )
}
