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

use std::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use finalytics::formats::{
    CsvOptions, MarketDocument, PortfolioDocument, fx_from_csv, prices_from_csv,
};
use finalytics::opt::{BlackLitterman, Objective, Problem, Rebalance, Resampling};
use finalytics::performance::{FlowTiming, Period};
use finalytics::risk::{
    self, CovarianceMethod, FactorModel, Matrix, ReturnsTable, StatisticsOptions, TailMethod,
};
use finalytics::{
    AccountId, Currency, Date, Decimal, Error, HoldingKey, HoldingSeries, InstrumentId, LotMethod,
    MarketData as EngineMarket, Observation, PortfolioData, TimeSeries, ValidationReport,
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

/** A portfolio document: a portfolio's records and, optionally, its market data. */
export interface Document {
  portfolio: PortfolioData;
  market?: MarketData;
}

/** Choices for `Portfolio.rebalance`, all optional. */
export interface RebalanceChoices {
  /** The accounts to rebalance together; every account by default. */
  accounts?: string[];
  /** The date of the prices used; the last price date by default. */
  as_of?: Date;
  /** The report's title, such as `"Morgan family portfolio"`. */
  name?: string;
  /** How sales choose lots without tax rates: each account's own method by default. */
  lot_method?: LotMethod;
  /** Trade whole units only, rounding each trade towards zero. */
  whole_units?: boolean;
  /** Leave out trades worth less than this in the base currency. */
  minimum_trade?: number;
  /** The fee on each trade, in basis points of its value, such as 5 for 0.05%. */
  fee_bps?: number;
  /** A fixed fee on each trade, in the instrument's currency. */
  fee_per_trade?: number;
  /** The smallest fee a trade pays, in the instrument's currency. */
  minimum_fee?: number;
  /**
   * Tax-aware rebalancing at these rates on short-term and long-term gains, such as
   * `[0.37, 0.2]`: sales in taxable accounts take the lots that cost the least tax.
   */
  tax?: [number, number];
  /** The holding period, in days, after which a gain is long-term: 365 by default. */
  long_term_after_days?: number;
}

/**
 * The CSV text of a portfolio's files, for `Portfolio.fromCsv`. Only the transactions are
 * required; `datasets.file("portfolio/transactions.csv")` and the other dataset files show each
 * layout, and the data guide describes every column:
 * https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md
 */
export interface PortfolioFiles {
  /**
   * One row per transaction: a `date` (or `trade_date`), a `type` and the columns the type
   * needs, such as `instrument`, `quantity` and `price` for a `buy`, or `amount` and `currency`
   * for a `deposit`. Rows may give an `id` (`t1`, `t2` and so on by default) and an `account`
   * (`main` by default).
   */
  transactions: string;
  /** A `date` column and one column per instrument, or `date`, `instrument` and `price`. */
  prices?: string;
  /** A `date` column and one column per currency pair, such as `EUR/USD`. */
  fx?: string;
  /**
   * `id`, `kind` and `currency`, with `class.<scheme>` columns such as `class.asset_class`.
   * Without it, each instrument is in the currency given with it in the transactions, or else in
   * the base currency.
   */
  instruments?: string;
  /** `id`, with `name`, `lot_method` and `class.<scheme>` columns. */
  accounts?: string;
  /** The currency results are reported in; by default, that of the first deposit. */
  base_currency?: string;
  /** The field delimiter: a comma by default. */
  delimiter?: string;
}

/** The settings of `Assets`, all optional. */
export interface AssetsOptions {
  /** The annual risk-free rate, for Sharpe ratios; 0 by default. */
  risk_free_rate?: number;
  /**
   * One of the assets, such as T-bills, as the risk-free asset instead: its mean return is the
   * risk-free rate, and optimisation leaves it out.
   */
  risk_free_asset?: string;
  /** 252 for daily returns, 52 for weekly, 12 for monthly; detected from the dates by default. */
  periods_per_year?: number;
  /**
   * `"ledoit_wolf"` (the default), `"sample"`, `"oas"`, `"ewma"` (decay 0.94),
   * `"constant_correlation"`, `"denoised"` or `"nonlinear_shrinkage"`, or a `CovarianceMethod`
   * such as `{ type: "statistical", factors: 3 }`.
   */
  covariance?: CovarianceMethod["type"] | CovarianceMethod;
  /**
   * How expected returns are estimated: `"historical"` (the default), `"james_stein"` (shrunk
   * towards the assets' average) or `"ewma"` (decay 0.94), or a `MeanMethod` such as
   * `{ type: "ewma", decay: 0.97 }`.
   */
  mean_returns?: MeanMethod["type"] | MeanMethod;
}

/** Choices for `Portfolio.report`, all optional. */
export interface ReportChoices extends Partial<ReportOptions> {
  /** The portfolio's name, such as `"Morgan family portfolio"`, shown under the title. */
  name?: string;
}

/**
 * The colours of a report's page: `"auto"` (the default) follows the reader's light or dark
 * setting; `"light"` and `"dark"` keep to one.
 */
export type Theme = "auto" | "light" | "dark";

/** The ways to lay out a report: as text, as an HTML page, or shown. */
export interface ReportMethods {
  /** The report as text for printing, 80 columns wide, with `name`, if given, under the title. */
  toText(name?: string): string;
  /**
   * The report as one self-contained HTML page, with `name`, if given, as its title: inline
   * styles and SVG charts, no scripts, in `theme` (the reader's light or dark setting by
   * default), ready to print.
   */
  toHtml(name?: string, theme?: Theme): string;
  /** In Node.js, writes `toHtml(name, theme)` to `path`, and returns the path. */
  saveHtml(path: string, name?: string, theme?: Theme): string;
  /**
   * Shows the report, in `theme`. In a browser, in a frame inside `element`, or else in a new
   * window. In Node.js, in the default browser, or with the command in the `BROWSER`
   * environment variable if it is set; the page is saved to a temporary file, whose path is
   * returned.
   */
  show(element?: Element, theme?: Theme): string | undefined;
}

/** Raw results that print as text: `${results}` or `results.toText()`. */
export interface ResultsMethods {
  /** The results as text: the weights, or each strategy's periods, growth, trades and refits. */
  toText(): string;
  toString(): string;
}

/**
 * What `Assets.optimise`, `Assets.compare`, `Assets.backtest` and `Assets.plan` return: the
 * report and the raw results. `${result}` is the report as text, and `result.show()` shows it as a page.
 */
export interface Result<R, D> {
  /** The report. */
  report: R & ReportMethods;
  /** The raw results. */
  results: D & ResultsMethods;
  /** The report as text for printing, 80 columns wide, with `name`, if given, under the title. */
  toText(name?: string): string;
  /** The report as one self-contained HTML page, with `name`, if given, as its title. */
  toHtml(name?: string, theme?: Theme): string;
  /** In Node.js, writes the report's page to `path`, and returns the path. */
  saveHtml(path: string, name?: string, theme?: Theme): string;
  /** Shows the report, as `report.show` does. */
  show(element?: Element, theme?: Theme): string | undefined;
  toString(): string;
}

/** The limits on the weights and the methods' parameters, as for `Assets.optimise`. */
export type OptimiseLimits = Omit<Partial<OptimiseOptions>, "method" | "frontier">;

/** The options of `Assets.optimise`: those of `OptimiseOptions`, with the report's title. */
export interface OptimiseChoices extends Partial<OptimiseOptions> {
  /** The report's title, such as `"Defensive equity"`: the method's name by default. */
  name?: string;
}

/**
 * How strategies are validated: `"in_sample"` (the default); `{ holdout: { start: "2016-11-01" } }`,
 * fitted on the history before the date and tested on the rest; or
 * `{ walk_forward: { years: 5 } }`, refitted at each scheduled rebalance on the last five years
 * (or on all the history so far, with `expanding: true`).
 */
export type ValidationChoice =
  | "in_sample"
  | { holdout: { start: Date } }
  | { walk_forward: { years?: number; periods?: number; expanding?: boolean } }
  | Validation;

/**
 * When the portfolio trades back to its targets: `"monthly"`, `"quarterly"` (the default),
 * `"yearly"`, `"never"`, `{ drift: { band: 0.05 } }` when a weight drifts five points away, or
 * `{ every: { periods: 21 } }` after every 21 periods.
 */
export type RebalanceChoice =
  | "monthly"
  | "quarterly"
  | "yearly"
  | "never"
  | { drift: { band: number } }
  | { every: { periods: number } }
  | RebalanceSchedule;

/** How `Assets.compare` and `Assets.backtest` run, all optional. */
export interface BacktestSettings {
  /** How the strategies are validated: in sample by default. */
  validation?: ValidationChoice;
  /** When the portfolio trades back to its targets: quarterly by default. */
  rebalance?: RebalanceChoice;
  /** The cost of trading, in basis points of the value traded: 10 is 0.1%. */
  trading_cost_bps?: number;
  /**
   * What the strategies are compared with: weights by asset, such as
   * `{ "US Equity": 0.6, Bonds: 0.4 }`, or an index's prices, such as
   * `await datasets.sp500Index()`.
   */
  benchmark?: Record<string, number> | PriceTable | Benchmark;
  /** The benchmark's name: `"Benchmark"` by default, or the prices' column name. */
  benchmark_name?: string;
  /** The value each run starts with, to follow money: the reports then give each run's ending value and money-weighted return. */
  start_value?: number;
  /** Money paid in (positive) or taken out (negative) on dates, such as `[{ date: "2020-01-31", amount: 500 }]`. */
  flows?: CashFlow[];
}

/** Choices for `Assets.compare`: the methods, with limits and settings applied to each. */
export interface CompareChoices extends OptimiseLimits, BacktestSettings {
  /** The methods to compare, such as `["equal_weight", "minimum_variance"]`. */
  methods?: Method[];
  /**
   * Fixed weights by name, such as `{ "60/40": { "US Equity": 0.6, Bonds: 0.4 } }`, held and
   * rebalanced as the methods are and scored on the same periods; not fitted to any returns.
   */
  weights?: Record<string, Record<string, number>>;
  /** The report's title, such as `"S&P 500 stocks"`. */
  name?: string;
}

/** Choices for `Assets.backtest`: a method, with its limits, or fixed weights. */
export type BacktestChoices =
  | (OptimiseLimits &
      BacktestSettings & {
        /** The method, refitted as the validation says. */
        method: Method;
        weights?: undefined;
        /** The name of the strategy, and the report's title; the method's own by default. */
        name?: string;
      })
  | (BacktestSettings & {
      /** Fixed weights by asset, adding up to one; a weight on the risk-free asset is cash. */
      weights: Record<string, number>;
      method?: undefined;
      /** The name of the strategy, and the report's title: `Weights` by default. */
      name?: string;
    });

/** An investment mix: weights by asset, adding up to one, or a `Mix`, such as a glide path. */
export type MixChoice = Record<string, number> | Mix;

/**
 * Choices for `Assets.plan`: those of `PlanOptions`, with plainer forms for the mixes, the
 * inflation, the chance and the model.
 */
export interface PlanChoices
  extends Partial<Omit<PlanOptions, "weights" | "alternatives" | "inflation" | "chance" | "model">> {
  /**
   * The goal: `{ type: "target", amount: 150000, by: "2030-06" }`, an amount by a month, or
   * `{ type: "payments", amount: 4000, start: "2046-11", end: "2076-10" }`, paid every month;
   * none, with `end`, for a projection.
   */
  goal?: Goal;
  /** What the money is invested in, rebalanced every month. */
  weights: MixChoice;
  /** Other mixes to compare on the same simulations: weights by name, or named mixes. */
  alternatives?: Record<string, MixChoice> | Mix[];
  /** The inflation a year, such as 0.025, or a price index of one column, such as `datasets.usCpi()`. */
  inflation: number | PriceTable | Inflation;
  /** The chance wanted, 0.8 by default, or it and the chance from which there is room to spare. */
  chance?: number | [number, number];
  /** How markets are simulated: `"bootstrap"` (the default), `"normal"` or `"historical"`, or a `Model`. */
  model?: "bootstrap" | "normal" | "historical" | Model;
}

/** Choices for `Portfolio.shockScenario` and `Portfolio.historicalScenario`, all optional. */
export interface ScenarioChoices extends Partial<ScenarioOptions> {
  /** The date of the holdings; the last price date by default. */
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
    // JSON has no NaN or Infinity: written as JSON they would become null, read as absent.
    // Prices are the exception, where a missing price may be given as NaN.
    let mut found = Vec::new();
    if !matches!(name, "prices" | "market") {
        non_finite(&value, name, &mut found, 0);
    }
    if !found.is_empty() {
        let mut report = ValidationReport::new();
        for (path, x) in found {
            report.error(path, format!("must be a finite number, not {x}"));
        }
        return Err(error(Error::Validation(report)));
    }
    let text: String = js_sys::JSON::stringify(&value)
        .map_err(|_| invalid(name, "cannot be written as JSON"))?
        .into();
    finalytics::formats::from_json(&text, name).map_err(error)
}

/// The paths of the numbers in `value` that are not finite, with the numbers: at most 20, as
/// one mistake tends to repeat.
fn non_finite(value: &JsValue, path: &str, found: &mut Vec<(String, f64)>, depth: usize) {
    if depth > 32 || found.len() >= 20 {
        return;
    }
    if let Some(x) = value.as_f64() {
        if !x.is_finite() {
            found.push((path.to_owned(), x));
        }
    } else if js_sys::Array::is_array(value) {
        for (i, item) in js_sys::Array::from(value).iter().enumerate() {
            non_finite(&item, &format!("{path}[{i}]"), found, depth + 1);
        }
    } else if value.is_object() && !value.is_function() {
        for entry in js_sys::Object::entries(value.unchecked_ref()).iter() {
            let pair = js_sys::Array::from(&entry);
            let key = pair.get(0).as_string().unwrap_or_default();
            non_finite(&pair.get(1), &format!("{path}.{key}"), found, depth + 1);
        }
    }
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
        Some([byte]) => Ok({
            let mut csv_options = CsvOptions::default();
            csv_options.delimiter = *byte;
            csv_options
        }),
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

/// A portfolio: its records (base currency, accounts, instruments and transactions), the market
/// data that values it, and the books kept from them. It does not change: `withTransactions` and
/// `withRebalance` return new portfolios.
#[wasm_bindgen]
pub struct Portfolio {
    inner: finalytics::Portfolio,
    /// The digest of the inputs, computed for the first report that needs it.
    source: std::cell::OnceCell<finalytics::reports::Source>,
}

impl From<finalytics::Portfolio> for Portfolio {
    fn from(inner: finalytics::Portfolio) -> Portfolio {
        Portfolio {
            inner,
            source: std::cell::OnceCell::new(),
        }
    }
}

/// The CSV text of a portfolio's files, for `Portfolio.fromCsv`.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PortfolioFiles {
    transactions: String,
    #[serde(default)]
    prices: Option<String>,
    #[serde(default)]
    fx: Option<String>,
    #[serde(default)]
    instruments: Option<String>,
    #[serde(default)]
    accounts: Option<String>,
    #[serde(default)]
    base_currency: Option<String>,
    #[serde(default)]
    delimiter: Option<String>,
}

#[wasm_bindgen]
impl Portfolio {
    /// A portfolio from its records and market data (pass `new MarketData()` for none): the
    /// books are kept by applying the transactions in order.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PortfolioData")] data: JsValue,
        market: &MarketData,
    ) -> Result<Portfolio, JsValue> {
        let data: PortfolioData = from_js(data, "portfolio")?;
        Ok(finalytics::Portfolio::new(data, market.inner.clone())
            .map_err(error)?
            .into())
    }

    /// A portfolio from the CSV text of its files; only the transactions are required. Every
    /// problem in every file is reported at once, by file, line and column.
    #[wasm_bindgen(js_name = fromCsv)]
    pub fn from_csv(
        #[wasm_bindgen(unchecked_param_type = "PortfolioFiles")] files: JsValue,
    ) -> Result<Portfolio, JsValue> {
        let files: PortfolioFiles = from_js(files, "files")?;
        let mut csv = finalytics::PortfolioCsv::new(&files.transactions);
        if let Some(text) = &files.prices {
            csv = csv.prices(text);
        }
        if let Some(text) = &files.fx {
            csv = csv.fx(text);
        }
        if let Some(text) = &files.instruments {
            csv = csv.instruments(text);
        }
        if let Some(text) = &files.accounts {
            csv = csv.accounts(text);
        }
        if let Some(code) = &files.base_currency {
            csv = csv.base_currency(code);
        }
        csv = csv.delimiter(csv_options(files.delimiter)?.delimiter);
        Ok(finalytics::Portfolio::from_csv(&csv).map_err(error)?.into())
    }

    /// A portfolio from a portfolio document in JSON text.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(text: &str) -> Result<Portfolio, JsValue> {
        Ok(finalytics::Portfolio::from_json(text)
            .map_err(error)?
            .into())
    }

    /// The portfolio document of the portfolio and its market data, in JSON text.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// The records: base currency, accounts, instruments and transactions.
    #[wasm_bindgen(getter, unchecked_return_type = "PortfolioData")]
    pub fn data(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.data())
    }

    /// The market data.
    #[wasm_bindgen(getter)]
    pub fn market(&self) -> MarketData {
        MarketData {
            inner: self.inner.market().clone(),
        }
    }

    /// The last date with a price: the date the portfolio is valued on when no other is given.
    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> Option<String> {
        self.inner.as_of().map(|d| d.to_string())
    }

    /// The same portfolio with more transactions, applied after its own.
    #[wasm_bindgen(js_name = withTransactions)]
    pub fn with_transactions(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Transaction[]")] transactions: JsValue,
    ) -> Result<Portfolio, JsValue> {
        let transactions: Vec<finalytics::Transaction> = from_js(transactions, "transactions")?;
        Ok(self
            .inner
            .with_transactions(transactions)
            .map_err(error)?
            .into())
    }

    /// The same portfolio with the trades of a rebalancing made on its date: the result of
    /// `rebalance`, or its raw results.
    #[wasm_bindgen(js_name = withRebalance)]
    pub fn with_rebalance(
        &self,
        #[wasm_bindgen(
            unchecked_param_type = "Result<RebalanceReport, RebalanceResults> | RebalanceResults"
        )]
        proposal: JsValue,
    ) -> Result<Portfolio, JsValue> {
        let mut value: serde_json::Value = from_js(proposal, "proposal")?;
        if let Some(results) = value.get_mut("results").filter(|v| v.is_object()) {
            value = results.take();
        }
        let proposal: finalytics::opt::RebalanceResults =
            finalytics::formats::from_json(&value.to_string(), "proposal").map_err(error)?;
        Ok(self.inner.with_rebalance(&proposal).map_err(error)?.into())
    }

    /// The same portfolio valued with other market data.
    #[wasm_bindgen(js_name = withMarket)]
    pub fn with_market(&self, market: &MarketData) -> Result<Portfolio, JsValue> {
        Ok(self
            .inner
            .with_market(market.inner.clone())
            .map_err(error)?
            .into())
    }

    /// The instruments held at the last price date, as `Assets`: their price returns in the
    /// base currency on the portfolio's dates, for statistics and optimisation.
    pub fn assets(&self) -> Result<Assets, JsValue> {
        Ok(Assets {
            inner: self.inner.assets().map_err(error)?,
        })
    }

    /// Every report on the portfolio, from one set of options. The attribution report needs
    /// `attribution` and the scenarios report `scenarios`; without them, they are absent.
    /// `name`, such as `"Morgan family portfolio"`, is shown under the report's title.
    #[wasm_bindgen(unchecked_return_type = "PortfolioReport & ReportMethods")]
    pub fn report(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "ReportChoices")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut choices = object(
            options,
            &fields::<finalytics::reports::ReportOptions>(&["name"]),
        )?;
        let name = match choices.remove("name") {
            Some(serde_json::Value::String(name)) => Some(name),
            Some(serde_json::Value::Null) | None => None,
            Some(_) => return Err(invalid("options.name", "must be a string")),
        };
        let options: finalytics::reports::ReportOptions = finalytics::formats::from_json(
            &serde_json::Value::Object(choices).to_string(),
            "options",
        )
        .map_err(error)?;
        let mut report = self.inner.report_with(&options).map_err(error)?;
        report.name = name;
        to_js(&report)
    }

    /// The digest and size of the inputs the reports are computed from.
    #[wasm_bindgen(unchecked_return_type = "Source")]
    pub fn source(&self) -> Result<JsValue, JsValue> {
        to_js(self.report_source())
    }

    /// Positions, lots and cash by holding and by account; allocation by every dimension;
    /// concentration; realised gains and income.
    #[wasm_bindgen(js_name = holdingsReport, unchecked_return_type = "HoldingsReport")]
    pub fn holdings_report(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::holdings_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// Headline figures, growth, allocation, risk by holding and by group, each holding's
    /// expected return and risk, and the efficient frontier.
    #[wasm_bindgen(js_name = overviewReport, unchecked_return_type = "OverviewReport")]
    pub fn overview_report(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::overview_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// Value and growth, returns over the standard periods, monthly and calendar-year returns,
    /// contribution, flows and income, and the household.
    #[wasm_bindgen(js_name = performanceReport, unchecked_return_type = "PerformanceReport")]
    pub fn performance_report(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::performance_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// Statistics, drawdowns, tail risk, the return distribution, the holdings' covariance,
    /// correlation and risk decomposition, and rolling statistics.
    #[wasm_bindgen(js_name = riskReport, unchecked_return_type = "RiskReport")]
    pub fn risk_report(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::risk_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// Brinson–Fachler attribution against the benchmark segments in the options.
    #[wasm_bindgen(js_name = attributionReport, unchecked_return_type = "AttributionReport")]
    pub fn attribution_report(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::attribution_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// The scenarios in the options, run together on the current holdings, side by side.
    #[wasm_bindgen(js_name = scenariosReport, unchecked_return_type = "ScenariosReport")]
    pub fn scenarios_report(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Partial<ReportOptions>")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options = report_options(options)?;
        let r = finalytics::reports::scenarios_report(
            self.inner.books(),
            self.inner.market(),
            self.report_source(),
            &options,
        );
        to_js(&r.map_err(error)?)
    }

    /// The currency results are reported in.
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> String {
        self.inner.books().base_currency().to_string()
    }

    /// The dates the portfolio is measured on when none are given: every date with a price,
    /// from the first transaction to the last price.
    pub fn dates(&self) -> Vec<String> {
        date_strings(&self.inner.dates())
    }

    fn dates_or_default(&self, given: Option<Vec<String>>) -> Result<Vec<Date>, JsValue> {
        match given {
            Some(d) => parse_dates(d, "dates"),
            None => Ok(self.inner.dates()),
        }
    }

    fn date_or_last(&self, date: Option<String>) -> Result<Date, JsValue> {
        match date {
            Some(d) => parse_date(&d),
            None => self
                .inner
                .as_of()
                .ok_or_else(|| invalid("date", "give a date: the portfolio has no prices")),
        }
    }

    /// Positions, lots and cash at the end of `date`, or after the last transaction.
    #[wasm_bindgen(unchecked_return_type = "Holdings")]
    pub fn holdings(&self, date: Option<String>) -> Result<JsValue, JsValue> {
        let holdings = match optional_date(date)? {
            Some(d) => self.inner.books().holdings_on(d),
            None => self.inner.books().holdings().clone(),
        };
        let lots: Vec<_> = holdings.lots().collect();
        to_js(&serde_json::json!({
            "positions": holdings.positions(),
            "lots": lots,
            "cash": holdings.cash_balances(),
        }))
    }

    /// The holdings valued at the end of `date`, by default the last price date.
    #[wasm_bindgen(unchecked_return_type = "Valuation")]
    pub fn valuation(&self, date: Option<String>) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .valuation_on(&self.date_or_last(date)?.to_string())
                .map_err(error)?,
        )
    }

    /// End-of-day values and external flows on `dates` (by default `dates()`).
    #[wasm_bindgen(js_name = valueSeries, unchecked_return_type = "ValueSeries")]
    pub fn value_series(&self, dates: Option<Vec<String>>) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(dates)?;
        to_js(
            &self
                .inner
                .books()
                .value_series(&dates, self.inner.market())
                .map_err(error)?,
        )
    }

    /// Each holding's daily value, trade cash and income on `dates`.
    #[wasm_bindgen(js_name = holdingSeries, unchecked_return_type = "HoldingSeries")]
    pub fn holding_series(&self, dates: Option<Vec<String>>) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(dates)?;
        to_js(
            &self
                .inner
                .books()
                .holding_series(&dates, self.inner.market())
                .map_err(error)?,
        )
    }

    /// Every realised gain, by lot.
    #[wasm_bindgen(js_name = realisedGains, unchecked_return_type = "RealisedGain[]")]
    pub fn realised_gains(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.books().realised_gains())
    }

    /// What each transaction did to the books, in the order applied.
    #[wasm_bindgen(unchecked_return_type = "Event[]")]
    pub fn events(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.books().events())
    }

    /// Each account's end-of-day value and external flows on `dates` (by default
    /// `dates()`).
    #[wasm_bindgen(js_name = accountSeries, unchecked_return_type = "AccountSeries")]
    pub fn account_series(&self, dates: Option<Vec<String>>) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(dates)?;
        to_js(
            &self
                .inner
                .books()
                .account_series(&dates, self.inner.market())
                .map_err(error)?,
        )
    }

    /// Performance over `dates` (by default `dates()`): every account together, each
    /// account, and the groups of accounts by each classification scheme in the options, with
    /// each one's contribution to the household's time-weighted return.
    #[wasm_bindgen(unchecked_return_type = "Household")]
    pub fn household(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Date[]")] dates: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "HouseholdOptions")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let dates = self.dates_or_default(optional(dates, "dates")?)?;
        let options: finalytics::performance::HouseholdOptions = optional(options, "options")?;
        to_js(
            &finalytics::performance::household(
                self.inner.books(),
                self.inner.market(),
                &dates,
                &options,
            )
            .map_err(error)?,
        )
    }

    /// Time-weighted performance on `dates` (by default `dates()`), with flows at the
    /// start (default) or end of the day.
    pub fn performance(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "Date[]")] dates: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "FlowTiming")] timing: Option<String>,
    ) -> Result<Performance, JsValue> {
        let dates = self.dates_or_default(optional(dates, "dates")?)?;
        let timing: FlowTiming = named(timing.as_deref().unwrap_or("start_of_day"), "timing")?;
        let values = self
            .inner
            .books()
            .value_series(&dates, self.inner.market())
            .map_err(error)?;
        let holdings = self
            .inner
            .books()
            .holding_series(&dates, self.inner.market())
            .map_err(error)?;
        Ok(Performance {
            inner: finalytics::performance::Performance::from_values(&values, timing)
                .map_err(error)?,
            holdings: Some(holdings),
            warnings: values.warnings,
        })
    }

    /// Each holding's share of the portfolio's value at the end of `date`, by default the last
    /// price date.
    #[wasm_bindgen(unchecked_return_type = "Record<HoldingKey, number>")]
    pub fn weights(&self, date: Option<String>) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .weights_on(&self.date_or_last(date)?.to_string())
                .map_err(error)?,
        )
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
        to_js(
            &risk::holding_returns(self.inner.books(), &holdings, &dates, self.inner.market())
                .map_err(error)?,
        )
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
            .inner
            .books()
            .holdings_on(parse_date(date)?)
            .lots_for_sale(&parse_account(account)?, &instrument(id)?, quantity, method)
            .map_err(error)?;
        to_js(&sale)
    }

    /// The trades that move the accounts (by default every account; `choices.accounts` names
    /// others) together to `targets`, as shares of their combined value: weights by holding, or
    /// an optimisation of the portfolio's assets, whose weights are its targets (one that found
    /// no weights is a `ValidationError`). The rebalance report is in `report`, and each
    /// account's proposal in `results`; `withRebalance` makes the trades.
    #[wasm_bindgen(unchecked_return_type = "Result<RebalanceReport, RebalanceResults>")]
    pub fn rebalance(
        &self,
        #[wasm_bindgen(
            unchecked_param_type = "Record<HoldingKey, number> | Result<OptimisationReport, OptimisationResult> | OptimisationResult"
        )]
        targets: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "RebalanceChoices")] choices: JsValue,
    ) -> Result<JsValue, JsValue> {
        let targets: serde_json::Value = from_js(targets, "targets")?;
        let targets = finalytics::targets_from_json(&targets.to_string()).map_err(error)?;
        let choices: finalytics::RebalanceChoices = optional(choices, "choices")?;
        let builder = choices.apply(self.inner.rebalance(targets));
        to_js(&builder.build().map_err(error)?)
    }

    /// What `shocks` would do to the holdings, with their risk before and after if the choices
    /// give a risk model.
    #[wasm_bindgen(js_name = shockScenario, unchecked_return_type = "ScenarioResult")]
    pub fn shock_scenario(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Shocks")] shocks: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "ScenarioChoices")] choices: JsValue,
    ) -> Result<JsValue, JsValue> {
        let shocks: finalytics::risk::Shocks = from_js(shocks, "shocks")?;
        let (on, options) = self.scenario_choices(choices)?;
        let result = finalytics::risk::shock_scenario(
            self.inner.books(),
            self.inner.market(),
            on,
            &shocks,
            &options,
        )
        .map_err(error)?;
        to_js(&result)
    }

    /// What the price and exchange-rate changes over a historical period would do to the
    /// holdings, with their value on each date of the period.
    #[wasm_bindgen(js_name = historicalScenario, unchecked_return_type = "ScenarioResult")]
    pub fn historical_scenario(
        &self,
        #[wasm_bindgen(unchecked_param_type = "HistoricalWindow")] window: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "ScenarioChoices")] choices: JsValue,
    ) -> Result<JsValue, JsValue> {
        let window: finalytics::risk::HistoricalWindow = from_js(window, "window")?;
        let (on, options) = self.scenario_choices(choices)?;
        let result = finalytics::risk::historical_scenario(
            self.inner.books(),
            self.inner.market(),
            on,
            &window,
            &options,
        )
        .map_err(error)?;
        to_js(&result)
    }
}

