//! The native extension behind the `finalytics` Python package.
//!
//! This layer only converts data between Python and Rust; the analytics live in the `finalytics`
//! crate. Structured values cross as JSON text in the formats of `schema/finalytics.schema.json`;
//! series cross as little-endian `float64` (values) and `int32` (days since 1970-01-01) buffers,
//! which the package turns into NumPy arrays without copying element by element. The Python
//! package wraps these functions in its public, typed API.

use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::Serialize;
use serde::de::DeserializeOwned;

use finalytics::formats::{
    CsvOptions, PortfolioDocument, fx_from_csv, portfolio_from_csv, prices_from_csv,
};
use finalytics::opt::{Objective, Problem, Rebalance, RebalanceOptions};
use finalytics::risk::{
    self, CovarianceMethod, Matrix, ReturnsTable, StatisticsOptions, TailMethod,
};
use finalytics::{
    Books, Currency, Date, Decimal, Error, FlowTiming, HoldingKey, HoldingSeries, InstrumentId,
    LotMethod, MarketData, Observation, Performance, Period, Portfolio, TimeSeries,
    ValidationReport,
};

create_exception!(
    _finalytics,
    EngineError,
    PyValueError,
    "An error from the engine: the message, and the issues of a validation error as JSON."
);

fn engine(error: Error) -> PyErr {
    let issues = match &error {
        Error::Validation(report) => serde_json::to_string(report.issues()).ok(),
        _ => None,
    };
    EngineError::new_err((error.to_string(), issues))
}

fn invalid(path: &str, message: impl Into<String>) -> PyErr {
    let mut report = ValidationReport::new();
    report.error(path, message);
    engine(Error::Validation(report))
}

fn to_json<T: Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| invalid("", format!("could not encode a result: {e}")))
}

/// Parses JSON input, reporting where it is malformed with paths under `name`.
fn from_json<T: DeserializeOwned>(text: &str, name: &str) -> PyResult<T> {
    finalytics::formats::from_json(text, name).map_err(engine)
}

/// Parses a name such as `"start_of_day"` or `"fifo"` as the enum value it names.
fn named<T: DeserializeOwned>(text: &str, name: &str) -> PyResult<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|e| invalid(name, e.to_string()))
}

fn date(text: &str) -> PyResult<Date> {
    text.parse()
        .map_err(|_| invalid("date", format!("{text:?} is not a date in YYYY-MM-DD form")))
}

fn optional_date(text: Option<&str>) -> PyResult<Option<Date>> {
    text.map(date).transpose()
}

fn floats(bytes: &[u8], name: &str) -> PyResult<Vec<f64>> {
    if !bytes.len().is_multiple_of(8) {
        return Err(invalid(name, "must be a buffer of float64 values"));
    }
    Ok(bytes
        .chunks_exact(8)
        .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect())
}

fn dates(bytes: &[u8], name: &str) -> PyResult<Vec<Date>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(invalid(name, "must be a buffer of int32 day numbers"));
    }
    bytes
        .chunks_exact(4)
        .map(|c| {
            Date::from_days_since_epoch(i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .map_err(engine)
        })
        .collect()
}

fn float_bytes<'py>(py: Python<'py>, values: &[f64]) -> Bound<'py, PyBytes> {
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    PyBytes::new(py, &bytes)
}

fn date_bytes<'py>(py: Python<'py>, values: &[Date]) -> Bound<'py, PyBytes> {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|d| d.days_since_epoch().to_le_bytes())
        .collect();
    PyBytes::new(py, &bytes)
}

fn decimals(text: &str, name: &str) -> PyResult<Vec<Decimal>> {
    from_json(text, name)
}

fn currency(text: &str) -> PyResult<Currency> {
    text.parse().map_err(engine)
}

fn instrument(text: &str) -> PyResult<InstrumentId> {
    InstrumentId::new(text)
        .ok_or_else(|| invalid("instrument", format!("{text:?} is not a valid identifier")))
}

fn csv_options(delimiter: &str) -> PyResult<CsvOptions> {
    match delimiter.as_bytes() {
        [byte] => Ok(CsvOptions { delimiter: *byte }),
        _ => Err(invalid("delimiter", "must be a single character")),
    }
}

