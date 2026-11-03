//! Backtests over `Assets`: methods refitted through time, fixed weights and benchmarks.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::{BacktestOptions, Benchmark, Rebalance, Strategy, Validation};
use finalytics::opt::Bound;
use finalytics::{Assets, Error, ObjectiveName, OptimiseOptions, datasets};

fn method(objective: ObjectiveName, max_weight: f64) -> Strategy {
    let mut options = OptimiseOptions::default();
    options.objective = objective;
    options.bounds = Some(Bound {
        min: 0.0,
        max: max_weight,
    });
    Strategy::optimise(options)
}

fn paths(error: Error) -> Vec<String> {
    match error {
        Error::Validation(report) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, found {other}"),
    }
}

#[test]
fn methods_walk_forward_on_the_sp500() {
    let assets = Assets::from_prices(&datasets::sp500())
        .unwrap()
        .with_risk_free_asset("T-Bills")
        .unwrap();
    let mut options = BacktestOptions::default();
    for objective in [
        ObjectiveName::EqualWeight,
        ObjectiveName::InverseVolatility,
        ObjectiveName::MaximumSharpe,
        ObjectiveName::HierarchicalRiskParity,
    ] {
        options.strategies.push(method(objective, 0.10));
    }
    // Yearly refits keep the test quick; the performance budget times quarterly ones.
    options.rebalance = Rebalance::Yearly;
    options.validation = Validation::walk_forward(5.0);
    options.trading_cost_bps = 10.0;
    options.benchmark = Some(Benchmark::Prices {
        name: None,
        prices: datasets::sp500_index(),
    });
    let backtest = assets.simulate(&options).unwrap();
    let names: Vec<&str> = backtest
        .strategies
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Equal weight",
            "Inverse volatility",
            "Maximum Sharpe",
            "Hierarchical risk parity",
        ]
    );
    assert_eq!(backtest.assets.len(), 55);
    let first_test = assets.dates()[1260];
    for s in &backtest.strategies {
        let out = s.out_of_sample.as_ref().unwrap();
        assert_eq!(out.dates[0], first_test);
        assert_eq!(out.returns.len(), assets.dates().len() - 1260);
        assert!(out.returns.iter().all(|r| r.is_finite()));
        for trade in &out.trades {
            let total: f64 = trade.weights.values().sum();
            assert!((total - 1.0).abs() < 1e-6, "{}: {total}", s.name);
        }
        // Year ends from 2010 to 2025.
        assert_eq!(out.trades.iter().filter(|t| t.refit).count(), 16);
    }
    // The limits apply to maximum Sharpe, and not to the methods that take none.
    for s in [
        &backtest.strategies[0],
        &backtest.strategies[1],
        &backtest.strategies[3],
    ] {
        assert_eq!(s.notes.len(), 1, "{}", s.name);
    }
    let maximum_sharpe = &backtest.strategies[2];
    assert!(maximum_sharpe.notes.is_empty());
    let out = maximum_sharpe.out_of_sample.as_ref().unwrap();
    assert!(
        out.trades
            .iter()
            .all(|t| t.weights.values().all(|w| *w <= 0.10 + 1e-6))
    );
    let equal = &backtest.strategies[0].out_of_sample.as_ref().unwrap().start;
    assert!(equal.values().all(|w| (w - 1.0 / 55.0).abs() < 1e-12));
    let benchmark = backtest.benchmark.unwrap();
    assert_eq!(benchmark.name, "S&P 500");
    assert_eq!(benchmark.out_of_sample.unwrap().dates[0], first_test);
}

#[test]
fn a_holdout_tests_fixed_weights_and_a_fitted_method_against_a_benchmark() {
    let assets = Assets::from_prices(&datasets::asset_class_prices())
        .unwrap()
        .with_risk_free_asset("T-Bills")
        .unwrap();
    let mut options = BacktestOptions::default();
    options.strategies = vec![
        method(ObjectiveName::MaximumSharpe, 0.4),
        Strategy::weights(
            "Cautious",
            [("US Equity", 0.3), ("Bonds", 0.5), ("T-Bills", 0.2)],
        ),
    ];
    options.validation = Validation::Holdout {
        start: "2016-11-01".parse().unwrap(),
    };
    options.benchmark = Some(Benchmark::Weights {
        name: Some("60/40".into()),
        weights: [("US Equity".to_owned(), 0.6), ("Bonds".to_owned(), 0.4)].into(),
    });
    let backtest = assets.simulate(&options).unwrap();
    let fitted = &backtest.strategies[0];
    assert_eq!(
        fitted.in_sample.dates.last().unwrap().to_string(),
        "2016-10-31"
    );
    let out = fitted.out_of_sample.as_ref().unwrap();
    assert_eq!(out.dates[0].to_string(), "2016-11-30");
    assert_eq!(out.start, fitted.in_sample.start);
    // The cautious mix holds a fifth in T-bills, as cash.
    let cautious = &backtest.strategies[1].in_sample;
    assert_eq!(cautious.start.values().sum::<f64>(), 0.8);
    let cash = backtest.cash[0];
    let r = &assets.returns().columns();
    let expected = 0.3 * r[0][0] + 0.5 * r[2][0] + 0.2 * cash;
    assert!((cautious.returns[0] - expected).abs() < 1e-15);
    assert_eq!(backtest.benchmark.unwrap().name, "60/40");
}