impl Portfolio {
    fn report_source(&self) -> &finalytics::reports::Source {
        self.source
            .get_or_init(|| finalytics::reports::Source::of(self.inner.data(), self.inner.market()))
    }

    fn scenario_choices(
        &self,
        choices: JsValue,
    ) -> Result<(Date, finalytics::risk::ScenarioOptions), JsValue> {
        self.dated_choices(optional(choices, "choices")?)
    }

    /// Options with an optional `date` among them, by default the last price date.
    fn dated_choices<T: DeserializeOwned>(
        &self,
        mut choices: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(Date, T), JsValue> {
        let on = match choices.remove("date") {
            Some(serde_json::Value::String(d)) => parse_date(&d)?,
            _ => self.date_or_last(None)?,
        };
        let options = finalytics::formats::from_json(
            &serde_json::Value::Object(choices).to_string(),
            "choices",
        )
        .map_err(error)?;
        Ok((on, options))
    }
}

/// The prices or returns of several assets over the same periods, with the settings that turn
/// them into annual figures, for statistics and optimisation.
#[wasm_bindgen]
pub struct Assets {
    inner: finalytics::Assets,
}

#[wasm_bindgen]
impl Assets {
    /// Assets from prices: dates, labels and one column of prices per asset (`null` where there
    /// is none). Dates on which an asset has no price are left out, with a warning.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PriceTable")] prices: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "AssetsOptions")] options: JsValue,
    ) -> Result<Assets, JsValue> {
        let prices: finalytics::PriceTable = from_js(prices, "prices")?;
        let assets = finalytics::Assets::from_prices(&prices).map_err(error)?;
        let options: finalytics::AssetSettings = optional(options, "options")?;
        Ok(Assets {
            inner: options.apply(assets).map_err(error)?,
        })
    }

