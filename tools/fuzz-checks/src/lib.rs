//! The checks behind the fuzz targets in `fuzz/`.
//!
//! Each check feeds arbitrary input to a part of the engine that reads untrusted data and asserts
//! what must hold for any input: no panic, and, where the input is accepted, that the result is
//! consistent (a document written back reads as the same document). The fuzz targets call these
//! functions; `tests/corpus.rs` replays the committed corpus through them on the stable
//! toolchain, so that every input found once stays checked.

use finalytics::formats::{
    CsvOptions, MarketDocument, PortfolioDocument, fx_from_csv, instruments_from_csv,
    portfolio_from_csv, prices_from_csv, transactions_from_csv,
};
use finalytics::opt::{Constraints, Objective, Problem};
use finalytics::risk::{self, Matrix, ReturnsTable, StatisticsOptions, TailMethod};
use finalytics::{Books, Currency, Date, FlowTiming, MarketData, Performance, Portfolio};

/// A check: it takes any bytes and panics if what must hold does not.
pub type Check = fn(&[u8]);

/// The checks, by name, as the fuzz targets and the corpus directories call them.
pub const CHECKS: &[(&str, Check)] = &[
    ("csv_instruments", csv_instruments),
    ("csv_transactions", csv_transactions),
    ("csv_prices", csv_prices),
    ("csv_fx", csv_fx),
    ("csv_portfolio", csv_portfolio),
    ("document", document),
    ("market_document", market_document),
    ("returns", returns),
    ("optimisation", optimisation),
];

fn text(data: &[u8]) -> Option<&str> {
    std::str::from_utf8(data).ok()
}

/// The CSV delimiter: a semicolon when the input starts with one, otherwise a comma.
fn options(text: &str) -> (CsvOptions, &str) {
    match text.strip_prefix(';') {
        Some(rest) => (CsvOptions { delimiter: b';' }, rest),
        None => (CsvOptions::default(), text),
    }
}

/// Instruments CSV.
pub fn csv_instruments(data: &[u8]) {
    if let Some(text) = text(data) {
        let (options, text) = options(text);
        let _ = instruments_from_csv(text, options);
    }
}

/// Transactions CSV.
pub fn csv_transactions(data: &[u8]) {
    if let Some(text) = text(data) {
        let (options, text) = options(text);
        let _ = transactions_from_csv(text, options);
    }
}

/// Prices CSV, in long or wide layout.
pub fn csv_prices(data: &[u8]) {
    if let Some(text) = text(data) {
        let (options, text) = options(text);
        let _ = prices_from_csv(text, options);
    }
}

/// FX rates CSV, in long or wide layout.
pub fn csv_fx(data: &[u8]) {
    if let Some(text) = text(data) {
        let (options, text) = options(text);
        let _ = fx_from_csv(text, options);
    }
}

/// Instruments and transactions separated by a NUL byte: read them, keep the books, and value
/// them on the last transaction date.
pub fn csv_portfolio(data: &[u8]) {
    let Some(text) = text(data) else { return };
    let (instruments, transactions) = text.split_once('\0').unwrap_or((text, ""));
    if let Ok(portfolio) = portfolio_from_csv(
        Currency::USD,
        instruments,
        transactions,
        CsvOptions::default(),
    ) {
        books(&portfolio, &MarketData::new());
    }
}

/// A portfolio document: if it reads, it must write back as JSON that reads as the same document,
/// and its books, valuations and performance must not panic.
pub fn document(data: &[u8]) {
    let Some(text) = text(data) else { return };
    let Ok(document) = PortfolioDocument::from_json(text) else {
        return;
    };
    let written = document.to_json();
    let again = PortfolioDocument::from_json(&written)
        .unwrap_or_else(|e| panic!("a document written back does not read: {e}\n{written}"));
    assert_eq!(again, document, "a document written back reads differently");
    books(&document.portfolio, &document.market.unwrap_or_default());
}

/// Market data in the document format: if they read, they must write back as JSON that reads
/// as the same market data.
pub fn market_document(data: &[u8]) {
    let Some(text) = text(data) else { return };
    if let Ok(MarketDocument(market)) = serde_json::from_str(text) {
        let written = serde_json::to_string(&MarketDocument(market.clone()))
            .unwrap_or_else(|e| panic!("market data do not write: {e}"));
        let MarketDocument(again) = serde_json::from_str(&written)
            .unwrap_or_else(|e| panic!("market data written back do not read: {e}\n{written}"));
        assert_eq!(again, market, "market data written back read differently");
    }
}

