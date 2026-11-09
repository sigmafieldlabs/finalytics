//! Turnover, trade and tracking-error limits in `Assets::optimise`, its report and frontier, and
//! at the refits of a walk-forward backtest.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::backtest::{Rebalance, Years};
use finalytics::{Assets, Error, Method, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

const CURRENT: [(&str, f64); 5] = [
    ("US Equity", 0.5),
    ("Intl Equity", 0.1),
    ("Bonds", 0.3),
    ("Gold", 0.05),
    ("T-Bills", 0.05),
];

fn turnover(weights: &BTreeMap<String, f64>, current: &[(&str, f64)]) -> f64 {
    let now: BTreeMap<&str, f64> = current.iter().copied().collect();
    let invested: f64 = weights.values().sum();
    let mut sum: f64 = weights
        .iter()
        .map(|(a, w)| (w - now.get(a.as_str()).copied().unwrap_or(0.0)).abs())
        .sum();
    sum += now
        .iter()
        .filter(|(a, _)| !weights.contains_key(**a))
        .map(|(a, w)| {
            if *a == "T-Bills" {
                (w - (1.0 - invested)).abs()
            } else {
                w.abs()
            }
        })
        .sum::<f64>();
    sum / 2.0
}

fn paths(error: Error) -> Vec<(String, String)> {
    let Error::Validation(report) = error else {
        panic!("{error}")
    };
    report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect()
}

#[test]
fn turnover_is_limited_from_the_current_weights_and_the_report_lists_the_trades() {
    let assets = assets();
    let free = assets
        .optimise(Method::MaximumSharpe)
        .current_weights(CURRENT)
        .build()
        .unwrap();
    let free_turnover = turnover(free.results.weights.as_ref().unwrap(), &CURRENT);
    // Investing the 5% in T-bills takes 2.5% of turnover on its own.
    let found = paths(
        assets
            .optimise(Method::MaximumSharpe)
            .current_weights(CURRENT)
            .max_turnover(0.02)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found[0].1,
        "investing the 5% held in cash turns over 2.5% on its own, more than 2%"
    );
    assert!(free_turnover > 0.2, "{free_turnover}");
    let limited = assets
        .optimise(Method::MaximumSharpe)
        .current_weights(CURRENT)
        .max_turnover(0.1)
        .with_frontier()
        .build()
        .unwrap();
    let weights = limited.results.weights.as_ref().unwrap();
    let found = turnover(weights, &CURRENT);
    assert!(found <= 0.1 + 1e-6, "{found}");
    let trades = limited.report.trades.as_ref().unwrap();
    assert!(
        (trades.turnover - found).abs() < 1e-9,
        "{} {found}",
        trades.turnover
    );
    assert!(trades.trades.iter().any(|t| t.asset == "T-Bills"));
    // The frontier is drawn under the same limit: no point is reached by more turnover.
    assert!(!limited.report.frontier.is_empty());
    let points = assets
        .optimise(Method::MaximumSharpe)
        .current_weights(CURRENT)
        .max_turnover(0.1)
        .frontier(8)
        .unwrap();
    for p in &points {
        assert!(turnover(&p.weights, &CURRENT) <= 0.1 + 1e-6);
    }
    // Both forms of the report show the trades and the limit.
    let text = limited.report.to_text(None);
    assert!(text.contains("Trades from the current weights"), "{text}");
    assert!(text.lines().all(|l| l.chars().count() <= 80));
    let words: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.contains("at most 10% turnover from the current weights"),
        "{text}"
    );
    let html = limited.report.to_html(None);
    assert!(html.contains("Trades from the current weights"));
    assert!(html.contains("Turnover"));
    let tables = limited.report.tables();
    assert!(tables.iter().any(|t| t.name == "trades"));
}

#[test]
fn trades_are_at_least_the_minimum_and_at_most_the_count() {
    let assets = assets();
    let result = assets
        .optimise(Method::MinimumVariance)
        .current_weights(CURRENT)
        .min_trade(0.03)
        .max_trades(2)
        .build()
        .unwrap();
    let weights = result.results.weights.as_ref().unwrap();
    let now: BTreeMap<&str, f64> = CURRENT.iter().copied().collect();
    let traded: Vec<f64> = weights
        .iter()
        .map(|(a, w)| (w - now.get(a.as_str()).copied().unwrap_or(0.0)).abs())
        .filter(|d| *d > 1e-7)
        .collect();
    assert!(traded.len() <= 2, "{traded:?}");
    assert!(traded.iter().all(|d| *d >= 0.03 - 1e-7), "{traded:?}");
    assert_eq!(result.report.trades.as_ref().unwrap().count, traded.len());
}