    /// Assets from prices in CSV text: a `date` column and one column per asset.
    #[wasm_bindgen(js_name = fromCsv)]
    pub fn from_csv(
        text: &str,
        #[wasm_bindgen(unchecked_optional_param_type = "AssetsOptions")] options: JsValue,
    ) -> Result<Assets, JsValue> {
        let assets = finalytics::Assets::from_csv(text).map_err(error)?;
        let options: finalytics::AssetSettings = optional(options, "options")?;
        Ok(Assets {
            inner: options.apply(assets).map_err(error)?,
        })
    }

    /// Assets from their returns: the date each return ends on, the labels, and one column of
    /// returns per asset.
    #[wasm_bindgen(js_name = fromReturns)]
    pub fn from_returns(
        dates: Vec<String>,
        labels: Vec<String>,
        #[wasm_bindgen(unchecked_param_type = "(Float64Array | number[])[]")] columns: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "AssetsOptions")] options: JsValue,
    ) -> Result<Assets, JsValue> {
        let columns: Vec<Vec<f64>> = from_js(columns, "columns")?;
        let table = ReturnsTable::new(labels, columns).map_err(error)?;
        let assets =
            finalytics::Assets::from_returns(parse_dates(dates, "dates")?, table).map_err(error)?;
        let options: finalytics::AssetSettings = optional(options, "options")?;
        Ok(Assets {
            inner: options.apply(assets).map_err(error)?,
        })
    }

