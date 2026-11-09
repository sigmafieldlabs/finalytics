//! Cash in optimisation: a portfolio's own cash valued on the estimates the overview report
//! makes, and limits on cash that let optimisation hold it, riskless, beside the risky assets.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::Years;
use finalytics::reports::ReportOptions;
use finalytics::{Assets, Error, Method, datasets};

fn with_bills() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-12 * b.abs().max(1.0),
        "{what}: {a} against {b}"
    );
}

fn messages(result: Result<impl std::fmt::Debug, Error>) -> Vec<(String, String)> {
    match result {
        Err(Error::Validation(report)) => report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect(),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn the_portfolio_mix_on_the_assets_estimates_is_the_overviews_current_point() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    let overview = portfolio
        .overview_report(&ReportOptions::default())
        .unwrap();
    let covariance = assets.covariance().unwrap().matrix;
    let means = assets.mean_returns();
    let labels = assets.labels();
    let w: Vec<f64> = labels
        .iter()
        .map(|l| {
            overview
                .holdings
                .iter()
                .find(|h| h.holding.to_string() == *l)
                .map_or(0.0, |h| h.weight)
        })
        .collect();
    close(w.iter().sum(), 1.0, "weights");
    let n = w.len();
    let variance: f64 = (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .map(|(i, j)| w[i] * covariance.get(i, j) * w[j])
        .sum();
    let expected: f64 = w.iter().zip(&means).map(|(a, b)| a * b).sum();
    let current = overview.current.unwrap();
    close(variance.sqrt(), current.volatility, "volatility");
    close(expected, current.expected_return, "expected return");
    // Set aside, the cash leaves the invested holdings in their own shares.
    let invested = overview.current_without_base_cash.unwrap();
    let cash = w[labels.iter().position(|l| l == "USD cash").unwrap()];
    assert!(cash > 0.0);
    close(
        invested.volatility,
        current.volatility / (1.0 - cash),
        "invested",
    );
    close(
        invested.expected_return,
        current.expected_return / (1.0 - cash),
        "invested return",
    );
    let text = portfolio.report().build().unwrap().to_string();
    assert!(text.contains("Without USD cash"), "{text}");
    // The frontier is the instruments', the one optimisation and the valuation of weights draw.
    let frontier = overview.frontier.as_ref().unwrap();
    for (k, h) in overview.holdings.iter().enumerate() {
        if h.holding.to_string().ends_with(" cash") {
            assert!(frontier.iter().all(|p| p.weights[k] == 0.0));
        }
    }
    let least = |volatilities: Vec<f64>| volatilities.into_iter().fold(f64::INFINITY, f64::min);
    let valued = assets
        .evaluate(portfolio.weights().unwrap())
        .with_frontier()
        .build()
        .unwrap();
    close(
        least(frontier.iter().map(|p| p.volatility).collect()),
        least(
            valued
                .report
                .frontier
                .iter()
                .map(|p| p.volatility)
                .collect(),
        ),
        "the least risky point",
    );
}

#[test]
fn limits_on_cash_let_optimisation_hold_the_risk_free_asset() {
    let assets = with_bills();
    let result = assets
        .optimise(Method::MinimumVariance)
        .cash(0.0, 0.3)
        .build()
        .unwrap();
    let weights = result.results.weights.as_ref().unwrap();
    // Riskless, the cash is all the limit allows.
    assert!((weights["T-Bills"] - 0.3).abs() < 1e-6, "{weights:?}");
    close(weights.values().sum(), 1.0, "fully invested with the cash");
    let r = &result.report;
    assert_eq!(r.context.limits, ["0% to 30% in cash"]);
    assert!(!r.context.assets.contains(&"T-Bills".to_owned()));
    assert!(r.allocation.iter().any(|a| a.asset == "T-Bills"));
    let s = r.summary.as_ref().unwrap();
    close(
        s.volatility,
        result.results.volatility.unwrap(),
        "volatility",
    );
    // Without the limits, the risky assets hold everything, as before.
    let invested = assets.optimise(Method::MinimumVariance).build().unwrap();
    assert!(!invested.results.weights.unwrap().contains_key("T-Bills"));
    // The frontier then starts from all cash, riskless. It takes the limits alone.
    let frontier = assets
        .optimise(Method::MeanVariance)
        .cash(0.0, 1.0)
        .frontier(10)
        .unwrap();
    // Within the interior-point solver's tolerance.
    assert!(frontier[0].volatility < 1e-4, "{:?}", frontier[0]);
    assert!((frontier[0].weights["T-Bills"] - 1.0).abs() < 1e-3);
    let drawn = assets
        .optimise(Method::MeanVariance)
        .risk_aversion(3.0)
        .cash(0.0, 1.0)
        .with_frontier()
        .build()
        .unwrap()
        .report
        .frontier;
    assert!(drawn[0].volatility < 1e-4, "{:?}", drawn[0]);
}

#[test]
fn without_a_risk_free_asset_cash_earns_the_risk_free_rate() {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_rate(0.02)
        .build()
        .unwrap();
    let result = assets
        .optimise(Method::MinimumVariance)
        .cash(0.1, 0.2)
        .build()
        .unwrap();
    let weights = result.results.weights.as_ref().unwrap();
    assert!((weights["Cash"] - 0.2).abs() < 1e-6, "{weights:?}");
    let means = assets.mean_returns();
    let expected: f64 = assets
        .labels()
        .iter()
        .zip(&means)
        .map(|(l, m)| weights.get(l).copied().unwrap_or(0.0) * m)
        .sum::<f64>()
        + weights["Cash"] * 0.02;
    assert!((result.results.expected_return.unwrap() - expected).abs() < 1e-9);
    // The history holds it as cash, and the report prints.
    assert!(result.report.history.is_some());
    assert!(result.to_string().contains("Cash"));
    // A backtest refits with it, from the cash each refit holds.
    let backtest = assets
        .backtest(Method::MinimumVariance)
        .cash(0.0, 0.5)
        .max_turnover(0.3)
        .walk_forward(Years(10))
        .build()
        .unwrap();
    let run = backtest.results.strategies[0]
        .out_of_sample
        .as_ref()
        .unwrap();
    let refits: Vec<_> = run.trades.iter().filter(|t| t.refit).collect();
    assert!(!refits.is_empty());
    assert!(
        refits
            .iter()
            .any(|t| t.weights.values().sum::<f64>() < 0.99)
    );
    assert!(refits.iter().all(|t| t.turnover <= 0.3 + 1e-6));
}

#[test]
fn cash_limits_are_checked_with_the_others() {
    let assets = with_bills();
    // The Sharpe ratio is the same with any cash, so the method takes no limits on it.
    let found = messages(
        assets
            .optimise(Method::MaximumSharpe)
            .cash(0.0, 0.2)
            .build(),
    );
    assert_eq!(found[0].0, "options.cash", "{found:?}");
    let found = messages(
        assets
            .optimise(Method::MinimumVariance)
            .cash(0.5, 0.2)
            .build(),
    );
    assert_eq!(found[0].0, "options.cash", "{found:?}");
    // The risk-free asset is limited through cash, not as an asset.
    let found = messages(
        assets
            .optimise(Method::MinimumVariance)
            .asset_bounds("T-Bills", 0.0, 0.2)
            .build(),
    );
    assert!(found[0].1.contains("limit it with cash"), "{found:?}");
    // With cash a choice, the weights held now may keep theirs without turning it over.
    let current = [
        ("US Equity", 0.3),
        ("Intl Equity", 0.1),
        ("Bonds", 0.1),
        ("Gold", 0.05),
        ("Real Estate", 0.05),
        ("T-Bills", 0.4),
    ];
    let kept = assets
        .optimise(Method::MinimumVariance)
        .current_weights(current)
        .max_turnover(0.1)
        .cash(0.0, 0.5)
        .build()
        .unwrap();
    let trades = kept.report.trades.as_ref().unwrap();
    assert!(trades.turnover <= 0.1 + 1e-6, "{trades:?}");
    assert_eq!(
        trades.trades.iter().filter(|t| t.cash).count(),
        usize::from(trades.trades.iter().any(|t| t.asset == "T-Bills")),
        "the cash is listed once"
    );
}