/// An observation as JSON: `{"value", "date", "carried"}`, or `None` if missing.
fn observation(found: Observation<Decimal>, date: Date) -> PyResult<Option<String>> {
    let (value, from, carried) = match found {
        Observation::Observed(v) => (v, date, false),
        Observation::Carried { value, from } => (value, from, true),
        Observation::Missing => return Ok(None),
    };
    to_json(&serde_json::json!({"value": value, "date": from, "carried": carried})).map(Some)
}

/// Market data: prices and FX rates. Each change returns a new object.
#[pyclass(module = "finalytics._finalytics", frozen)]
#[derive(Clone)]
struct Market {
    inner: MarketData,
}

#[pymethods]
impl Market {
    #[new]
    fn new() -> Market {
        Market {
            inner: MarketData::new(),
        }
    }

    fn with_prices(&self, id: &str, dates_buffer: &[u8], values: &str) -> PyResult<Market> {
        let series = TimeSeries::new(dates(dates_buffer, "dates")?, decimals(values, "values")?)
            .map_err(engine)?;
        Ok(Market {
            inner: self.inner.clone().with_prices(instrument(id)?, series),
        })
    }

    fn with_fx(
        &self,
        base: &str,
        quote: &str,
        dates_buffer: &[u8],
        values: &str,
    ) -> PyResult<Market> {
        let series = TimeSeries::new(dates(dates_buffer, "dates")?, decimals(values, "rates")?)
            .map_err(engine)?;
        Ok(Market {
            inner: self
                .inner
                .clone()
                .with_fx(currency(base)?, currency(quote)?, series),
        })
    }

    fn with_pivot(&self, pivot: &str) -> PyResult<Market> {
        Ok(Market {
            inner: self.inner.clone().with_pivot(currency(pivot)?),
        })
    }

    fn with_max_carry_days(&self, days: u32) -> Market {
        Market {
            inner: self.inner.clone().with_max_carry_days(days),
        }
    }

    fn pivot(&self) -> String {
        self.inner.pivot().to_string()
    }

    fn max_carry_days(&self) -> u32 {
        self.inner.max_carry_days()
    }

    fn price(&self, id: &str, on: &str) -> PyResult<Option<String>> {
        let on = date(on)?;
        observation(self.inner.price(&instrument(id)?, on), on)
    }

    fn fx_rate(&self, base: &str, quote: &str, on: &str) -> PyResult<Option<String>> {
        let on = date(on)?;
        observation(
            self.inner.fx_rate(currency(base)?, currency(quote)?, on),
            on,
        )
    }

    /// The prices by instrument, as JSON `{id: {dates, values}}`.
    fn prices(&self) -> PyResult<String> {
        let all: std::collections::BTreeMap<_, _> = self.inner.all_prices().collect();
        to_json(&all)
    }

    /// The FX rates, as JSON `[{base, quote, rates}]`.
    fn fx(&self) -> PyResult<String> {
        let all: Vec<_> = self
            .inner
            .all_fx()
            .map(|((base, quote), rates)| serde_json::json!({"base": base, "quote": quote, "rates": rates}))
            .collect();
        to_json(&all)
    }

    /// Every date with a price, in order.
    fn price_dates<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let mut all: Vec<Date> = self
            .inner
            .all_prices()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .collect();
        all.sort();
        all.dedup();
        date_bytes(py, &all)
    }
}

/// The books of a portfolio, with the market data used to value them.
#[pyclass(module = "finalytics._finalytics", frozen)]
struct NativeBooks {
    books: Books,
    market: MarketData,
}

#[pymethods]
impl NativeBooks {
    #[new]
    fn new(portfolio: &str, market: &Market) -> PyResult<NativeBooks> {
        let portfolio: Portfolio = from_json(portfolio, "portfolio")?;
        let books = Books::build(&portfolio, &market.inner).map_err(engine)?;
        Ok(NativeBooks {
            books,
            market: market.inner.clone(),
        })
    }

    fn base_currency(&self) -> String {
        self.books.base_currency().to_string()
    }

