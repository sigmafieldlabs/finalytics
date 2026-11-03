//! The optimisation and backtest reports: their figures agree with the backtest they describe,
//! and they print and lay out as pages.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::{Rebalance, Years};
use finalytics::risk::{StatisticsOptions, statistics};
use finalytics::{Assets, Method, datasets};

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
fn the_optimisation_report_scores_each_method_on_its_test() {
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
    let backtest = compare.simulate().unwrap();
    let report = compare.build().unwrap();
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
    let backtest = builder.simulate().unwrap();
    let report = builder.build().unwrap();
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
        .unwrap();
    let text = report.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(text.contains("FINALYTICS  ·  OPTIMISATION"));
    assert!(text.contains("Maximum Sharpe found no weights"));
    let html = report.to_html(Some("Asset classes"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Asset classes</title>"));
    for anchor in [
        "o-summary",
        "o-performance",
        "o-risk",
        "o-robustness",
        "o-weights",
        "o-method",
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
        .unwrap();
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
        .unwrap();
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
