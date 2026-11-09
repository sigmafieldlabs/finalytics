//! Given weights valued on the assets' estimates: the same figures an optimisation gives its own
//! weights, a portfolio's current mix valued as the overview report values it, and the report.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::{Assets, Error, Method, OptimiseOptions, datasets};

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

fn issues(result: Result<impl std::fmt::Debug, Error>) -> Vec<(String, String)> {
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
fn a_portfolios_own_weights_are_the_overviews_current_point() {
    let portfolio = datasets::portfolio();
    let valued = portfolio
        .assets()
        .unwrap()
        .evaluate(portfolio.weights().unwrap())
        .build()
        .unwrap();
    let current = portfolio
        .report()
        .build()
        .unwrap()
        .overview
        .current
        .unwrap();
    close(valued.results.volatility, current.volatility, "volatility");
    close(
        valued.results.expected_return,
        current.expected_return,
        "expected return",
    );
    let shares: f64 = valued.results.risk_shares.values().sum();
    close(shares, 1.0, "shares of risk");
    // Base-currency cash is riskless; cash in other currencies carries its rate's risk.
    assert_eq!(valued.results.risk_shares["USD cash"], 0.0);
    assert!(valued.results.risk_shares["EUR cash"] != 0.0);
    // The report lists the cash with the instruments, but counts only the instruments as
    // assets, and has no solver.
    let r = &valued.report;
    assert!(r.context.given);
    assert_eq!(r.context.assets, ["BND", "EXSA", "GLD", "VNQ", "VTI"]);
    assert!(r.allocation.iter().any(|a| a.asset == "EUR cash"));
    let text = valued.to_string();
    assert!(text.contains("Given weights"), "{text}");
    assert!(text.contains("How the weights were valued"), "{text}");
    assert!(!text.contains("Solver"), "{text}");
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    let page = r.to_html(Some("Current portfolio"));
    assert!(page.starts_with("<!doctype html>") && page.contains("Current portfolio"));
    // The history holds the euro cash as the asset it is.
    assert!(r.history.is_some());
}

#[test]
fn an_optimisations_weights_have_the_optimisations_figures() {
    let assets = with_bills();
    let optimised = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .build()
        .unwrap();
    let weights = optimised.results.weights.clone().unwrap();
    let valued = assets.evaluate(weights.clone()).build().unwrap();
    let r = &valued.results;
    let o = &optimised.results;
    assert!((r.expected_return - o.expected_return.unwrap()).abs() < 1e-9);
    assert!((r.volatility - o.volatility.unwrap()).abs() < 1e-9);
    assert!((r.sharpe_ratio.unwrap() - o.sharpe_ratio.unwrap()).abs() < 1e-8);
    close(r.risk_free_rate, o.risk_free_rate, "risk-free rate");
    assert_eq!(r.weights, weights);
    // The report's figures and shares of risk are the results'.
    let summary = valued.report.summary.as_ref().unwrap();
    close(summary.volatility, r.volatility, "report volatility");
    for row in &valued.report.allocation {
        close(row.risk_share, r.risk_shares[&row.asset], &row.asset);
    }
    // Printed, the raw figures list the weights.
    let raw = r.to_string();
    assert!(
        raw.contains("Evaluation · Given weights") && raw.contains("US Equity"),
        "{raw}"
    );
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(finalytics::results_text(&json).unwrap(), raw);
}

#[test]
fn the_frontier_is_drawn_under_its_limits_with_cash_where_asked() {
    let assets = with_bills();
    let weights = [
        ("US Equity", 0.4),
        ("Intl Equity", 0.1),
        ("Bonds", 0.3),
        ("T-Bills", 0.2),
    ];
    let valued = assets
        .evaluate(weights)
        .max_weight(0.5)
        .cash(0.0, 0.3)
        .with_frontier()
        .name("Sixty forty")
        .build()
        .unwrap();
    let r = &valued.report;
    assert_eq!(r.frontier.len(), 30);
    assert_eq!(
        r.context.limits,
        ["At most 50% in any asset", "0% to 30% in cash"]
    );
    assert_eq!(r.name.as_deref(), Some("Sixty forty"));
    let text = valued.to_string();
    assert!(text.contains("frontier at most 50% in any asset"), "{text}");
    // Riskless, the T-bills add their return and no risk.
    let means = assets.mean_returns();
    let labels = assets.labels();
    let mean = |a: &str| means[labels.iter().position(|l| l == a).unwrap()];
    let expected: f64 = weights.iter().map(|(a, w)| w * mean(a)).sum();
    close(valued.results.expected_return, expected, "expected return");
    assert_eq!(valued.results.risk_shares["T-Bills"], 0.0);
    // Without a risk-free asset, Cash earns the risk-free rate.
    let plain = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_rate(0.02)
        .build()
        .unwrap();
    let half = plain
        .evaluate([("US Equity", 0.5), ("Cash", 0.5)])
        .build()
        .unwrap();
    let us = plain.mean_returns()[0];
    close(half.results.expected_return, 0.5 * us + 0.01, "with cash");
    let sd = plain.covariance().unwrap().matrix.get(0, 0).sqrt();
    close(half.results.volatility, 0.5 * sd, "half the volatility");
}

#[test]
fn every_problem_with_the_weights_and_options_is_reported_together() {
    let assets = with_bills();
    let found = issues(
        assets
            .evaluate([("US Equity", 0.6), ("Gold Miners", 0.2), ("Bonds", 0.1)])
            .max_weight(0.4)
            .build(),
    );
    let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
    // The total is checked too, as for every input of weights.
    assert_eq!(
        paths,
        ["options.max_weight", "weights.Gold Miners", "weights"],
        "{found:?}"
    );
    let short = issues(
        assets
            .evaluate([("US Equity", 0.6), ("Bonds", 0.3)])
            .build(),
    );
    assert_eq!(short[0].0, "weights");
    assert!(
        short[0]
            .1
            .contains("give T-Bills, the risk-free asset, any cash"),
        "{short:?}"
    );
    // An option that valuing weights does not use is refused, by name.
    let mut options = OptimiseOptions::default();
    options.target = Some(0.05);
    let weights: BTreeMap<String, f64> = [("Bonds".to_owned(), 1.0)].into();
    let found = issues(assets.evaluate_with(&weights, &options));
    assert_eq!(
        found,
        [(
            "options.target".to_owned(),
            "valuing given weights does not use target".to_owned()
        )]
    );
    let empty = issues(assets.evaluate(BTreeMap::<String, f64>::new()).build());
    assert_eq!(empty[0].0, "weights");
}