    /// Positions, lots and cash at the end of `on` (the latest transaction when `None`).
    fn holdings(&self, on: Option<&str>) -> PyResult<String> {
        let holdings = match optional_date(on)? {
            Some(d) => self.books.holdings_on(d),
            None => self.books.holdings().clone(),
        };
        let lots: Vec<_> = holdings.lots().collect();
        to_json(&serde_json::json!({
            "positions": holdings.positions(),
            "lots": lots,
            "cash": holdings.cash_balances(),
        }))
    }

    fn valuation(&self, on: &str) -> PyResult<String> {
        to_json(
            &self
                .books
                .value_on(date(on)?, &self.market)
                .map_err(engine)?,
        )
    }

    fn value_series(&self, dates_buffer: &[u8]) -> PyResult<String> {
        let series = self
            .books
            .value_series(&dates(dates_buffer, "dates")?, &self.market)
            .map_err(engine)?;
        to_json(&series)
    }

    fn holding_series(&self, dates_buffer: &[u8]) -> PyResult<String> {
        let series = self
            .books
            .holding_series(&dates(dates_buffer, "dates")?, &self.market)
            .map_err(engine)?;
        to_json(&series)
    }

    fn realised_gains(&self) -> PyResult<String> {
        to_json(&self.books.realised_gains())
    }

    fn events(&self) -> PyResult<String> {
        to_json(&self.books.events())
    }

    /// Performance on `dates`, with the holding-level series for contribution.
    fn performance(&self, dates_buffer: &[u8], timing: &str) -> PyResult<NativePerformance> {
        let dates = dates(dates_buffer, "dates")?;
        let timing: FlowTiming = named(timing, "timing")?;
        let values = self
            .books
            .value_series(&dates, &self.market)
            .map_err(engine)?;
        let holdings = self
            .books
            .holding_series(&dates, &self.market)
            .map_err(engine)?;
        Ok(NativePerformance {
            inner: Performance::from_values(&values, timing).map_err(engine)?,
            holdings: Some(holdings),
            warnings: values.warnings,
        })
    }

    fn holding_weights(&self, on: &str) -> PyResult<String> {
        let valuation = self
            .books
            .value_on(date(on)?, &self.market)
            .map_err(engine)?;
        to_json(&risk::holding_weights(&valuation).map_err(engine)?)
    }

    fn holding_returns(&self, holdings: &str, dates_buffer: &[u8]) -> PyResult<String> {
        let holdings: Vec<HoldingKey> = from_json(holdings, "holdings")?;
        let found = risk::holding_returns(
            &self.books,
            &holdings,
            &dates(dates_buffer, "dates")?,
            &self.market,
        )
        .map_err(engine)?;
        to_json(&found)
    }

    fn lots_for_sale(
        &self,
        on: &str,
        account: &str,
        id: &str,
        quantity: &str,
        method: &str,
    ) -> PyResult<String> {
        let account = finalytics::AccountId::new(account)
            .ok_or_else(|| invalid("account", format!("{account:?} is not a valid identifier")))?;
        let quantity: Decimal = named(quantity, "quantity")?;
        let method: LotMethod = named(method, "method")?;
        let sale = self
            .books
            .holdings_on(date(on)?)
            .lots_for_sale(&account, &instrument(id)?, quantity, method)
            .map_err(engine)?;
        to_json(&sale)
    }

    fn rebalance(&self, on: &str, account: &str, targets: &str, options: &str) -> PyResult<String> {
        let account = finalytics::AccountId::new(account)
            .ok_or_else(|| invalid("account", format!("{account:?} is not a valid identifier")))?;
        let targets: Vec<(HoldingKey, f64)> = from_json(targets, "targets")?;
        let options: RebalanceOptions = from_json(options, "options")?;
        let plan = finalytics::opt::rebalance(
            &self.books,
            &self.market,
            date(on)?,
            &account,
            &targets,
            &options,
        )
        .map_err(engine)?;
        to_json(&plan)
    }
}

/// Time-weighted performance, and the holding series for contribution when built from books.
#[pyclass(module = "finalytics._finalytics", frozen)]
struct NativePerformance {
    inner: Performance,
    holdings: Option<HoldingSeries>,
    warnings: ValidationReport,
}