#[test]
fn tracking_error_is_limited_against_the_benchmark_weights() {
    let assets = assets();
    let benchmark = [("US Equity", 0.6), ("Bonds", 0.4)];
    let free = assets
        .optimise(Method::MaximumSharpe)
        .benchmark_weights(benchmark)
        .build()
        .unwrap();
    let te = free
        .report
        .summary
        .as_ref()
        .unwrap()
        .tracking_error
        .unwrap();
    assert!(te > 0.03, "{te}");
    let limited = assets
        .optimise(Method::MaximumSharpe)
        .benchmark_weights(benchmark)
        .max_tracking_error(0.03)
        .build()
        .unwrap();
    let te = limited
        .report
        .summary
        .as_ref()
        .unwrap()
        .tracking_error
        .unwrap();
    assert!(te <= 0.03 + 1e-6, "{te}");
    assert!(limited.report.to_text(None).contains("Tracking error"));
}

#[test]
fn mistakes_with_the_trade_limits_are_reported_together() {
    let assets = assets();
    let found = paths(
        assets
            .optimise(Method::MaximumSharpe)
            .max_turnover(0.2)
            .max_tracking_error(0.03)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [
            (
                "options.max_turnover".to_owned(),
                "limits trades from the weights held now: give current_weights too".to_owned()
            ),
            (
                "options.max_tracking_error".to_owned(),
                "limits the tracking error against a benchmark: give benchmark_weights too"
                    .to_owned()
            ),
        ]
    );
    let found = paths(
        assets
            .optimise(Method::MaximumSharpe)
            .current_weights([("US Equity", 0.5), ("Bitcoin", 0.2)])
            .max_turnover(1.5)
            .build()
            .unwrap_err(),
    );
    let at: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        at,
        [
            "options.current_weights.Bitcoin",
            "options.current_weights",
            "options.max_turnover"
        ]
    );
    let found = paths(
        assets
            .optimise(Method::MaximumSharpe)
            .current_weights([("US Equity", 0.5), ("Bonds", 0.3)])
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].0, "options.current_weights");
    assert!(
        found[0]
            .1
            .contains("give T-Bills, the risk-free asset, any cash"),
        "{found:?}"
    );
    let found = paths(
        assets
            .optimise(Method::MinimumCvar)
            .current_weights(CURRENT)
            .max_trades(2)
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].0, "options.max_trades");
    assert!(
        found[0]
            .1
            .starts_with("minimum CVaR does not take max_trades"),
        "{found:?}"
    );
    // A method that takes no limits refuses them, as it does every limit.
    let found = paths(
        assets
            .optimise(Method::EqualWeight)
            .current_weights(CURRENT)
            .max_turnover(0.1)
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].0, "options.max_turnover");
    // Current weights alone only add the trades to the report.
    let equal = assets
        .optimise(Method::EqualWeight)
        .current_weights(CURRENT)
        .build()
        .unwrap();
    assert_eq!(equal.report.trades.as_ref().unwrap().count, 5);
}

