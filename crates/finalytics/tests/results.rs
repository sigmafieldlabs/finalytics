//! The results of optimise, compare and backtest: each has the same shape, a report and the raw
//! results; the reports' figures agree with the results they describe; and every result prints
//! and lays out as a page.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::{Rebalance, Years};
use finalytics::risk::{StatisticsOptions, statistics};
use finalytics::{Assets, Error, Method, datasets};

fn assets() -> Assets {
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

#[test]
fn every_result_has_a_report_and_raw_results() {
    let assets = assets();
    let optimisation = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .build()
        .unwrap();
    let comparison = assets
        .compare([Method::EqualWeight, Method::MinimumVariance])
        .walk_forward(Years(5))
        .build()
        .unwrap();
    let backtest = assets.backtest(Method::MinimumVariance).build().unwrap();
    for (text, page, report_text) in [
        (
            optimisation.to_string(),
            optimisation.report.to_html(None),
            optimisation.report.to_string(),
        ),
        (
            comparison.to_string(),
            comparison.report.to_html(None),
            comparison.report.to_string(),
        ),
        (
            backtest.to_string(),
            backtest.report.to_html(None),
            backtest.report.to_string(),
        ),
    ] {
        assert_eq!(text, report_text, "printing a result prints its report");
        assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
        assert!(page.starts_with("<!doctype html>") && !page.contains("<script"));
    }
    // The raw results print readably too.
    let weights = optimisation.results.to_string();
    assert!(
        weights.starts_with("OptimisationResult · Maximum Sharpe · optimal"),
        "{weights}"
    );
    let runs = comparison.results.to_string();
    assert!(
        runs.starts_with("BacktestResults · walk-forward on 5 years · 2 strategies"),
        "{runs}"
    );
    assert!(runs.lines().all(|l| l.chars().count() <= 80), "{runs}");
    assert!(backtest.results.to_string().contains("Minimum variance"));
}

#[test]
fn the_optimisation_report_explains_the_weights() {
    let assets = assets();
    let result = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .build()
        .unwrap();
    let r = &result.report;
    let weights = result.results.weights.as_ref().unwrap();
    let s = r.summary.as_ref().unwrap();
    // The summary matches the solver's figures.
    close(
        s.expected_return,
        result.results.expected_return.unwrap(),
        "expected return",
    );
    assert!((s.volatility - result.results.volatility.unwrap()).abs() < 1e-9);
    assert!((s.sharpe_ratio.unwrap() - result.results.sharpe_ratio.unwrap()).abs() < 1e-9);
    // Every asset, largest weight first; the shares of risk add up to one.
    assert_eq!(r.allocation.len(), 5);
    assert!(r.allocation.windows(2).all(|p| p[0].weight >= p[1].weight));
    for a in &r.allocation {
        close(a.weight, weights[&a.asset], &a.asset);
    }
    let shares: f64 = r.allocation.iter().map(|a| a.risk_share).sum();
    close(shares, 1.0, "risk shares");
    assert_eq!(s.largest_asset, r.allocation[0].asset);
    // Equal weight and the history are there; the frontier only when asked for.
    assert!(r.frontier.is_empty());
    assert!(r.equal_weight.is_some());
    let h = r.history.as_ref().unwrap();
    assert_eq!(h.weights.statistics.observations, assets.dates().len());
    assert_eq!(r.context.method, "Maximum Sharpe");
    assert_eq!(r.context.limits, ["At most 40% in any asset"]);
    assert_eq!(r.correlation.rows.len(), 5);
    let page = r.to_html(Some("Six asset classes"));
    for anchor in [
        "p-summary",
        "p-risk",
        "p-risk-return",
        "p-history",
        "p-method",
    ] {
        assert!(page.contains(&format!(r#"id="{anchor}""#)), "{anchor}");
    }
    assert!(page.contains("Optimisation report"));
    assert!(page.contains(".with_frontier() in Rust") && !page.contains("The efficient frontier"));
    // Through JSON, as the other languages call it.
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(
        finalytics::reports::report_html(&json, None).unwrap(),
        r.to_html(None)
    );
    assert_eq!(
        finalytics::reports::report_text(&json, None).unwrap(),
        r.to_string()
    );
    assert_eq!(
        finalytics::reports::report_tables(&json).unwrap(),
        r.tables()
    );
    // Rebalancing takes the whole result.
    let portfolio = datasets::portfolio();
    let holdings = portfolio.assets().unwrap();
    let target = holdings.optimise(Method::MinimumVariance).build().unwrap();
    assert!(portfolio.rebalance(&target).build().is_ok());
}

#[test]
fn the_frontier_is_drawn_when_asked_for() {
    let assets = assets();
    let without = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .build()
        .unwrap();
    let result = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .with_frontier()
        .build()
        .unwrap();
    let r = &result.report;
    let s = r.summary.as_ref().unwrap();
    assert!(r.frontier.len() > 10);
    // No point on the frontier does better than the weights at the same risk.
    assert!(
        r.frontier
            .iter()
            .all(|p| p.volatility >= s.volatility - 1e-3
                || p.expected_return <= s.expected_return + 1e-9)
    );
    // The frontier changes the report, not the weights.
    assert_eq!(result.results, without.results);
    assert_eq!(r.context, without.report.context);
    let page = r.to_html(None);
    assert!(page.contains("The efficient frontier") && !page.contains(".with_frontier()"));
    // The option is left out of JSON when it is off, so options read as before.
    let options = serde_json::to_value(assets.optimise(Method::MaximumSharpe).options()).unwrap();
    assert!(options.get("frontier").is_none());
}

#[test]
fn an_optimisation_without_weights_is_not_a_target() {
    let portfolio = datasets::portfolio();
    let failed = portfolio
        .assets()
        .unwrap()
        .optimise(Method::MinimumVariance)
        .max_weight(0.1)
        .build()
        .unwrap();
    assert!(failed.results.weights.is_none());
    let Err(Error::Validation(report)) = portfolio.rebalance(&failed).build() else {
        panic!("rebalanced to an optimisation without weights");
    };
    let issues: Vec<_> = report
        .issues()
        .iter()
        .map(|i| (i.path.as_str(), i.message.as_str()))
        .collect();
    assert_eq!(
        issues,
        [(
            "targets",
            "the optimisation ended infeasible and found no weights"
        )]
    );
    // The same through JSON, as the other languages give it.
    let json = serde_json::to_string(&failed).unwrap();
    assert!(finalytics::targets_from_json(&json).is_err());
    let json = serde_json::to_string(&failed.results).unwrap();
    assert!(finalytics::targets_from_json(&json).is_err());
    let weights = r#"{"VTI": 0.6, "BND": 0.4}"#;
    assert_eq!(finalytics::targets_from_json(weights).unwrap().len(), 2);
}

#[test]
fn the_comparison_report_scores_each_method_on_its_test() {
    let assets = assets();
    let compare = assets
        .compare([
            Method::EqualWeight,
            Method::MinimumVariance,
            Method::RiskParity,
        ])
        .max_weight(0.4)
        .rebalance(Rebalance::Quarterly)
        .walk_forward(Years(5))
        .trading_cost_bps(10.0)
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .name("Six asset classes");
    let result = compare.build().unwrap();
    let (report, backtest) = (&result.report, &result.results);
    assert_eq!(report.name.as_deref(), Some("Six asset classes"));
    let c = &report.context;
    assert_eq!(c.window, Some(60));
    assert_eq!(c.test_start, assets.dates()[60]);
    assert_eq!(c.limits, ["At most 40% in any asset"]);
    assert_eq!(c.risk_free_asset.as_deref(), Some("T-Bills"));
    assert_eq!(c.digest.len(), 64);
    let cash = &backtest.cash[60..];
    let rf = cash
        .iter()
        .map(|r| 1.0 + r)
        .product::<f64>()
        .powf(12.0 / cash.len() as f64)
        - 1.0;
    close(c.risk_free_rate, rf, "risk-free rate");
    for (s, raw) in report.strategies.iter().zip(&backtest.strategies) {
        let out = raw.out_of_sample.as_ref().unwrap();
        let mut options = StatisticsOptions::default();
        options.periods_per_year = 12.0;
        options.risk_free_rate = rf;
        let expected = statistics(&out.returns, &options).unwrap();
        assert_eq!(s.tested.statistics, expected, "{}", s.name);
        close(
            s.tested.turnover_per_year,
            out.turnover_per_year(12.0),
            "turnover",
        );
        close(s.tested.costs_per_year, out.costs_per_year(12.0), "costs");
        assert_eq!(s.tested.rebalances, out.trades.len());
        close(
            *s.tested.growth.values.last().unwrap(),
            100.0 * out.growth(),
            "growth",
        );
        assert_eq!(s.tested.growth.dates, out.dates);
        let worst = s.tested.drawdown.values.iter().copied().fold(0.0, f64::min);
        close(worst, s.tested.statistics.max_drawdown, "drawdown");
        // The years and the months compound to the whole.
        let years: f64 = s.tested.years.iter().map(|y| 1.0 + y.value).product();
        let months: f64 = s
            .tested
            .months
            .values
            .iter()
            .flatten()
            .flatten()
            .map(|r| 1.0 + r)
            .product();
        close(years, out.growth(), "years");
        close(months, out.growth(), "months");
        // Against the benchmark, over the same periods; in sample alike.
        assert!(s.tested.relative.is_some());
        let i = s.in_sample.as_ref().unwrap();
        assert_eq!(i.start, s.tested.start);
        assert!(i.relative.is_some());
        // The weights at the start, then at each refit.
        assert_eq!(s.weights[0].weights, out.start);
        assert_eq!(
            s.weights.len(),
            1 + out.trades.iter().filter(|t| t.refit).count()
        );
        assert_eq!(s.weights[0].date, assets.dates()[59]);
        assert!(s.tested.rolling_sharpe.as_ref().unwrap().dates.len() > 200);
    }
    let note = &report.strategies[2].notes;
    assert_eq!(
        note,
        &["Fully invested and long only; the limits on weights do not apply."]
    );
    let benchmark = report.benchmark.as_ref().unwrap();
    assert_eq!(benchmark.name, "Benchmark");
    assert!(benchmark.tested.relative.is_none());
}

#[test]
fn the_backtest_report_shows_the_whole_history_of_a_holdout() {
    let assets = assets();
    let builder = assets
        .backtest(Method::MaximumSharpe)
        .max_weight(0.4)
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .holdout_from("2016-11-01");
    let result = builder.build().unwrap();
    let (report, backtest) = (&result.report, &result.results);
    let raw = &backtest.strategies[0];
    let (ins, out) = (&raw.in_sample, raw.out_of_sample.as_ref().unwrap());
    let history = report.history.as_ref().unwrap();
    assert_eq!(history.split, out.dates[0]);
    assert_eq!(report.context.fit_end, Some(*ins.dates.last().unwrap()));
    close(
        *history.strategy.growth.values.last().unwrap(),
        100.0 * ins.growth() * out.growth(),
        "the history chains in and out of sample",
    );
    assert_eq!(history.strategy.growth.dates.len(), assets.dates().len());
    assert!(history.benchmark.is_some());
    let s = &report.strategy;
    assert_eq!(s.tested.start, out.dates[0]);
    assert_eq!(
        s.in_sample.as_ref().unwrap().end,
        *ins.dates.last().unwrap()
    );
    // The weights held drift between trades and return to the targets at each.
    let total: Vec<f64> = (0..report.held[0].values.len())
        .map(|i| report.held.iter().map(|h| h.values[i]).sum())
        .collect();
    assert!(total.iter().all(|t| (t - 1.0).abs() < 1e-9));
    let first = &report
        .held
        .iter()
        .find(|h| h.name == "US Equity")
        .unwrap()
        .values;
    close(first[0], out.start["US Equity"], "the weight bought");
    assert!((first[1] - first[0]).abs() > 1e-6, "weights drift");
    assert!(report.held.len() <= 9);
}

#[test]
fn reports_print_and_lay_out_as_pages() {
    let assets = assets();
    let report = assets
        .compare([
            Method::EqualWeight,
            Method::MinimumVariance,
            Method::MaximumSharpe,
        ])
        .bounds(0.0, 0.5)
        .walk_forward(Years(5))
        .build()
        .unwrap()
        .report;
    let text = report.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(text.contains("FINALYTICS  ·  COMPARISON"));
    assert!(text.contains("Maximum Sharpe found no weights"));
    let html = report.to_html(Some("Asset classes"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Asset classes</title>"));
    for anchor in [
        "c-summary",
        "c-performance",
        "c-risk",
        "c-robustness",
        "c-weights",
        "c-method",
    ] {
        assert!(html.contains(&format!(r#"id="{anchor}""#)), "{anchor}");
    }
    assert!(!html.contains("<script"));
    let tables = report.tables();
    assert_eq!(
        tables.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["statistics", "calendar_years", "weights"]
    );
    assert_eq!(tables[0].rows.len(), 6, "each method in and out of sample");

    let fixed = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .name("60/40")
        .rebalance(Rebalance::drift(0.05))
        .build()
        .unwrap()
        .report;
    assert!(fixed.history.is_none() && fixed.strategy.in_sample.is_none());
    let text = fixed.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(text.contains("60/40"));
    let html = fixed.to_html(None);
    assert!(html.contains("<title>60/40</title>") && html.contains("Backtest report"));
    // Through JSON, as the other languages call them.
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(
        finalytics::reports::report_html(&json, None).unwrap(),
        report.to_html(None)
    );
    assert_eq!(
        finalytics::reports::report_text(&json, None).unwrap(),
        report.to_string()
    );
    let json = serde_json::to_string(&fixed).unwrap();
    assert_eq!(
        finalytics::reports::report_tables(&json).unwrap(),
        fixed.tables()
    );
}

#[test]
fn daily_returns_are_charted_by_month() {
    let prices = datasets::sp500();
    let assets = Assets::builder()
        .prices(prices)
        .risk_free_asset("T-Bills")
        .build()
        .unwrap();
    let report = assets
        .backtest(Method::EqualWeight)
        .rebalance(Rebalance::Yearly)
        .benchmark(datasets::sp500_index())
        .build()
        .unwrap()
        .report;
    let s = &report.strategy.tested;
    assert_eq!(s.statistics.observations, assets.dates().len());
    // One point a month: about 262 from January 2005 to October 2026.
    assert!(
        (260..=264).contains(&s.growth.dates.len()),
        "{}",
        s.growth.dates.len()
    );
    assert_eq!(s.months.rows.first().map(String::as_str), Some("2005"));
    assert_eq!(report.benchmark.as_ref().unwrap().name, "S&P 500");
    // 55 stocks: the largest seven and the rest together, with no cash.
    assert_eq!(report.held.len(), 8);
    assert_eq!(report.held[7].name, "48 others");
    assert!(report.strategy.tested.relative.is_some());
}