#[pymethods]
impl NativePerformance {
    #[staticmethod]
    fn from_values(
        dates_buffer: &[u8],
        values: &[u8],
        flows: &[u8],
        timing: &str,
    ) -> PyResult<NativePerformance> {
        let timing: FlowTiming = named(timing, "timing")?;
        let inner = Performance::from_parts(
            dates(dates_buffer, "dates")?,
            floats(values, "values")?,
            floats(flows, "flows")?,
            timing,
        )
        .map_err(engine)?;
        Ok(NativePerformance {
            inner,
            holdings: None,
            warnings: ValidationReport::new(),
        })
    }

    fn dates<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        date_bytes(py, self.inner.dates())
    }

    fn returns<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        float_bytes(py, self.inner.returns().values())
    }

    fn growth<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        float_bytes(py, self.inner.growth_of(1.0).values())
    }

    fn values<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        float_bytes(py, self.inner.values().values())
    }

    fn warnings(&self) -> PyResult<String> {
        to_json(&self.warnings)
    }

    fn time_weighted(&self, start: &str, end: &str) -> PyResult<Option<f64>> {
        Ok(self.inner.time_weighted(date(start)?, date(end)?))
    }

    fn period(&self, period: &str, as_of: Option<&str>) -> PyResult<Option<String>> {
        let period: Period = from_json(period, "period")?;
        self.inner
            .period(period, optional_date(as_of)?)
            .map(|p| to_json(&p))
            .transpose()
    }

    fn money_weighted(&self, start: &str, end: &str) -> PyResult<Option<f64>> {
        Ok(self.inner.money_weighted(date(start)?, date(end)?))
    }

    fn modified_dietz(&self, start: &str, end: &str) -> PyResult<Option<f64>> {
        Ok(self.inner.modified_dietz(date(start)?, date(end)?))
    }

    fn excess_return(
        &self,
        period: &str,
        as_of: Option<&str>,
        benchmark_dates: &[u8],
        benchmark_levels: &[u8],
        max_carry_days: u32,
    ) -> PyResult<Option<String>> {
        let period: Period = from_json(period, "period")?;
        let benchmark = TimeSeries::new(
            dates(benchmark_dates, "benchmark.dates")?,
            floats(benchmark_levels, "benchmark.levels")?,
        )
        .map_err(engine)?;
        self.inner
            .excess_return(period, optional_date(as_of)?, &benchmark, max_carry_days)
            .map(|e| to_json(&e))
            .transpose()
    }

    fn contributions(&self, start: &str, end: &str) -> PyResult<String> {
        let Some(holdings) = &self.holdings else {
            return Err(invalid(
                "performance",
                "contributions need performance measured from the books",
            ));
        };
        to_json(
            &self
                .inner
                .contributions(holdings, date(start)?, date(end)?)
                .map_err(engine)?,
        )
    }
}

#[pyfunction]
fn version() -> &'static str {
    finalytics::version()
}

#[pyfunction]
fn read_document(text: &str) -> PyResult<(String, Option<Market>)> {
    let document = PortfolioDocument::from_json(text).map_err(engine)?;
    Ok((
        to_json(&document.portfolio)?,
        document.market.map(|inner| Market { inner }),
    ))
}

#[pyfunction]
#[pyo3(signature = (portfolio, market=None))]
fn write_document(portfolio: &str, market: Option<&Market>) -> PyResult<String> {
    let mut document = PortfolioDocument::new(from_json(portfolio, "portfolio")?);
    document.market = market.map(|m| m.inner.clone());
    Ok(document.to_json())
}

#[pyfunction]
fn validate_portfolio(portfolio: &str) -> PyResult<String> {
    let portfolio: Portfolio = from_json(portfolio, "portfolio")?;
    to_json(&portfolio.validate())
}

#[pyfunction]
fn read_portfolio_csv(
    base_currency: &str,
    instruments: &str,
    transactions: &str,
    delimiter: &str,
) -> PyResult<String> {
    let portfolio = portfolio_from_csv(
        currency(base_currency)?,
        instruments,
        transactions,
        csv_options(delimiter)?,
    )
    .map_err(engine)?;
    to_json(&portfolio)
}

