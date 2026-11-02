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
    ("views", views),
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

/// A Black–Litterman model from bytes: the first byte picks the number of assets and views; the
/// rest are numbers for a factor of the covariance, market weights, the views' portfolios and
/// returns, and their uncertainties. An estimate must be finite, and its model must read back
/// equal.
pub fn views(data: &[u8]) {
    let Some((&control, rest)) = data.split_first() else {
        return;
    };
    let n = usize::from(control % 4) + 1;
    let k = usize::from(control / 4 % 4);
    let values = numbers(rest);
    if values.len() < n * n + n + k * (n + 2) {
        return;
    }
    let labels: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
    let factor = &values[..n * n];
    let mut covariance = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            covariance[i * n + j] = (0..n).map(|m| factor[i * n + m] * factor[j * n + m]).sum();
        }
    }
    let Ok(covariance) = Matrix::new(labels.clone(), covariance) else {
        return;
    };
    let mut at = n * n + n;
    let mut views = Vec::with_capacity(k);
    for v in 0..k {
        let portfolio = labels
            .iter()
            .zip(&values[at..at + n])
            .filter(|(_, w)| **w != 0.0)
            .map(|(l, w)| (l.clone(), *w))
            .collect();
        let expected_return = values[at + n];
        let u = values[at + n + 1];
        let uncertainty = match (control as usize + v) % 3 {
            0 => finalytics::opt::ViewUncertainty::Proportional,
            1 => finalytics::opt::ViewUncertainty::Confidence { confidence: u },
            _ => finalytics::opt::ViewUncertainty::Variance { variance: u },
        };
        views.push(finalytics::opt::View {
            portfolio,
            expected_return,
            uncertainty,
        });
        at += n + 2;
    }
    let model = finalytics::opt::BlackLitterman {
        covariance,
        prior: finalytics::opt::Prior::Market {
            weights: values[n * n..n * n + n].to_vec(),
            risk_aversion: 2.5,
            risk_free_rate: 0.0,
        },
        views,
        tau: 0.05,
    };
    if let Ok(estimate) = model.estimate() {
        assert!(
            estimate
                .posterior_returns
                .iter()
                .chain(estimate.posterior_covariance.values())
                .all(|v| v.is_finite()),
            "an estimate with a non-finite posterior"
        );
        // A valid model holds only finite numbers, so it must read back equal.
        let text = serde_json::to_string(&model).unwrap_or_else(|e| panic!("{e}"));
        let back: finalytics::opt::BlackLitterman =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, model, "a model that does not read back equal");
    }
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
/// rest are numbers for a factor of the covariance, expected returns and bounds. The problem is
/// solved with the covariance matrix and, in factor form, with the same risk as a factor model
/// and an exposure limit. Every solution with weights must report how far they break the
/// constraints. The numbers also make a table of returns for the covariance estimators, the
/// factor model estimators and minimum CVaR.
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
    let expected = values[n * n..n * n + n].to_vec();
    let constraints = values.get(n * n + n).map(|&bound| Constraints {
        bounds: finalytics::opt::Bound {
            min: -bound.abs().min(1.0),
            max: 1.0,
        },
        leverage: Some(2.0),
        ..Default::default()
    });
    let mut problem = Problem::new(matrix).with_expected_returns(expected.clone());
    if let Some(c) = &constraints {
        problem = problem.with_constraints(c.clone());
    }
    let objective = match control / 5 % 10 {
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
        7 => Objective::RobustMeanVariance {
            risk_aversion: 3.0,
            uncertainty: finalytics::opt::ReturnUncertainty::Ellipsoid {
                size: values.get(n * n + n + 1).map_or(0.5, |v| v.abs().min(1e6)),
                covariance: None,
            },
        },
        8 => Objective::RobustMeanVariance {
            risk_aversion: 3.0,
            uncertainty: finalytics::opt::ReturnUncertainty::Box {
                half_widths: values[n * n..n * n + n].iter().map(|v| v.abs()).collect(),
            },
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
    // A small resampling run, which must account for every simulated problem.
    let resampling = finalytics::opt::Resampling {
        samples: 2,
        observations: 3,
        seed: u64::from(control),
    };
    if let Ok(found) = problem.resampled(&objective, &resampling) {
        assert_eq!(
            found.solved + found.failed,
            2,
            "a simulated problem went missing"
        );
        assert_eq!(found.weights.is_some(), found.solved > 0);
    }
    // The same covariance as a factor model, with the rows of the factor as exposures and
    // uncorrelated unit-variance factors, solved in factor form with an exposure limit.
    let factors: Vec<String> = (0..n).map(|k| format!("f{k}")).collect();
    let identity: Vec<f64> = (0..n * n)
        .map(|k| if k % (n + 1) == 0 { 1.0 } else { 0.0 })
        .collect();
    let exposures: Vec<Vec<f64>> = factor.chunks(n).map(<[f64]>::to_vec).collect();
    if let Ok(identity) = Matrix::new(factors, identity)
        && let Ok(model) = risk::FactorModel::new(labels.clone(), exposures, identity, vec![0.0; n])
    {
        let _ = risk::factor_risk(&vec![1.0 / n as f64; n], &model);
        let mut limited = constraints.unwrap_or_default();
        limited
            .factor_exposures
            .push(finalytics::opt::FactorExposureLimit {
                factor: "f0".into(),
                min: -1.0,
                max: 1.0,
                benchmark: None,
            });
        let factor_problem = Problem::from_factor_model(model)
            .with_expected_returns(expected)
            .with_constraints(limited);
        if let Ok(solution) = factor_problem.solve(&objective)
            && solution.weights.is_some()
        {
            assert!(
                solution.max_violation.is_some(),
                "a factor-model solution without its violation"
            );
        }
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
        let observations = table.observations();
        let _ = risk::statistical_model(&table, n.min(observations - 1).max(1));
        let _ = risk::regression_model(&table, &table);
        let _ = Problem::from_scenarios(table).solve(&Objective::MinimumCvar { confidence: 0.9 });
    }
}
