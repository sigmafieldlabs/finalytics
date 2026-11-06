//! How the reports describe what they show: fixed weights are not "in sample", the highlights
//! say what they rank, names keep their case, long lists of limits are counted in the header,
//! and the downside methods show the optimum they reach.

#![allow(clippy::unwrap_used)]

use finalytics::{Assets, Method, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

fn fits_eighty_columns(text: &str) {
    for line in text.lines() {
        assert!(line.chars().count() <= 80, "{line}");
    }
}

#[test]
fn fixed_weights_on_the_prices_given_are_not_called_in_sample() {
    let assets = assets();
    let result = assets
        .compare([])
        .weights("Min-CVaR", [("US Equity", 0.3), ("Bonds", 0.7)])
        .weights("Growth", [("US Equity", 0.8), ("Gold", 0.2)])
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .benchmark_name("60/40")
        .build()
        .unwrap();
    let text = result.report.to_text(None);
    fits_eighty_columns(&text);
    let html = result.report.to_html(None);
    for page in [&text, &html] {
        assert!(!page.to_lowercase().contains("in sample"), "{page}");
        assert!(page.contains("Fixed weights"), "{page}");
        assert!(page.contains("2 portfolios"), "{page}");
    }
    assert!(html.contains("Each portfolio, held over"), "{html}");
    // The highlights rank the portfolios, and say so beside the benchmark.
    assert!(
        html.contains("the highlights rank the portfolios"),
        "{html}"
    );
    assert!(
        html.contains("Bold marks the best of the portfolios in each column; 60/40, the benchmark, is not ranked."),
        "{html}"
    );
    // The names keep their case wherever they appear.
    assert!(!html.contains("min-CVaR") && !text.contains("min-CVaR"));

    // A backtest of fixed weights alone reads the same way.
    let backtest = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .name("A long name for a portfolio")
        .benchmark([("US Equity", 1.0)])
        .benchmark_name("US equity only")
        .build()
        .unwrap();
    let text = backtest.report.to_text(None);
    fits_eighty_columns(&text);
    assert!(!text.to_lowercase().contains("in sample"), "{text}");
    // Names that fit the columns are printed whole.
    assert!(text.contains("A long name for a portfolio"), "{text}");
    assert!(
        !backtest
            .report
            .to_html(None)
            .to_lowercase()
            .contains("in sample")
    );
}

#[test]
fn methods_beside_fixed_weights_are_strategies_scored_in_sample() {
    let result = assets()
        .compare([Method::MinimumVariance])
        .weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])
        .build()
        .unwrap();
    let text = result.report.to_text(None);
    assert!(text.contains("In sample"), "{text}");
    assert!(text.contains("2 strategies"), "{text}");
}

#[test]
fn many_group_limits_are_counted_in_the_header_and_listed_under_method() {
    let assets = assets();
    let result = assets
        .optimise(Method::MinimumVariance)
        .group("Equity", &["US Equity", "Intl Equity"], 0.0, 0.5)
        .group("Defensive", &["Bonds", "Gold"], 0.0, 0.6)
        .group("Property", &["Real Estate"], 0.0, 0.05)
        .build()
        .unwrap();
    let text = result.report.to_text(None);
    fits_eighty_columns(&text);
    let header: String = text.lines().take(6).collect::<Vec<_>>().join("\n");
    assert!(header.contains("3 group limits"), "{header}");
    assert!(!header.contains("together"), "{header}");
    // The Method section lists each group, wrapped to the page.
    let method = text
        .split("\n  Limits ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        method.contains("Equity (US Equity, Intl Equity): 0% to 50% together"),
        "{text}"
    );
    // Two groups are short enough to list in full.
    let two = assets
        .optimise(Method::MinimumVariance)
        .group("Equity", &["US Equity", "Intl Equity"], 0.0, 0.5)
        .group("Defensive", &["Bonds", "Gold"], 0.0, 0.6)
        .build()
        .unwrap();
    let text = two.report.to_text(None);
    let header: String = text.lines().take(6).collect::<Vec<_>>().join("\n");
    assert!(!header.contains("group limits"), "{header}");
}

#[test]
fn the_downside_methods_show_the_optimum_they_reach() {
    let assets = assets();
    let semi = assets
        .optimise(Method::MinimumSemivariance)
        .build()
        .unwrap();
    let optimum = semi.results.solution.objective_value.unwrap();
    let summary = semi.report.summary.as_ref().unwrap();
    assert_eq!(summary.downside_deviation, Some(optimum.sqrt()));
    assert!(summary.drawdown_at_risk.is_none() && summary.expected_shortfall.is_none());
    let text = semi.report.to_text(None);
    assert!(text.contains("Downside deviation"), "{text}");
    assert!(semi.report.to_html(None).contains("Downside deviation"));

    let cdar = assets
        .optimise(Method::MinimumCdar)
        .confidence(0.9)
        .build()
        .unwrap();
    let figure = cdar
        .report
        .summary
        .as_ref()
        .unwrap()
        .drawdown_at_risk
        .unwrap();
    assert_eq!(figure.confidence, 0.9);
    assert_eq!(Some(figure.value), cdar.results.solution.objective_value);
    let text = cdar.report.to_text(None);
    assert!(text.contains("CDaR 90%"), "{text}");
    assert!(
        cdar.report
            .to_html(None)
            .contains("deepest 10% of the drawdowns")
    );
}

#[test]
fn reports_of_fixed_weights_speak_of_portfolios_throughout() {
    let assets = assets();
    let result = assets
        .compare([])
        .weights("Cautious", [("US Equity", 0.3), ("Bonds", 0.7)])
        .weights("Growth", [("US Equity", 0.8), ("Gold", 0.2)])
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .build()
        .unwrap();
    let text = result.report.to_text(None);
    let html = result.report.to_html(None);
    assert!(text.contains("How the portfolios were tested"), "{text}");
    assert!(text.contains("  Portfolio "), "{text}");
    assert!(html.contains("How far and how long each portfolio fell"));
    assert!(html.contains("What each portfolio held"));
    assert!(!html.contains("How the methods were tested"));
    // The default benchmark name is not repeated.
    assert!(html.contains("the benchmark is not ranked"), "{html}");
}

#[test]
fn one_strategy_beside_a_benchmark_is_not_ranked() {
    let result = assets()
        .compare([])
        .weights("Bonds only", [("Bonds", 1.0)])
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .build()
        .unwrap();
    let html = result.report.to_html(None);
    assert!(
        !html.contains("Highest") && !html.contains("Smallest"),
        "{html}"
    );
    assert!(html.contains("Sharpe ratio") && html.contains("Maximum drawdown"));
}

#[test]
fn a_holdout_of_fixed_weights_is_split_before_and_from_the_test() {
    let backtest = assets()
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .holdout_from("2010-01-01")
        .build()
        .unwrap();
    let html = backtest.report.to_html(None);
    assert!(!html.to_lowercase().contains("in sample"), "{html}");
    assert!(
        html.contains("Before Jan 2010") && html.contains("From Jan 2010"),
        "{html}"
    );
    assert!(
        !backtest
            .report
            .to_text(None)
            .to_lowercase()
            .contains("in sample")
    );
}