#[pyfunction]
fn read_prices_csv(text: &str, delimiter: &str) -> PyResult<String> {
    to_json(&prices_from_csv(text, csv_options(delimiter)?).map_err(engine)?)
}

#[pyfunction]
fn read_fx_csv(text: &str, delimiter: &str) -> PyResult<String> {
    let series = fx_from_csv(text, csv_options(delimiter)?).map_err(engine)?;
    let all: Vec<_> = series
        .iter()
        .map(|s| serde_json::json!({"base": s.base, "quote": s.quote, "rates": s.rates}))
        .collect();
    to_json(&all)
}

#[pyfunction]
fn statistics(returns: &[u8], options: &str) -> PyResult<String> {
    let options: StatisticsOptions = from_json(options, "options")?;
    to_json(&risk::statistics(&floats(returns, "returns")?, &options).map_err(engine)?)
}

#[pyfunction]
fn relative_statistics(returns: &[u8], benchmark: &[u8], options: &str) -> PyResult<String> {
    let options: StatisticsOptions = from_json(options, "options")?;
    let found = risk::relative_statistics(
        &floats(returns, "returns")?,
        &floats(benchmark, "benchmark")?,
        &options,
    )
    .map_err(engine)?;
    to_json(&found)
}

#[pyfunction]
fn tail_risk(returns: &[u8], confidence: f64, method: &str) -> PyResult<String> {
    let method: TailMethod = named(method, "method")?;
    to_json(&risk::tail_risk(&floats(returns, "returns")?, confidence, method).map_err(engine)?)
}

#[pyfunction]
fn rolling_statistics(returns: &[u8], window: usize, options: &str) -> PyResult<String> {
    let options: StatisticsOptions = from_json(options, "options")?;
    to_json(
        &risk::rolling_statistics(&floats(returns, "returns")?, window, &options)
            .map_err(engine)?,
    )
}

#[pyfunction]
fn rolling_relative_statistics(
    returns: &[u8],
    benchmark: &[u8],
    window: usize,
    options: &str,
) -> PyResult<String> {
    let options: StatisticsOptions = from_json(options, "options")?;
    let found = risk::rolling_relative_statistics(
        &floats(returns, "returns")?,
        &floats(benchmark, "benchmark")?,
        window,
        &options,
    )
    .map_err(engine)?;
    to_json(&found)
}

#[pyfunction]
fn rolling_tail_risk(
    returns: &[u8],
    window: usize,
    confidence: f64,
    method: &str,
) -> PyResult<String> {
    let method: TailMethod = named(method, "method")?;
    let found = risk::rolling_tail_risk(&floats(returns, "returns")?, window, confidence, method)
        .map_err(engine)?;
    to_json(&found)
}

fn levels(dates_buffer: &[u8], values: &[u8]) -> PyResult<TimeSeries<f64>> {
    TimeSeries::new(dates(dates_buffer, "dates")?, floats(values, "levels")?).map_err(engine)
}

#[pyfunction]
fn drawdowns(dates_buffer: &[u8], values: &[u8]) -> PyResult<String> {
    to_json(&risk::drawdowns(&levels(dates_buffer, values)?).map_err(engine)?)
}

#[pyfunction]
fn drawdown_series<'py>(
    py: Python<'py>,
    dates_buffer: &[u8],
    values: &[u8],
) -> PyResult<Bound<'py, PyBytes>> {
    let series = risk::drawdown_series(&levels(dates_buffer, values)?).map_err(engine)?;
    Ok(float_bytes(py, series.values()))
}

/// `columns` holds the assets' returns one asset after another (each `observations` long).
#[pyfunction]
fn covariance(columns: &[u8], labels: &str, method: &str) -> PyResult<String> {
    let labels: Vec<String> = from_json(labels, "labels")?;
    let values = floats(columns, "returns")?;
    let n = labels.len().max(1);
    if !values.len().is_multiple_of(n) {
        return Err(invalid(
            "returns",
            "must have the same number of returns for every asset",
        ));
    }
    let t = values.len() / n;
    let table = ReturnsTable::new(
        labels,
        values.chunks(t.max(1)).map(<[f64]>::to_vec).collect(),
    )
    .map_err(engine)?;
    let method: CovarianceMethod = from_json(method, "method")?;
    to_json(&risk::covariance(&table, method).map_err(engine)?)
}