    /// The assets.
    #[wasm_bindgen(getter)]
    pub fn labels(&self) -> Vec<String> {
        self.inner.labels().to_vec()
    }

    /// The date each return ends on.
    #[wasm_bindgen(getter)]
    pub fn dates(&self) -> Vec<String> {
        date_strings(self.inner.dates())
    }

    /// The returns, one array per asset.
    #[wasm_bindgen(getter, unchecked_return_type = "Float64Array[]")]
    pub fn returns(&self) -> js_sys::Array {
        self.inner
            .returns()
            .columns()
            .iter()
            .map(|c| JsValue::from(js_sys::Float64Array::from(c.as_slice())))
            .collect()
    }

    /// The number of return periods in a year.
    #[wasm_bindgen(getter, js_name = periodsPerYear)]
    pub fn periods_per_year(&self) -> f64 {
        self.inner.periods_per_year()
    }

    /// The annual risk-free rate.
    #[wasm_bindgen(getter, js_name = riskFreeRate)]
    pub fn risk_free_rate(&self) -> f64 {
        self.inner.risk_free_rate()
    }

    /// The risk-free asset, if one of the assets is.
    #[wasm_bindgen(getter, js_name = riskFreeAsset)]
    pub fn risk_free_asset(&self) -> Option<String> {
        self.inner.risk_free_asset().map(str::to_owned)
    }