#[test]
fn mistakes_in_the_options_are_reported_together() {
    let assets = Assets::from_prices(&datasets::asset_class_prices())
        .unwrap()
        .with_risk_free_asset("T-Bills")
        .unwrap();
    let mut options = BacktestOptions::default();
    options.strategies = vec![
        Strategy::weights("Mine", [("US Equity", 0.5), ("Crypto", 0.4)]),
        method(ObjectiveName::MinimumVariance, 0.4),
    ];
    options.rebalance = Rebalance::Never;
    options.validation = Validation::walk_forward(5.0);
    options.benchmark = Some(Benchmark::Prices {
        name: None,
        prices: datasets::asset_class_prices(),
    });
    assert_eq!(
        paths(assets.simulate(&options).unwrap_err()),
        [
            "strategies[0].weights.Crypto",
            "strategies[0].weights",
            "benchmark.prices",
        ]
    );
    options.strategies.remove(0);
    options.benchmark = None;
    assert_eq!(
        paths(assets.simulate(&options).unwrap_err()),
        ["settings.rebalance"]
    );
}

/// The backtests of `scripts/reference/backtest.py`, simulated independently with NumPy and
/// solved in CVXPY.
#[test]
fn backtests_match_the_reference_simulation() {
    use serde_json::Value;
    let fixture: Value = serde_json::from_str(include_str!("fixtures/backtest.json")).unwrap();
    let assets = Assets::from_prices(&datasets::asset_class_prices())
        .unwrap()
        .with_risk_free_asset("T-Bills")
        .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let options: BacktestOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let backtest = assets.simulate(&options).unwrap();
        let expected = &case["expected"];
        // Equal weight, inverse volatility and fixed weights agree to rounding; minimum variance
        // agrees to the solvers' tolerance, which leaves a weight that belongs at a bound a few
        // millionths from it.
        let solved = |s: &finalytics::backtest::StrategyResult| s.name == "Minimum variance";
        let mut pairs: Vec<(String, &finalytics::backtest::StrategyResult, &Value, f64)> = backtest
            .strategies
            .iter()
            .zip(expected["strategies"].as_array().unwrap())
            .map(|(s, e)| {
                let tolerance = if solved(s) { 1e-5 } else { 1e-12 };
                (format!("{name}: {}", s.name), s, e, tolerance)
            })
            .collect();
        if let Some(benchmark) = &backtest.benchmark {
            pairs.push((
                format!("{name}: benchmark"),
                benchmark,
                &expected["benchmark"],
                1e-12,
            ));
        }
        assert_eq!(
            pairs.len(),
            expected["strategies"].as_array().unwrap().len()
                + usize::from(!expected["benchmark"].is_null())
        );
        for (label, got, want, tolerance) in pairs {
            compare(
                &format!("{label}, in sample"),
                &got.in_sample,
                &want["in_sample"],
                tolerance,
            );
            match (&got.out_of_sample, &want["out_of_sample"]) {
                (Some(run), want) if !want.is_null() => {
                    compare(&format!("{label}, out of sample"), run, want, tolerance);
                }
                (None, want) if want.is_null() => {}
                _ => panic!("{label}: out of sample differs"),
            }
            assert!(got.failures.is_empty(), "{label}");
        }
    }
}

fn compare(label: &str, run: &finalytics::backtest::Run, want: &serde_json::Value, tolerance: f64) {
    let dates: Vec<String> = run.dates.iter().map(ToString::to_string).collect();
    let want_dates: Vec<String> = serde_json::from_value(want["dates"].clone()).unwrap();
    assert_eq!(dates, want_dates, "{label}: dates");
    let returns: Vec<f64> = serde_json::from_value(want["returns"].clone()).unwrap();
    for (t, (a, b)) in run.returns.iter().zip(&returns).enumerate() {
        assert!(
            (a - b).abs() < tolerance,
            "{label}: return {t}: {a} against {b}"
        );
    }
    for (asset, w) in want["start"].as_object().unwrap() {
        let w = w.as_f64().unwrap();
        assert!(
            (run.start[asset] - w).abs() < tolerance,
            "{label}: start {asset}"
        );
    }
    let trades = want["trades"].as_array().unwrap();
    assert_eq!(run.trades.len(), trades.len(), "{label}: trades");
    for (got, want) in run.trades.iter().zip(trades) {
        assert_eq!(
            got.date.to_string(),
            want["date"].as_str().unwrap(),
            "{label}"
        );
        assert_eq!(
            got.refit,
            want["refit"].as_bool().unwrap(),
            "{label}: refit"
        );
        for field in ["turnover", "cost"] {
            let value = if field == "turnover" {
                got.turnover
            } else {
                got.cost
            };
            let expected = want[field].as_f64().unwrap();
            assert!(
                (value - expected).abs() < tolerance,
                "{label}: {field} on {}",
                got.date
            );
        }
        for (asset, w) in want["weights"].as_object().unwrap() {
            let w = w.as_f64().unwrap();
            assert!(
                (got.weights[asset] - w).abs() < tolerance,
                "{label}: {asset} on {}",
                got.date
            );
        }
    }
}
