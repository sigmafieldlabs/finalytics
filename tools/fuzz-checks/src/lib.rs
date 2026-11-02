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
    ("attribution", attribution),
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
        rebalancing(portfolio, &books, market, last);
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

/// Rebalances the first account, and all of them together, to equal weights in every instrument,
/// with fees and tax rates. A proposal must turn into transactions, and a sale must report its
/// tax.
fn rebalancing(portfolio: &Portfolio, books: &Books, market: &MarketData, date: Date) {
    let Some(first) = portfolio.accounts.first() else {
        return;
    };
    let n = portfolio.instruments.len().max(1) as f64;
    let targets: Vec<(finalytics::HoldingKey, f64)> = portfolio
        .instruments
        .iter()
        .map(|i| (finalytics::HoldingKey::Instrument(i.id.clone()), 0.9 / n))
        .collect();
    let options = finalytics::opt::RebalanceOptions {
        fees: finalytics::opt::Fees {
            rate: finalytics::Decimal::new(1, 3),
            per_trade: finalytics::Decimal::ZERO,
            minimum: finalytics::Decimal::ONE,
        },
        tax: Some(finalytics::opt::TaxRates {
            short_term: finalytics::Decimal::new(3, 1),
            long_term: finalytics::Decimal::new(15, 2),
            long_term_after_days: 365,
        }),
        ..Default::default()
    };
    if let Ok(plan) = finalytics::opt::rebalance(books, market, date, &first.id, &targets, &options)
    {
        assert!(
            plan.trades
                .iter()
                .all(|t| (t.side == finalytics::opt::Side::Sell) == t.tax_base.is_some()),
            "a sale without its tax, or a purchase with one"
        );
        let _ = plan.transactions("fuzz-");
    }
    let accounts: Vec<_> = portfolio.accounts.iter().map(|a| a.id.clone()).collect();
    let _ = finalytics::opt::rebalance_accounts(books, market, date, &accounts, &targets, &options);
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

/// Attribution from bytes: the first byte picks the number of segments and periods, the linking
/// method and whether the attribution is by currency (with or without given exposures); the rest
/// are numbers for the weights (the last of each side making up the rest of one), returns and
/// currency returns. In every accepted result, the effects must add up to the excess return in
/// each period and over all of them, to within rounding of the inputs' size, the linked effects
/// must be the periods' effects multiplied by their coefficients, and the result must read back
/// equal from JSON.
pub fn attribution(data: &[u8]) {
    use finalytics::{
        AttributionPeriod, CurrencyAttributionPeriod, CurrencyReturn, CurrencySegment, Linking,
        Segment, SegmentEffects,
    };
    let Some((&control, rest)) = data.split_first() else {
        return;
    };
    let n = usize::from(control % 4) + 1;
    let periods = usize::from(control / 4 % 4) + 1;
    let linking = if control & 0x10 == 0 {
        Linking::Carino
    } else {
        Linking::Menchero
    };
    let mode = (control >> 5) % 3;
    let values = numbers(rest);
    let per_period = 4 * n + 6 + 4;
    if values.len() < periods * per_period {
        return;
    }
    let currencies = [Currency::USD, Currency::EUR, Currency::JPY];
    let dates: Vec<Date> = (0..=periods)
        .map(|k| {
            Date::from_ymd(2026, 1, 1)
                .and_then(|d| d.add_days(7 * k as i32))
                .unwrap_or_else(|e| panic!("{e}"))
        })
        .collect();
    // The last weight makes the others up to one, so that short positions are explored.
    let weights = |raw: &[f64]| -> Vec<f64> {
        let mut w = raw.to_vec();
        w[n - 1] = 1.0 - raw[..n - 1].iter().sum::<f64>();
        w
    };
    let mut plain = Vec::new();
    let mut by_currency = Vec::new();
    for k in 0..periods {
        let v = &values[k * per_period..(k + 1) * per_period];
        let (w, b) = (weights(&v[..n]), weights(&v[n..2 * n]));
        let (r, s) = (&v[2 * n..3 * n], &v[3 * n..4 * n]);
        let segments: Vec<Segment> = (0..n)
            .map(|i| Segment {
                name: format!("s{i}"),
                portfolio_weight: w[i],
                portfolio_return: r[i],
                benchmark_weight: b[i],
                benchmark_return: s[i],
            })
            .collect();
        let rates = &v[4 * n..4 * n + 6];
        let exposures = |raw: &[f64]| {
            let mut e = raw.to_vec();
            e[2] = 1.0 - raw[0] - raw[1];
            Some(currencies.iter().copied().zip(e).collect())
        };
        let hedged = mode == 2;
        by_currency.push(CurrencyAttributionPeriod {
            start: dates[k],
            end: dates[k + 1],
            segments: segments
                .iter()
                .enumerate()
                .map(|(i, x)| CurrencySegment {
                    name: x.name.clone(),
                    currency: currencies[i % 3],
                    portfolio_weight: x.portfolio_weight,
                    portfolio_return: x.portfolio_return,
                    benchmark_weight: x.benchmark_weight,
                    benchmark_return: x.benchmark_return,
                })
                .collect(),
            currencies: (0..3)
                .map(|i| CurrencyReturn {
                    currency: currencies[i],
                    spot_return: if i == 0 { 0.0 } else { rates[2 * i] },
                    cash_return: rates[2 * i + 1],
                })
                .collect(),
            portfolio_exposures: if hedged {
                exposures(&v[4 * n + 6..4 * n + 9])
            } else {
                None
            },
            benchmark_exposures: if hedged {
                exposures(&[v[4 * n + 9], v[4 * n + 7], v[4 * n + 8]])
            } else {
                None
            },
        });
        plain.push(AttributionPeriod {
            start: dates[k],
            end: dates[k + 1],
            segments,
        });
    }
    // Floating point holds the identities only to within rounding of the numbers involved, so
    // each period's tolerance scales with the size of its weights and returns.
    let size = |weights: f64, returns: f64| weights * (1.0 + returns).powi(2) * (1.0 + weights);
    let magnitudes: Vec<f64> = if mode == 0 {
        plain
            .iter()
            .map(|p| {
                let w: f64 = p
                    .segments
                    .iter()
                    .map(|x| x.portfolio_weight.abs() + x.benchmark_weight.abs())
                    .sum();
                let r = p
                    .segments
                    .iter()
                    .map(|x| x.portfolio_return.abs().max(x.benchmark_return.abs()))
                    .fold(0.0, f64::max);
                size(w, r)
            })
            .collect()
    } else {
        by_currency
            .iter()
            .map(|p| {
                let exposures = |e: &Option<Vec<(Currency, f64)>>| {
                    e.iter().flatten().map(|(_, w)| w.abs()).sum::<f64>()
                };
                let w: f64 = p
                    .segments
                    .iter()
                    .map(|x| x.portfolio_weight.abs() + x.benchmark_weight.abs())
                    .sum::<f64>()
                    + exposures(&p.portfolio_exposures)
                    + exposures(&p.benchmark_exposures);
                let r = p
                    .segments
                    .iter()
                    .map(|x| x.portfolio_return.abs().max(x.benchmark_return.abs()))
                    .chain(
                        p.currencies
                            .iter()
                            .map(|c| c.spot_return.abs().max(c.cash_return.abs())),
                    )
                    .fold(0.0, f64::max);
                size(w, r)
            })
            .collect()
    };
    #[track_caller]
    fn adds_up(total: f64, expected: f64, tolerance: f64, what: &str) {
        assert!(
            (total - expected).abs() <= tolerance,
            "{what}: the effects add up to {total}, not {expected}"
        );
    }
    #[track_caller]
    fn linked(linked: f64, periods: impl Iterator<Item = (f64, f64)>, what: &str) {
        let (mut sum, mut scale) = (0.0, 0.0);
        for (c, e) in periods {
            sum += c * e;
            scale += (c * e).abs();
        }
        assert!(
            (linked - sum).abs() <= 1e-12 * scale,
            "{what}: {linked} is not the periods' effects linked, {sum}"
        );
    }
    let effects = |e: &SegmentEffects| e.allocation + e.selection + e.interaction;
    if mode == 0 {
        let Ok(a) = finalytics::brinson_fachler(&plain, linking) else {
            return;
        };
        let mut overall = 1.0 + a.portfolio_return.abs() + a.benchmark_return.abs();
        for (p, m) in a.periods.iter().zip(&magnitudes) {
            let total: f64 = p.segments.iter().map(effects).sum();
            adds_up(
                total,
                p.portfolio_return - p.benchmark_return,
                1e-12 * m,
                "a period",
            );
            overall += p.linking_coefficient.abs() * m;
        }
        for (k, s) in a.segments.iter().enumerate() {
            let periods = || {
                a.periods.iter().filter_map(move |p| {
                    p.segments
                        .iter()
                        .find(|x| x.name == s.name)
                        .map(|x| (p.linking_coefficient, x))
                })
            };
            linked(
                s.allocation,
                periods().map(|(c, x)| (c, x.allocation)),
                &format!("segments[{k}].allocation"),
            );
            linked(
                s.selection,
                periods().map(|(c, x)| (c, x.selection)),
                &format!("segments[{k}].selection"),
            );
            linked(
                s.interaction,
                periods().map(|(c, x)| (c, x.interaction)),
                &format!("segments[{k}].interaction"),
            );
        }
        let totals = a.allocation + a.selection + a.interaction;
        adds_up(
            totals,
            a.excess_return,
            1e-10 * overall,
            "the linked totals",
        );
        let text = serde_json::to_string(&a).unwrap_or_else(|e| panic!("{e}"));
        let back: finalytics::Attribution =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, a, "an attribution that does not read back equal");
    } else {
        let Ok(a) = finalytics::karnosky_singer(&by_currency, linking) else {
            return;
        };
        let mut overall = 1.0 + a.portfolio_return.abs() + a.benchmark_return.abs();
        for (p, m) in a.periods.iter().zip(&magnitudes) {
            let market: f64 = p.segments.iter().map(effects).sum();
            let currency: f64 = p.currencies.iter().map(|c| c.allocation).sum();
            adds_up(
                market + currency + p.residual,
                p.portfolio_return - p.benchmark_return,
                1e-12 * m,
                "a period",
            );
            overall += p.linking_coefficient.abs() * m;
        }
        for (k, x) in a.currencies.iter().enumerate() {
            let periods = a.periods.iter().filter_map(|p| {
                p.currencies
                    .iter()
                    .find(|y| y.currency == x.currency)
                    .map(|y| (p.linking_coefficient, y.allocation))
            });
            linked(
                x.allocation,
                periods,
                &format!("currencies[{k}].allocation"),
            );
        }
        linked(
            a.residual,
            a.periods
                .iter()
                .map(|p| (p.linking_coefficient, p.residual)),
            "residual",
        );
        let totals = a.allocation + a.selection + a.interaction + a.currency + a.residual;
        adds_up(
            totals,
            a.excess_return,
            1e-10 * overall,
            "the linked totals",
        );
        let text = serde_json::to_string(&a).unwrap_or_else(|e| panic!("{e}"));
        let back: finalytics::CurrencyAttribution =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, a, "an attribution that does not read back equal");
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
        turnover: (control & 0x80 != 0).then(|| finalytics::opt::Turnover {
            current: vec![1.0 / n as f64; n],
            limit: None,
            costs: None,
            quadratic_costs: Some(vec![bound.abs().min(10.0); n]),
        }),
        trades: (control & 0x80 != 0).then_some(finalytics::opt::TradeLimits {
            minimum: Some(0.01),
            maximum_count: Some(n.div_ceil(2)),
        }),
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
        && let Some(weights) = &solution.weights
    {
        assert!(
            solution.max_violation.is_some(),
            "a solution without its violation"
        );
        if let Some(limits) = problem.constraints.trades.as_ref()
            && let Some(most) = limits.maximum_count
        {
            let traded = weights
                .iter()
                .filter(|w| (**w - 1.0 / n as f64).abs() > 1e-7)
                .count();
            assert!(traded <= most, "{traded} trades, more than {most}");
        }
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