    /// Problems met reading the prices that did not stop the assets being built.
    #[wasm_bindgen(getter, unchecked_return_type = "Issue[]")]
    pub fn warnings(&self) -> Result<JsValue, JsValue> {
        to_js(self.inner.warnings().issues())
    }

    /// Each asset's statistics, with its value at risk and expected shortfall over one period
    /// at 95%, in the order of the assets.
    #[wasm_bindgen(unchecked_return_type = "AssetStatistics[]")]
    pub fn statistics(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.statistics().map_err(error)?)
    }

    /// The covariance of the assets' returns, annualised.
    #[wasm_bindgen(unchecked_return_type = "CovarianceEstimate")]
    pub fn covariance(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.covariance().map_err(error)?)
    }

    /// The correlation of the assets' returns.
    #[wasm_bindgen(unchecked_return_type = "Matrix")]
    pub fn correlation(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.correlation().map_err(error)?)
    }

    /// Each asset's mean return, annualised, by the `mean_returns` estimator (the risk-free
    /// asset's is its historical mean): the expected returns optimisation uses unless others are
    /// given.
    #[wasm_bindgen(js_name = meanReturns, unchecked_return_type = "Record<string, number>")]
    pub fn mean_returns(&self) -> Result<JsValue, JsValue> {
        let means: BTreeMap<String, f64> = self
            .inner
            .labels()
            .iter()
            .cloned()
            .zip(self.inner.mean_returns())
            .collect();
        self.in_asset_order(&means)
    }

    /// The optimisation of the risky assets by the method of `options` (the largest Sharpe
    /// ratio by default), fully invested, within its limits: the optimisation report in
    /// `report`, and the weights, in the order of the assets, with the solver's solution in
    /// `results`.
    #[wasm_bindgen(unchecked_return_type = "Result<OptimisationReport, OptimisationResult>")]
    pub fn optimise(
        &self,
        #[wasm_bindgen(unchecked_optional_param_type = "OptimiseChoices")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut options = object(
            options,
            &fields::<finalytics::OptimiseOptions>(&["frontier", "name"]),
        )?;
        let name = match options.remove("name") {
            Some(serde_json::Value::String(name)) => Some(name),
            Some(serde_json::Value::Null) | None => None,
            Some(_) => return Err(invalid("options.name", "must be a string")),
        };
        let options: finalytics::OptimiseOptions = finalytics::formats::from_json(
            &serde_json::Value::Object(options).to_string(),
            "options",
        )
        .map_err(error)?;
        let mut result = self.inner.optimise_with(&options).map_err(error)?;
        result.report.name = name;
        let value = to_js(&result)?;
        if let Some(weights) = &result.results.weights {
            // In the order of the assets, rather than of their names.
            let results = js_sys::Reflect::get(&value, &"results".into())?;
            js_sys::Reflect::set(&results, &"weights".into(), &self.in_asset_order(weights)?)?;
        }
        Ok(value)
    }

    /// The comparison of `options.methods`, each backtested on these assets with the same
    /// limits, rebalancing, validation and trading costs: the comparison report in `report`,
    /// and every return and trade in `results`.
    #[wasm_bindgen(unchecked_return_type = "Result<ComparisonReport, BacktestResults>")]
    pub fn compare(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CompareChoices")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut allowed = fields::<finalytics::OptimiseOptions>(&["methods", "weights", "name"]);
        allowed.retain(|name| name != "method");
        allowed.extend(SETTINGS.map(str::to_owned));
        let mut choices = object(options, &allowed)?;
        let methods = choices.remove("methods");
        let fixed = choices.remove("weights");
        if methods.is_none() && fixed.is_none() {
            return Err(invalid(
                "options.methods",
                "give the methods to compare, or fixed weights",
            ));
        }
        let methods: Vec<finalytics::Method> = match methods {
            Some(m) => {
                finalytics::formats::from_json(&m.to_string(), "options.methods").map_err(error)?
            }
            None => Vec::new(),
        };
        let fixed: BTreeMap<String, BTreeMap<String, f64>> = match fixed {
            Some(w) => {
                finalytics::formats::from_json(&w.to_string(), "options.weights").map_err(error)?
            }
            None => BTreeMap::new(),
        };
        let name = match choices.remove("name") {
            Some(serde_json::Value::String(name)) => Some(name),
            Some(serde_json::Value::Null) | None => None,
            Some(_) => return Err(invalid("options.name", "must be a string")),
        };
        let mut settings = settings(&mut choices)?;
        let limits = limits(choices)?;
        let strategies = methods
            .into_iter()
            .map(|method| {
                let mut options = limits.clone();
                options.insert("method".to_owned(), serde_json::json!(method));
                serde_json::json!({ "type": "optimise", "options": options })
            })
            .chain(fixed.into_iter().map(|(name, weights)| {
                serde_json::json!({ "type": "weights", "name": name, "weights": weights })
            }))
            .collect();
        settings.insert(
            "strategies".to_owned(),
            serde_json::Value::Array(strategies),
        );
        let mut comparison = self
            .inner
            .compare_with(&backtest_options(settings)?)
            .map_err(error)?;
        comparison.report.name = name;
        to_js(&comparison)
    }

    /// The backtest of one strategy: `options.method`, fitted as the validation says, or fixed
    /// `options.weights` by asset (a weight on the risk-free asset is held as cash), against the
    /// benchmark if one is given: the backtest report in `report`, and every return and trade
    /// in `results`.
    #[wasm_bindgen(unchecked_return_type = "Result<BacktestReport, BacktestResults>")]
    pub fn backtest(
        &self,
        #[wasm_bindgen(unchecked_param_type = "BacktestChoices")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut allowed = fields::<finalytics::OptimiseOptions>(&["weights", "name"]);
        allowed.extend(SETTINGS.map(str::to_owned));
        let mut choices = object(options, &allowed)?;
        let method = choices.remove("method");
        let weights = choices.remove("weights");
        let name = choices.remove("name").unwrap_or(serde_json::Value::Null);
        let mut settings = settings(&mut choices)?;
        let strategy = match (method, weights) {
            (Some(method), None) => {
                let mut options = limits(choices)?;
                options.insert("method".to_owned(), method);
                serde_json::json!({ "type": "optimise", "name": name, "options": options })
            }
            (None, Some(weights)) => {
                let mut report = ValidationReport::new();
                for key in choices.keys() {
                    report.error(
                        format!("options.{key}"),
                        "fixed weights take no limits or parameters",
                    );
                }
                report.into_result(()).map_err(error)?;
                serde_json::json!({ "type": "weights", "name": name, "weights": weights })
            }
            (None, None) => {
                return Err(invalid("options.method", "give a method, or fixed weights"));
            }
            (Some(_), Some(_)) => {
                return Err(invalid(
                    "options.weights",
                    "give a method or fixed weights, not both",
                ));
            }
        };
        settings.insert(
            "strategies".to_owned(),
            serde_json::Value::Array(vec![strategy]),
        );
        to_js(
            &self
                .inner
                .backtest_with(&backtest_options(settings)?)
                .map_err(error)?,
        )
    }

    /// The chance that a plan of saving, spending and investing meets `options.goal`, from
    /// `options.simulations` markets drawn from these assets' monthly returns: the planning
    /// report in `report`, and the raw results in `results`.
    #[wasm_bindgen(unchecked_return_type = "Result<PlanReport, PlanResults>")]
    pub fn plan(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PlanChoices")] options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut choices = object(options, &fields::<finalytics::PlanOptions>(&[]))?;
        let mix = |value: serde_json::Value, name: Option<&str>| -> serde_json::Value {
            let mut mix = match value {
                serde_json::Value::Object(map) if map.contains_key("type") => map,
                weights => serde_json::Map::from_iter([
                    ("type".to_owned(), "fixed".into()),
                    ("weights".to_owned(), weights),
                ]),
            };
            if let Some(name) = name {
                mix.insert("name".to_owned(), name.into());
            }
            serde_json::Value::Object(mix)
        };
        if let Some(weights) = choices.remove("weights") {
            choices.insert("weights".to_owned(), mix(weights, None));
        }
        match choices.remove("alternatives") {
            Some(serde_json::Value::Object(named)) => {
                let mixes = named
                    .into_iter()
                    .map(|(name, weights)| mix(weights, Some(&name)))
                    .collect();
                choices.insert("alternatives".to_owned(), serde_json::Value::Array(mixes));
            }
            Some(serde_json::Value::Array(list)) => {
                let mixes = list.into_iter().map(|m| mix(m, None)).collect();
                choices.insert("alternatives".to_owned(), serde_json::Value::Array(mixes));
            }
            Some(other) => {
                choices.insert("alternatives".to_owned(), other);
            }
            None => {}
        }
        match choices.remove("inflation") {
            Some(serde_json::Value::Number(rate)) => {
                choices.insert(
                    "inflation".to_owned(),
                    serde_json::json!({ "type": "rate", "rate": rate }),
                );
            }
            Some(serde_json::Value::Object(map)) if !map.contains_key("type") => {
                choices.insert(
                    "inflation".to_owned(),
                    serde_json::json!({ "type": "index", "prices": map }),
                );
            }
            Some(other) => {
                choices.insert("inflation".to_owned(), other);
            }
            None => {}
        }
        if let Some(serde_json::Value::Array(pair)) = choices.get("chance").cloned()
            && let [wanted, spare] = pair.as_slice()
        {
            choices.insert("chance".to_owned(), wanted.clone());
            choices.insert("spare".to_owned(), spare.clone());
        }
        if let Some(serde_json::Value::String(model)) = choices.get("model").cloned() {
            choices.insert("model".to_owned(), serde_json::json!({ "type": model }));
        }
        if let (Some(name), Some(serde_json::Value::Object(goal))) =
            (choices.get("name").cloned(), choices.get_mut("goal"))
        {
            goal.entry("name").or_insert(name);
        }
        let options: finalytics::PlanOptions = finalytics::formats::from_json(
            &serde_json::Value::Object(choices).to_string(),
            "options",
        )
        .map_err(error)?;
        to_js(&self.inner.plan_with(&options).map_err(error)?)
    }

    /// The efficient frontier: `points` portfolios (20 by default) of the risky assets, under
    /// the limits of `options`.
    #[wasm_bindgen(unchecked_return_type = "FrontierPortfolio[]")]
    pub fn frontier(
        &self,
        points: Option<usize>,
        #[wasm_bindgen(unchecked_optional_param_type = "Partial<OptimiseOptions>")]
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        // The same options as `optimise`, each checked by name.
        let options = object(options, &fields::<finalytics::OptimiseOptions>(&[]))?;
        let options: finalytics::OptimiseOptions = finalytics::formats::from_json(
            &serde_json::Value::Object(options).to_string(),
            "options",
        )
        .map_err(error)?;
        let points = self
            .inner
            .frontier(points.unwrap_or(20), &options)
            .map_err(error)?;
        let value = to_js(&points)?;
        // Each portfolio's weights in the order of the assets, rather than of their names.
        let array = js_sys::Array::from(&value);
        for (k, point) in points.iter().enumerate() {
            js_sys::Reflect::set(
                &array.get(k as u32),
                &"weights".into(),
                &self.in_asset_order(&point.weights)?,
            )?;
        }
        Ok(value)
    }

    /// `values` by asset as an object whose keys are in the order of the assets.
    fn in_asset_order(&self, values: &BTreeMap<String, f64>) -> Result<JsValue, JsValue> {
        let ordered = js_sys::Object::new();
        for asset in self.inner.labels() {
            if let Some(v) = values.get(asset) {
                js_sys::Reflect::set(&ordered, &asset.into(), &(*v).into())?;
            }
        }
        Ok(ordered.into())
    }
}