/// Keeps the books and exercises what reads them, on at most a few dates.
fn books(portfolio: &Portfolio, market: &MarketData) {
    let Ok(books) = Books::build(portfolio, market) else {
        return;
    };
    let mut dates: Vec<Date> = portfolio
        .transactions
        .iter()
        .map(|t| t.trade_date)
        .collect();
    dates.sort();
    dates.dedup();
    dates.truncate(8);
    if let Some(&last) = dates.last() {
        let _ = books.holdings_on(last);
        let _ = books.value_on(last, market);
    }
    if let (Ok(values), Ok(holdings)) = (
        books.value_series(&dates, market),
        books.holding_series(&dates, market),
    ) {
        for timing in [FlowTiming::StartOfDay, FlowTiming::EndOfDay] {
            if let Ok(performance) = Performance::from_values(&values, timing)
                && let (Some(&first), Some(&last)) = (dates.first(), dates.last())
            {
                let _ = performance.time_weighted(first, last);
                let _ = performance.money_weighted(first, last);
                let _ = performance.modified_dietz(first, last);
                let _ = performance.contributions(&holdings, first, last);
            }
        }
    }
}

/// Numbers from bytes, eight at a time.
fn numbers(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(8)
        .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}

/// Arbitrary numbers as returns, through every statistic.
pub fn returns(data: &[u8]) {
    let Some((&control, rest)) = data.split_first() else {
        return;
    };
    let values = numbers(rest);
    let options = StatisticsOptions {
        periods_per_year: f64::from(control % 13) * 21.0,
        ..Default::default()
    };
    let _ = risk::statistics(&values, &options);
    let half = values.len() / 2;
    let _ = risk::relative_statistics(&values[..half], &values[half..half * 2], &options);
    let confidence = f64::from(control) / 256.0;
    for method in [
        TailMethod::Historical,
        TailMethod::Parametric,
        TailMethod::CornishFisher,
    ] {
        let _ = risk::tail_risk(&values, confidence, method);
    }
    let _ = risk::rolling_statistics(&values, usize::from(control % 8) + 2, &options);
    if let Ok(dates) = (0..values.len())
        .map(|i| Date::from_days_since_epoch(i32::try_from(i).unwrap_or(i32::MAX)))
        .collect::<finalytics::Result<Vec<_>>>()
        && let Ok(levels) = finalytics::TimeSeries::new(dates, values.clone())
    {
        let _ = risk::drawdowns(&levels);
    }
    let _ = risk::max_drawdown(&values);
}

/// A small problem from bytes: the first byte picks the number of assets and the objective; the
/// rest are numbers for a factor of the covariance, expected returns and bounds. Every solution
/// with weights must report how far they break the constraints.
pub fn optimisation(data: &[u8]) {
    let Some((&control, rest)) = data.split_first() else {
        return;
    };
    let n = usize::from(control % 5) + 1;
    let values = numbers(rest);
    if values.len() < n * n + n {
        return;
    }
    let labels: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
    let factor = &values[..n * n];
    // A covariance of the form F F', positive semidefinite whenever the numbers are finite.
    let mut covariance = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            covariance[i * n + j] = (0..n).map(|k| factor[i * n + k] * factor[j * n + k]).sum();
        }
    }
    let Ok(matrix) = Matrix::new(labels.clone(), covariance) else {
        return;
    };
    let mut problem = Problem::new(matrix).with_expected_returns(values[n * n..n * n + n].to_vec());
    if let Some(&bound) = values.get(n * n + n) {
        problem = problem.with_constraints(Constraints {
            bounds: finalytics::opt::Bound {
                min: -bound.abs().min(1.0),
                max: 1.0,
            },
            leverage: Some(2.0),
            ..Default::default()
        });
    }
    let objective = match control / 5 % 8 {
        0 => Objective::MinimumVariance,
        1 => Objective::MeanVariance { risk_aversion: 3.0 },
        2 => Objective::MaximumSharpe {
            risk_free_rate: 0.0,
        },
        3 => Objective::TargetVolatility { target: 0.1 },
        4 => Objective::MaximumDiversification,
        5 => Objective::RiskBudgeting { budgets: None },
        6 => Objective::HierarchicalRiskParity {
            linkage: Default::default(),
        },
        _ => Objective::TargetReturn { target: 0.05 },
    };
    if let Ok(solution) = problem.solve(&objective)
        && solution.weights.is_some()
    {
        assert!(
            solution.max_violation.is_some(),
            "a solution without its violation"
        );
    }
    let scenarios = values.chunks_exact(n).take(64).map(<[f64]>::to_vec);
    let mut columns = vec![Vec::new(); n];
    for row in scenarios {
        for (c, v) in columns.iter_mut().zip(row) {
            c.push(v);
        }
    }
    if let Ok(table) = ReturnsTable::new(labels, columns) {
        for method in [
            risk::CovarianceMethod::Sample,
            risk::CovarianceMethod::Ewma { decay: 0.94 },
            risk::CovarianceMethod::LedoitWolf,
            risk::CovarianceMethod::Oas,
        ] {
            let _ = risk::covariance(&table, method);
        }
        let _ = Problem::from_scenarios(table).solve(&Objective::MinimumCvar { confidence: 0.9 });
    }
}