#[test]
fn a_walk_forward_limits_the_trades_at_each_refit_from_the_weights_held() {
    let assets = assets();
    let result = assets
        .backtest(Method::MaximumSharpe)
        .walk_forward(Years(5))
        .rebalance(Rebalance::Yearly)
        .max_turnover(0.05)
        .build()
        .unwrap();
    let run = result.results.strategies[0].out_of_sample.as_ref().unwrap();
    // The first fit enters from cash; every refit after it trades at most 5%.
    assert!(run.trades.len() > 10);
    for trade in run.trades.iter().filter(|t| t.refit) {
        assert!(
            trade.turnover <= 0.05 + 1e-6,
            "{} {}",
            trade.date,
            trade.turnover
        );
    }
    let free = assets
        .backtest(Method::MaximumSharpe)
        .walk_forward(Years(5))
        .rebalance(Rebalance::Yearly)
        .build()
        .unwrap();
    let most = free.results.strategies[0]
        .out_of_sample
        .as_ref()
        .unwrap()
        .trades
        .iter()
        .map(|t| t.turnover)
        .fold(0.0, f64::max);
    assert!(most > 0.05, "{most}");
    let words: String = result
        .report
        .to_text(None)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        words.contains("At most 5% turnover at each refit"),
        "{words}"
    );
    // In sample, the weights are fitted once: a turnover limit has nothing to limit.
    let found = paths(
        assets
            .backtest(Method::MaximumSharpe)
            .max_turnover(0.05)
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].0, "options.max_turnover");
    let mut options = assets
        .compare([Method::MaximumSharpe, Method::MinimumVariance])
        .walk_forward(Years(5))
        .options()
        .unwrap();
    for strategy in &mut options.strategies {
        if let finalytics::backtest::Strategy::Optimise { options, .. } = strategy {
            options.current_weights = CURRENT.iter().map(|(a, w)| ((*a).to_owned(), *w)).collect();
        }
    }
    let found = paths(assets.compare_with(&options).unwrap_err());
    assert_eq!(found[0].0, "options.current_weights");
    // A tracking-error limit applies at every fit, in sample too.
    let te = assets
        .compare([Method::MaximumSharpe, Method::EqualWeight])
        .benchmark_weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .max_tracking_error(0.02)
        .build()
        .unwrap();
    assert_eq!(
        te.results.strategies[1].notes,
        ["Fully invested and long only; the limits on weights do not apply."]
    );
}

/// The limits reach the optimiser as its own constraints, which `scripts/reference/
/// optimisation.py` checks against CVXPY: the same problem built by hand gives the same weights.
#[test]
fn the_limits_are_the_optimisers_constraints() {
    use finalytics::opt::{Constraints, Objective, Problem, TrackingErrorLimit, Turnover};
    let assets = assets();
    let risky: Vec<&str> = assets.risky_assets();
    let columns: Vec<Vec<f64>> = risky
        .iter()
        .map(|a| {
            let k = assets.labels().iter().position(|l| l == a).unwrap();
            assets.returns().columns()[k].clone()
        })
        .collect();
    let table = finalytics::risk::ReturnsTable::new(
        risky.iter().map(|a| (*a).to_owned()).collect(),
        columns,
    )
    .unwrap();
    let covariance =
        finalytics::risk::covariance(&table, finalytics::risk::CovarianceMethod::LedoitWolf)
            .unwrap()
            .matrix
            .scaled(12.0);
    let means = assets.mean_returns();
    let mu: Vec<f64> = risky
        .iter()
        .map(|a| means[assets.labels().iter().position(|l| l == a).unwrap()])
        .collect();
    let now: BTreeMap<&str, f64> = CURRENT.iter().copied().collect();
    let current: Vec<f64> = risky
        .iter()
        .map(|a| now.get(a).copied().unwrap_or(0.0))
        .collect();
    let benchmark: Vec<f64> = risky
        .iter()
        .map(|a| match *a {
            "US Equity" => 0.6,
            "Bonds" => 0.4,
            _ => 0.0,
        })
        .collect();
    let mut constraints = Constraints::default();
    let mut turnover = Turnover::new(current);
    // 15% in all, of which investing the 5% of cash takes 2.5%.
    turnover.limit = Some(0.15 - 0.025);
    constraints.turnover = Some(turnover);
    constraints.tracking_error = Some(TrackingErrorLimit {
        benchmark,
        limit: 0.04,
    });
    let by_hand = Problem::from_covariance(covariance)
        .with_expected_returns(mu)
        .with_constraints(constraints)
        .solve(&Objective::MeanVariance { risk_aversion: 3.0 })
        .unwrap();
    let result = assets
        .optimise(Method::MeanVariance)
        .risk_aversion(3.0)
        .current_weights(CURRENT)
        .max_turnover(0.15)
        .benchmark_weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .max_tracking_error(0.04)
        .build()
        .unwrap();
    // To the no-trade tolerance, within which the engine makes a weight exact.
    let weights = result.results.weights.unwrap();
    for (a, w) in risky.iter().zip(by_hand.weights.unwrap()) {
        assert!((weights[*a] - w).abs() < 1e-7, "{a}: {} {w}", weights[*a]);
    }
}