/// The options of a backtest, from the JSON of `BacktestOptions`.
fn backtest_options(
    options: serde_json::Map<String, serde_json::Value>,
) -> Result<finalytics::backtest::BacktestOptions, JsValue> {
    finalytics::formats::from_json(&serde_json::Value::Object(options).to_string(), "options")
        .map_err(error)
}

/// How `Assets.compare` and `Assets.backtest` run.
const SETTINGS: [&str; 7] = [
    "validation",
    "rebalance",
    "trading_cost_bps",
    "benchmark",
    "benchmark_name",
    "start_value",
    "flows",
];

/// The names of the fields of `T`, as JSON writes them, with `extra`.
fn fields<T: Serialize + Default>(extra: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = match serde_json::to_value(T::default()) {
        Ok(serde_json::Value::Object(map)) => map.into_iter().map(|(name, _)| name).collect(),
        _ => Vec::new(),
    };
    names.extend(extra.iter().map(|name| (*name).to_owned()));
    names
}

/// The options object `value` (empty if absent), with an error for each key not in `allowed`.
fn object(
    value: JsValue,
    allowed: &[String],
) -> Result<serde_json::Map<String, serde_json::Value>, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(serde_json::Map::new());
    }
    let serde_json::Value::Object(map) = from_js(value, "options")? else {
        return Err(invalid("options", "must be an object"));
    };
    let mut report = ValidationReport::new();
    for key in map.keys() {
        if !allowed.contains(key) {
            report.error(format!("options.{key}"), "is not an option");
        }
    }
    report.into_result(map).map_err(error)
}

/// The settings of a backtest, taken out of `choices`, in the form of `BacktestOptions`. A
/// choice may be given by name (`"quarterly"`), by name with its fields
/// (`{ walk_forward: { years: 5 } }`) or as tagged data (`{ type: "walk_forward", years: 5 }`);
/// a benchmark may be weights by asset or a price table.
fn settings(
    choices: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Map<String, serde_json::Value>, JsValue> {
    let mut settings = serde_json::Map::new();
    for key in ["validation", "rebalance"] {
        if let Some(value) = choices.remove(key) {
            settings.insert(key.to_owned(), tagged(value));
        }
    }
    for key in ["trading_cost_bps", "start_value", "flows"] {
        if let Some(value) = choices.remove(key) {
            settings.insert(key.to_owned(), value);
        }
    }
    if let Some(benchmark) = choices.remove("benchmark") {
        let tagged = match &benchmark {
            serde_json::Value::Object(map) if map.contains_key("type") => benchmark,
            serde_json::Value::Object(map) if map.contains_key("dates") => {
                serde_json::json!({ "type": "prices", "prices": benchmark })
            }
            serde_json::Value::Null => serde_json::Value::Null,
            _ => serde_json::json!({ "type": "weights", "weights": benchmark }),
        };
        settings.insert("benchmark".to_owned(), tagged);
    }
    if let Some(name) = choices.remove("benchmark_name") {
        match settings.get_mut("benchmark") {
            Some(serde_json::Value::Object(benchmark)) => {
                benchmark.insert("name".to_owned(), name);
            }
            _ => {
                return Err(invalid(
                    "options.benchmark_name",
                    "names a benchmark: give the benchmark too",
                ));
            }
        }
    }
    Ok(settings)
}

/// `"name"` or `{ name: { fields } }` as `{ type: "name", fields }`; anything else as it is.
fn tagged(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(name) => serde_json::json!({ "type": name }),
        serde_json::Value::Object(map) if map.len() == 1 && !map.contains_key("type") => {
            let mut out = serde_json::Map::new();
            for (name, fields) in map {
                match fields {
                    serde_json::Value::Object(fields) => out.extend(fields),
                    other => {
                        out.insert(name.clone(), other);
                    }
                }
                out.insert("type".to_owned(), serde_json::Value::String(name));
            }
            serde_json::Value::Object(out)
        }
        other => other,
    }
}