#[pyfunction]
fn correlation(matrix: &str) -> PyResult<String> {
    let matrix: Matrix = from_json(matrix, "matrix")?;
    to_json(&matrix.correlation().map_err(engine)?)
}

#[pyfunction]
fn risk_decomposition(weights: &[u8], matrix: &str) -> PyResult<String> {
    let matrix: Matrix = from_json(matrix, "covariance")?;
    to_json(&risk::risk_decomposition(&floats(weights, "weights")?, &matrix).map_err(engine)?)
}

#[pyfunction]
fn tracking_error_decomposition(
    weights: &[u8],
    benchmark: &[u8],
    matrix: &str,
) -> PyResult<String> {
    let matrix: Matrix = from_json(matrix, "covariance")?;
    let found = risk::tracking_error_decomposition(
        &floats(weights, "weights")?,
        &floats(benchmark, "benchmark")?,
        &matrix,
    )
    .map_err(engine)?;
    to_json(&found)
}

#[pyfunction]
fn normal_cdf(x: f64) -> f64 {
    risk::normal_cdf(x)
}

#[pyfunction]
fn normal_pdf(x: f64) -> f64 {
    risk::normal_pdf(x)
}

#[pyfunction]
fn normal_quantile(p: f64) -> f64 {
    risk::normal_quantile(p)
}

#[pyfunction]
fn solve(problem: &str, objective: &str) -> PyResult<String> {
    let problem: Problem = from_json(problem, "problem")?;
    let objective: Objective = from_json(objective, "objective")?;
    to_json(&problem.solve(&objective).map_err(engine)?)
}

#[pyfunction]
fn frontier(problem: &str, points: usize) -> PyResult<String> {
    let problem: Problem = from_json(problem, "problem")?;
    to_json(&problem.frontier(points).map_err(engine)?)
}

#[pyfunction]
fn rebalance_transactions(rebalance: &str, prefix: &str) -> PyResult<String> {
    let plan: Rebalance = from_json(rebalance, "rebalance")?;
    to_json(&plan.transactions(prefix).map_err(engine)?)
}

/// The `finalytics._finalytics` extension module.
#[pymodule]
fn _finalytics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("EngineError", m.py().get_type::<EngineError>())?;
    m.add_class::<Market>()?;
    m.add_class::<NativeBooks>()?;
    m.add_class::<NativePerformance>()?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(read_document, m)?)?;
    m.add_function(wrap_pyfunction!(write_document, m)?)?;
    m.add_function(wrap_pyfunction!(validate_portfolio, m)?)?;
    m.add_function(wrap_pyfunction!(read_portfolio_csv, m)?)?;
    m.add_function(wrap_pyfunction!(read_prices_csv, m)?)?;
    m.add_function(wrap_pyfunction!(read_fx_csv, m)?)?;
    m.add_function(wrap_pyfunction!(statistics, m)?)?;
    m.add_function(wrap_pyfunction!(relative_statistics, m)?)?;
    m.add_function(wrap_pyfunction!(tail_risk, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_statistics, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_relative_statistics, m)?)?;
    m.add_function(wrap_pyfunction!(rolling_tail_risk, m)?)?;
    m.add_function(wrap_pyfunction!(drawdowns, m)?)?;
    m.add_function(wrap_pyfunction!(drawdown_series, m)?)?;
    m.add_function(wrap_pyfunction!(covariance, m)?)?;
    m.add_function(wrap_pyfunction!(correlation, m)?)?;
    m.add_function(wrap_pyfunction!(risk_decomposition, m)?)?;
    m.add_function(wrap_pyfunction!(tracking_error_decomposition, m)?)?;
    m.add_function(wrap_pyfunction!(normal_cdf, m)?)?;
    m.add_function(wrap_pyfunction!(normal_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(normal_quantile, m)?)?;
    m.add_function(wrap_pyfunction!(solve, m)?)?;
    m.add_function(wrap_pyfunction!(frontier, m)?)?;
    m.add_function(wrap_pyfunction!(rebalance_transactions, m)?)?;
    Ok(())
}