/// The limits and parameters left in `choices`, checked as `OptimiseOptions` so that a problem
/// is reported where it was given.
fn limits(
    choices: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Map<String, serde_json::Value>, JsValue> {
    let text = serde_json::Value::Object(choices.clone()).to_string();
    finalytics::formats::from_json::<finalytics::OptimiseOptions>(&text, "options")
        .map_err(error)?;
    Ok(choices)
}

/// Datasets for examples, tests and practice: daily prices of 55 S&P 500 companies and T-bills
/// with the S&P 500 index (`await datasets.sp500()`, `await datasets.sp500Index()`), a
/// two-account household holding five real funds, and synthetic asset-class indices. The market
/// data is real, from Yahoo Finance; the household's transactions are illustrative.
#[wasm_bindgen(js_name = datasets)]
pub struct Datasets;

#[wasm_bindgen(js_class = datasets)]
impl Datasets {
    /// The names of the dataset files, for `datasets.file`.
    pub fn files() -> Vec<String> {
        finalytics::datasets::FILES
            .iter()
            .map(|f| (*f).to_owned())
            .collect()
    }

    /// The text of a dataset file, such as `"portfolio/transactions.csv"`: an example of the
    /// layout each file takes.
    pub fn file(name: &str) -> Result<String, JsValue> {
        if matches!(name, "sp500/prices.csv" | "sp500/index.csv") {
            return Err(invalid(
                "name",
                format!(
                    "{name:?} is loaded on demand: use await datasets.sp500() or await datasets.sp500Index()"
                ),
            ));
        }
        finalytics::datasets::file(name)
            .map(str::to_owned)
            .ok_or_else(|| invalid("name", format!("{name:?} is not a dataset file")))
    }

    /// The companies of `datasets.sp500()`, in the order of its columns, with their names, GICS
    /// sectors and sub-industries.
    #[wasm_bindgen(js_name = sp500Companies, unchecked_return_type = "Company[]")]
    pub fn sp500_companies() -> Result<JsValue, JsValue> {
        to_js(&finalytics::datasets::sp500_companies())
    }

    /// Prices in CSV text as a `PriceTable`: how `datasets.sp500()` reads the package's file.
    #[wasm_bindgen(js_name = __priceTable, skip_typescript)]
    pub fn price_table(text: &str) -> Result<JsValue, JsValue> {
        to_js(&finalytics::datasets::price_table(text).map_err(error)?)
    }

    /// Month-end levels, from October 1996 to October 2026, of six asset-class indices that
    /// start at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate.
    #[wasm_bindgen(js_name = assetClassPrices, unchecked_return_type = "PriceTable")]
    pub fn asset_class_prices() -> Result<JsValue, JsValue> {
        to_js(&finalytics::datasets::asset_class_prices())
    }

    /// The US consumer price index for all urban consumers (CPI-U, all items, seasonally
    /// adjusted), monthly from January 1947 to September 2026, dated at each month's end, in one
    /// column, `US CPI`: the inflation of a plan, in place of a fixed rate.
    #[wasm_bindgen(js_name = usCpi, unchecked_return_type = "PriceTable")]
    pub fn us_cpi() -> Result<JsValue, JsValue> {
        to_js(&finalytics::datasets::us_cpi())
    }

    /// A portfolio of two accounts in US dollars and euros, from November 2023 to October 2026:
    /// five real funds at their real prices, with illustrative transactions and a balanced
    /// benchmark, `BENCH`.
    pub fn portfolio() -> Portfolio {
        finalytics::datasets::portfolio().into()
    }

    /// The options of the full report on `datasets.portfolio()`.
    #[wasm_bindgen(js_name = reportOptions, unchecked_return_type = "ReportOptions")]
    pub fn report_options() -> Result<JsValue, JsValue> {
        to_js(&finalytics::datasets::report_options())
    }
}

/// A portfolio's daily time-weighted returns and the measures built on them.
#[wasm_bindgen]
pub struct Performance {
    inner: finalytics::performance::Performance,
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
        let inner = finalytics::performance::Performance::from_parts(
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
    pub fn dates(&self) -> Vec<String> {
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

    /// Each holding's contribution to the time-weighted return; with `expenses`, they add up to
    /// the return.
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

    /// What fees and taxes charged to the accounts rather than to a holding took from the
    /// time-weighted return: negative, or zero without them.
    pub fn expenses(&self, start: Option<String>, end: Option<String>) -> Result<f64, JsValue> {
        let Some(holdings) = &self.holdings else {
            return Err(invalid(
                "performance",
                "expenses need performance measured from the books",
            ));
        };
        let (s, e) = self.range(start, end)?;
        self.inner.expenses(holdings, s, e).map_err(error)
    }
}

/// A period from its short label (`"MTD"`, `"QTD"`, `"YTD"`, `"ITD"`, `"3Y"`) or its object form.
fn period(value: JsValue) -> Result<Period, JsValue> {
    match value.as_string() {
        Some(label) => label.parse().map_err(error),
        None => from_js(value, "period"),
    }
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

/// A portfolio document holding `portfolio` and, optionally, its market data, as JSON text. The
/// market data is a `MarketData`, as `readDocument` returns it, or the document format
/// (`market.toDocument()`).
#[wasm_bindgen(js_name = writeDocument)]
pub fn write_document(
    #[wasm_bindgen(unchecked_param_type = "Portfolio")] portfolio: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "MarketData | DocumentMarketData")]
    market: JsValue,
) -> Result<String, JsValue> {
    let mut document = PortfolioDocument::new(from_js(portfolio, "portfolio")?);
    // A `MarketData` object turns itself into the document format.
    let to_document = js_sys::Reflect::get(&market, &"toDocument".into()).unwrap_or_default();
    let market = match to_document.dyn_ref::<js_sys::Function>() {
        Some(f) if !market.is_undefined() && !market.is_null() => f.call0(&market)?,
        _ => market,
    };
    let market: Option<MarketDocument> = optional(market, "market")?;
    document.market = market.map(|MarketDocument(m)| m);
    Ok(document.to_json())
}

/// Every structural problem in a portfolio, without applying its transactions.
#[wasm_bindgen(js_name = validatePortfolio, unchecked_return_type = "Issue[]")]
pub fn validate_portfolio(
    #[wasm_bindgen(unchecked_param_type = "Portfolio")] portfolio: JsValue,
) -> Result<JsValue, JsValue> {
    let portfolio: PortfolioData = from_js(portfolio, "portfolio")?;
    to_js(portfolio.validate().issues())
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

/// Splits the expected shortfall at `confidence` (0.95 by default) of a portfolio with
/// `weights` of the assets of `scenarios` into each asset's contribution: its weight times its
/// mean loss in the worst scenarios. The contributions add up to the expected shortfall.
#[wasm_bindgen(js_name = shortfallDecomposition, unchecked_return_type = "ShortfallDecomposition")]
pub fn shortfall_decomposition(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] weights: &[f64],
    #[wasm_bindgen(unchecked_param_type = "ReturnsTable")] scenarios: JsValue,
    confidence: Option<f64>,
) -> Result<JsValue, JsValue> {
    let table: ReturnsTable = from_js(scenarios, "scenarios")?;
    to_js(
        &risk::shortfall_decomposition(weights, &table, confidence.unwrap_or(0.95))
            .map_err(error)?,
    )
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

/// Estimates a factor model by regressing each asset's returns on factor returns, with an
/// intercept. Both tables cover the same periods; the model is per period.
#[wasm_bindgen(js_name = regressionModel, unchecked_return_type = "FactorModelEstimate")]
pub fn regression_model(
    #[wasm_bindgen(unchecked_param_type = "ReturnsTable")] returns: JsValue,
    #[wasm_bindgen(js_name = factorReturns, unchecked_param_type = "ReturnsTable")]
    factor_returns: JsValue,
) -> Result<JsValue, JsValue> {
    let returns: ReturnsTable = from_js(returns, "returns")?;
    let factors: ReturnsTable = from_js(factor_returns, "factor_returns")?;
    to_js(&risk::regression_model(&returns, &factors).map_err(error)?)
}

/// Estimates a statistical factor model from the first `factors` principal components of the
/// sample covariance matrix. The factors are named `pc1`, `pc2` and so on.
#[wasm_bindgen(js_name = statisticalModel, unchecked_return_type = "FactorModelEstimate")]
pub fn statistical_model(
    #[wasm_bindgen(unchecked_param_type = "ReturnsTable")] returns: JsValue,
    factors: usize,
) -> Result<JsValue, JsValue> {
    let returns: ReturnsTable = from_js(returns, "returns")?;
    to_js(&risk::statistical_model(&returns, factors).map_err(error)?)
}

/// Estimates a fundamental factor model for given exposures, one row per asset and one entry per
/// factor: each period's returns are regressed on the exposures to estimate the factor returns.
#[wasm_bindgen(js_name = fundamentalModel, unchecked_return_type = "FactorModelEstimate")]
pub fn fundamental_model(
    #[wasm_bindgen(unchecked_param_type = "ReturnsTable")] returns: JsValue,
    #[wasm_bindgen(unchecked_param_type = "string[]")] factors: JsValue,
    #[wasm_bindgen(unchecked_param_type = "number[][]")] exposures: JsValue,
) -> Result<JsValue, JsValue> {
    let returns: ReturnsTable = from_js(returns, "returns")?;
    let factors: Vec<String> = from_js(factors, "factors")?;
    let exposures: Vec<Vec<f64>> = from_js(exposures, "exposures")?;
    to_js(&risk::fundamental_model(&returns, &factors, &exposures).map_err(error)?)
}

/// Checks a factor model built from exposures and risk estimates supplied from elsewhere,
/// returning it unchanged.
#[wasm_bindgen(js_name = factorModel, unchecked_return_type = "FactorModel")]
pub fn factor_model(
    #[wasm_bindgen(unchecked_param_type = "FactorModel")] model: JsValue,
) -> Result<JsValue, JsValue> {
    let model: FactorModel = from_js(model, "model")?;
    to_js(&model)
}

fn report_options(value: JsValue) -> Result<finalytics::reports::ReportOptions, JsValue> {
    let options = object(value, &fields::<finalytics::reports::ReportOptions>(&[]))?;
    finalytics::formats::from_json(&serde_json::Value::Object(options).to_string(), "options")
        .map_err(error)
}

/// Any report, as returned by the engine, as labelled tables for export.
#[wasm_bindgen(js_name = reportTables, unchecked_return_type = "ReportTable[]")]
pub fn report_tables(
    #[wasm_bindgen(unchecked_param_type = "object")] report: JsValue,
) -> Result<JsValue, JsValue> {
    let text = js_sys::JSON::stringify(&report)
        .map_err(|_| invalid("report", "could not be written as JSON"))?
        .as_string()
        .unwrap_or_default();
    to_js(&finalytics::reports::report_tables(&text).map_err(error)?)
}

/// Any report, the six from `Portfolio.report`, or the report of `Assets.compare` or
/// `Assets.backtest`, as text for printing, 80 columns wide, with `name`, if given, under each
/// title.
#[wasm_bindgen(js_name = reportText)]
pub fn report_text(
    #[wasm_bindgen(unchecked_param_type = "object")] report: JsValue,
    name: Option<String>,
) -> Result<String, JsValue> {
    let text = js_sys::JSON::stringify(&report)
        .map_err(|_| invalid("report", "could not be written as JSON"))?
        .as_string()
        .unwrap_or_default();
    finalytics::reports::report_text(&text, name.as_deref()).map_err(error)
}

/// The six reports from `Portfolio.report`, or the report of `Assets.compare` or
/// `Assets.backtest`, as one self-contained HTML page, with `name`, if given, as its title:
/// inline styles and SVG charts, no scripts, in light and dark themes, ready to print.
#[wasm_bindgen(js_name = reportHtml)]
pub fn report_html(
    #[wasm_bindgen(
        unchecked_param_type = "PortfolioReport | OptimisationReport | ComparisonReport | BacktestReport | RebalanceReport | PlanReport"
    )]
    report: JsValue,
    name: Option<String>,
    #[wasm_bindgen(unchecked_param_type = "Theme")] theme: Option<String>,
) -> Result<String, JsValue> {
    let text = js_sys::JSON::stringify(&report)
        .map_err(|_| invalid("report", "could not be written as JSON"))?
        .as_string()
        .unwrap_or_default();
    let theme: finalytics::Theme = match theme.as_deref() {
        None => finalytics::Theme::Auto,
        Some(name) => serde_json::from_value(serde_json::Value::String(name.to_owned()))
            .map_err(|_| invalid("theme", "must be \"auto\", \"light\" or \"dark\""))?,
    };
    finalytics::reports::report_html(&text, name.as_deref())
        .map(|page| theme.apply(&page))
        .map_err(error)
}

/// The raw results of `Assets.optimise`, `Assets.compare`, `Assets.backtest` or `Assets.plan` as
/// text: the weights, each strategy's periods, growth, trades and refits, or a plan's chance.
#[wasm_bindgen(js_name = resultsText)]
pub fn results_text(
    #[wasm_bindgen(unchecked_param_type = "OptimisationResult | BacktestResults | PlanResults")]
    results: JsValue,
) -> Result<String, JsValue> {
    let text = js_sys::JSON::stringify(&results)
        .map_err(|_| invalid("results", "could not be written as JSON"))?
        .as_string()
        .unwrap_or_default();
    finalytics::results_text(&text).map_err(error)
}

/// `covariance` with every volatility scaled and every correlation moved toward one by
/// `stress`: `s^2 ((1 - a) C + a v v')`, with `v` the volatilities.
#[wasm_bindgen(js_name = stressedCovariance, unchecked_return_type = "Matrix")]
pub fn stressed_covariance(
    #[wasm_bindgen(unchecked_param_type = "Matrix")] covariance: JsValue,
    #[wasm_bindgen(unchecked_param_type = "CorrelationStress")] stress: JsValue,
) -> Result<JsValue, JsValue> {
    let covariance: Matrix = from_js(covariance, "covariance")?;
    let stress: finalytics::risk::CorrelationStress = from_js(stress, "stress")?;
    to_js(&finalytics::risk::stressed_covariance(&covariance, &stress).map_err(error)?)
}

/// The covariance matrix of the assets that a factor model implies, `B F B' + diag(d)`.
#[wasm_bindgen(js_name = impliedCovariance, unchecked_return_type = "Matrix")]
pub fn implied_covariance(
    #[wasm_bindgen(unchecked_param_type = "FactorModel")] model: JsValue,
) -> Result<JsValue, JsValue> {
    let model: FactorModel = from_js(model, "model")?;
    to_js(&model.covariance())
}

/// Splits a portfolio's volatility under a factor model into each factor's contribution and the
/// specific part, and into each asset's contribution.
#[wasm_bindgen(js_name = factorRisk, unchecked_return_type = "FactorRisk")]
pub fn factor_risk(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] weights: &[f64],
    #[wasm_bindgen(unchecked_param_type = "FactorModel")] model: JsValue,
) -> Result<JsValue, JsValue> {
    let model: FactorModel = from_js(model, "model")?;
    to_js(&risk::factor_risk(weights, &model).map_err(error)?)
}

/// The ex-ante tracking error against benchmark weights under a factor model, split by factor
/// and by asset.
#[wasm_bindgen(js_name = factorTrackingError, unchecked_return_type = "FactorRisk")]
pub fn factor_tracking_error(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] weights: &[f64],
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] benchmark: &[f64],
    #[wasm_bindgen(unchecked_param_type = "FactorModel")] model: JsValue,
) -> Result<JsValue, JsValue> {
    let model: FactorModel = from_js(model, "model")?;
    to_js(&risk::factor_tracking_error(weights, benchmark, &model).map_err(error)?)
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

/// Brinson–Fachler attribution by segment over one or more periods, linked by `linking`
/// (`"carino"` by default, or `"menchero"`).
#[wasm_bindgen(js_name = brinsonFachler, unchecked_return_type = "Attribution")]
pub fn brinson_fachler(
    #[wasm_bindgen(unchecked_param_type = "AttributionPeriod[]")] periods: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "Linking")] linking: JsValue,
) -> Result<JsValue, JsValue> {
    let periods: Vec<finalytics::performance::AttributionPeriod> = from_js(periods, "periods")?;
    let linking: finalytics::performance::Linking = optional(linking, "linking")?;
    to_js(&finalytics::performance::brinson_fachler(&periods, linking).map_err(error)?)
}

/// Currency attribution by the method of Karnosky and Singer over one or more periods.
#[wasm_bindgen(js_name = karnoskySinger, unchecked_return_type = "CurrencyAttribution")]
pub fn karnosky_singer(
    #[wasm_bindgen(unchecked_param_type = "CurrencyAttributionPeriod[]")] periods: JsValue,
    #[wasm_bindgen(unchecked_optional_param_type = "Linking")] linking: JsValue,
) -> Result<JsValue, JsValue> {
    let periods: Vec<finalytics::performance::CurrencyAttributionPeriod> =
        from_js(periods, "periods")?;
    let linking: finalytics::performance::Linking = optional(linking, "linking")?;
    to_js(&finalytics::performance::karnosky_singer(&periods, linking).map_err(error)?)
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

/// Resampled optimisation: the average of the optimal portfolios for estimates re-made from
/// simulated histories. The same seed gives the same result on every platform.
#[wasm_bindgen(unchecked_return_type = "ResampledSolution")]
pub fn resampled(
    #[wasm_bindgen(unchecked_param_type = "Problem")] problem: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Objective")] objective: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Resampling")] resampling: JsValue,
) -> Result<JsValue, JsValue> {
    let problem: Problem = from_js(problem, "problem")?;
    let objective: Objective = from_js(objective, "objective")?;
    let resampling: Resampling = from_js(resampling, "resampling")?;
    to_js(&problem.resampled(&objective, &resampling).map_err(error)?)
}

/// The Black–Litterman posterior expected returns and covariance.
#[wasm_bindgen(js_name = blackLitterman, unchecked_return_type = "BlackLittermanEstimate")]
pub fn black_litterman(
    #[wasm_bindgen(unchecked_param_type = "BlackLitterman")] model: JsValue,
) -> Result<JsValue, JsValue> {
    let model: BlackLitterman = from_js(model, "model")?;
    to_js(&model.estimate().map_err(error)?)
}

/// The market's risk aversion implied by its returns: the annualised excess return over the
/// annualised variance. Options default to 252 periods a year and a zero risk-free rate.
#[wasm_bindgen(js_name = impliedRiskAversion)]
pub fn implied_risk_aversion(
    #[wasm_bindgen(unchecked_param_type = "Float64Array | number[]")] returns: &[f64],
    #[wasm_bindgen(
        unchecked_optional_param_type = "{ periods_per_year?: number; risk_free_rate?: number }"
    )]
    options: JsValue,
) -> Result<f64, JsValue> {
    // The same options, and defaults, as the statistics take.
    let options = statistics_options(options)?;
    finalytics::opt::implied_risk_aversion(
        returns,
        options.periods_per_year,
        options.risk_free_rate,
    )
    .map_err(error)
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
