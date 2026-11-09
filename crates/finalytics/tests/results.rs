//! The results of optimise, compare and backtest: each has the same shape, a report and the raw
//! results; the reports' figures agree with the results they describe; and every result prints
//! and lays out as a page.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::{Rebalance, Years};
use finalytics::risk::{StatisticsOptions, statistics};
use finalytics::{Assets, Error, Method, datasets};

mod common;

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
    let holdings = portfolio.assets().build().unwrap();
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
    // The frontier changes the report, and so its digest, not the weights.
    assert_eq!(result.results, without.results);
    assert_ne!(r.context.digest, without.report.context.digest);
    let mut context = r.context.clone();
    context.digest.clone_from(&without.report.context.digest);
    assert_eq!(context, without.report.context);
    let page = r.to_html(None);
    assert!(page.contains("The efficient frontier") && !page.contains(".with_frontier()"));
    // The option is left out of JSON when it is off, so options read as before.
    let options = serde_json::to_value(assets.optimise(Method::MaximumSharpe).options()).unwrap();
    assert!(options.get("frontier").is_none());
}

#[test]
fn an_optimisation_without_weights_is_not_a_target() {
    let portfolio = datasets::portfolio();
    // No mix of the holdings returns 90% a year.
    let failed = portfolio
        .assets()
        .build()
        .unwrap()
        .optimise(Method::TargetReturn)
        .target(0.9)
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
            "the optimisation found no weights: no portfolio meets every limit"
        )]
    );
    // The same through JSON, as the other languages give it.
    for json in [
        serde_json::to_string(&failed).unwrap(),
        serde_json::to_string(&failed.results).unwrap(),
    ] {
        let targets = finalytics::targets_from_json(&json).unwrap();
        assert!(portfolio.rebalance(targets).build().is_err());
    }
    let weights = finalytics::targets_from_json(r#"{"VTI": 0.6, "BND": 0.4}"#).unwrap();
    assert!(portfolio.rebalance(weights).build().is_ok());
}

#[test]
fn a_rebalance_has_a_report_and_raw_results() {
    let portfolio = datasets::portfolio();
    let target = portfolio
        .assets()
        .build()
        .unwrap()
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.4)
        .build()
        .unwrap();
    let result = portfolio
        .rebalance(&target)
        .whole_units()
        .fee_bps(5.0)
        .minimum_fee(1.0)
        .tax(0.37, 0.20)
        .name("Morgan family portfolio")
        .build()
        .unwrap();
    let r = &result.report;
    // Plain numbers become the exact decimals they are written as.
    let fees = &r.context.fees;
    assert_eq!(fees.rate.to_string(), "0.0005");
    assert_eq!(fees.minimum.to_string(), "1");
    let tax = r.context.tax.as_ref().unwrap();
    assert_eq!(
        (tax.short_term.to_string(), tax.long_term.to_string()),
        ("0.37".into(), "0.2".into())
    );
    assert_eq!(tax.long_term_after_days, 365);
    // Mistakes are reported at build, by the setting's name.
    let wrong = portfolio
        .rebalance(&target)
        .fee_bps(f64::NAN)
        .long_term_after_days(400)
        .build();
    let Err(Error::Validation(report)) = wrong else {
        panic!("built with a fee that is not a number");
    };
    let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["options.fee_bps", "options.long_term_after_days"]);
    assert_eq!(r.context.method.as_deref(), Some("Minimum variance"));
    assert_eq!(r.context.limits, ["5% to 40% in each asset"]);
    // The report's figures are the proposal's.
    let s = &r.summary;
    assert_eq!(s.trades, 10);
    close(s.turnover, result.results.turnover, "turnover");
    let held: f64 = r.holdings.iter().map(|h| h.after).sum();
    close(held, 1.0, "weights after");
    let target_total: f64 = r.holdings.iter().map(|h| h.target).sum();
    close(target_total, 1.0, "targets");
    let (before, after) = (s.before.as_ref().unwrap(), s.after.as_ref().unwrap());
    assert!(after.volatility < before.volatility);
    // The target's figures are the optimisation's: the same estimates.
    let at = s.target.as_ref().unwrap();
    close(
        at.volatility,
        target.results.volatility.unwrap(),
        "target volatility",
    );
    close(
        at.expected_return,
        target.results.expected_return.unwrap(),
        "target return",
    );
    assert!((after.volatility - at.volatility).abs() < 1e-3);
    // The retirement account is not taxable, and nothing goes short of cash.
    let [brokerage, ira] = &r.accounts[..] else {
        panic!("two accounts")
    };
    assert!(brokerage.taxable && brokerage.tax_base.unwrap() > 0.0);
    assert!(!ira.taxable && ira.tax_base.is_none() && ira.realised_gain_base > 0.0);
    assert_eq!(s.tax_base, brokerage.tax_base);
    let long = s.long_term_gain_base.unwrap();
    assert!(long > 0.0 && long <= s.realised_gain_base);
    assert!(r.accounts.iter().all(|a| a.warnings.is_empty()));
    // It prints, lays out as a page and exports, as JSON gives it too.
    let text = result.to_string();
    assert!(text.contains("FINALYTICS  ·  REBALANCE") && text.contains("not taxed"));
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    let raw = result.results.to_string();
    assert!(raw.starts_with("RebalanceResults · 2 accounts · 10 trades"));
    assert!(raw.lines().all(|l| l.chars().count() <= 80), "{raw}");
    let page = r.to_html(None);
    for anchor in ["r-summary", "r-trades", "r-costs", "r-method"] {
        assert!(page.contains(&format!(r#"id="{anchor}""#)), "{anchor}");
    }
    assert!(page.contains("<title>Morgan family portfolio</title>") && !page.contains("<script"));
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(finalytics::reports::report_text(&json, None).unwrap(), text);
    assert_eq!(finalytics::reports::report_html(&json, None).unwrap(), page);
    let tables = finalytics::reports::report_tables(&json).unwrap();
    assert_eq!(
        tables.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["holdings", "trades"]
    );
    let raw_json = serde_json::to_string(&result.results).unwrap();
    assert_eq!(finalytics::results_text(&raw_json).unwrap(), raw);
    // The trades apply to the portfolio, from the result or its raw results.
    let traded = portfolio.with_rebalance(&result).unwrap();
    assert_eq!(
        traded.weights().unwrap(),
        portfolio
            .with_rebalance(&result.results)
            .unwrap()
            .weights()
            .unwrap()
    );
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
        options.target_return = rf;
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
        // From 100 on the last date before the test, when the walk-forward strategy enters.
        assert_eq!(s.tested.growth.dates[1..], out.dates[..]);
        assert_eq!(s.tested.growth.values[0], 100.0);
        assert_eq!(s.tested.drawdown.values[0], 0.0);
        let entry = assets.dates()[assets.dates().partition_point(|d| *d < out.dates[0]) - 1];
        assert_eq!(s.tested.growth.dates[0], entry);
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
        // Each rolling window against the risk-free return over that window.
        let rolling = s.tested.rolling_sharpe.as_ref().unwrap();
        assert!(rolling.dates.len() > 200);
        assert_eq!(out.returns.len(), cash.len());
        let last = out
            .dates
            .iter()
            .position(|d| *d == rolling.dates[0])
            .unwrap();
        let window = last + 1 - 36..=last;
        let mut options = StatisticsOptions::default();
        options.periods_per_year = 12.0;
        options.risk_free_rate = cash[window.clone()]
            .iter()
            .map(|r| 1.0 + r)
            .product::<f64>()
            .powf(12.0 / 36.0)
            - 1.0;
        options.target_return = options.risk_free_rate;
        let expected = statistics(&out.returns[window], &options).unwrap();
        close(
            rolling.values[0],
            expected.sharpe_ratio.unwrap(),
            "rolling Sharpe ratio",
        );
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
    // From 100 on the date of the first prices.
    assert_eq!(
        history.strategy.growth.dates.len(),
        assets.dates().len() + 1
    );
    assert_eq!(history.strategy.growth.dates[0], assets.origin().unwrap());
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
    // 2016 runs across the split, so it is marked as both.
    let mixed = "† 2016 is in sample to Oct 2016 and out of sample after.";
    let text = report.to_string();
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(words.contains("2016†") && words.contains(mixed), "{text}");
    assert!(!words.contains("2017†"));
    let html = report.to_html(None);
    assert!(html.contains("2016†") && html.contains(mixed));
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
    let prices = common::sp500();
    let assets = Assets::builder()
        .prices(prices)
        .risk_free_asset("T-Bills")
        .build()
        .unwrap();
    let report = assets
        .backtest(Method::EqualWeight)
        .rebalance(Rebalance::Yearly)
        .benchmark(common::sp500_index())
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

#[test]
fn cash_at_a_risk_free_rate_compounds_to_that_rate() {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_rate(0.05)
        .build()
        .unwrap();
    let result = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .build()
        .unwrap();
    let per_month = 1.05f64.powf(1.0 / 12.0) - 1.0;
    assert!(
        result
            .results
            .cash
            .iter()
            .all(|c| (c - per_month).abs() < 1e-15)
    );
    close(
        result.report.strategy.tested.risk_free_rate,
        0.05,
        "risk-free rate",
    );
}

#[test]
fn fixed_weights_are_described_as_fixed() {
    let assets = assets();
    let result = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .walk_forward(Years(5))
        .build()
        .unwrap();
    let raw = &result.results.strategies[0];
    assert!(!raw.fitted);
    let s = &result.report.strategy;
    assert!(!s.fitted && s.in_sample.is_none());
    let text = result.report.to_string();
    let html = result.report.to_html(None);
    for page in [&text, &html] {
        assert!(!page.contains("refit"), "{page}");
        assert!(!page.contains("In sample"), "{page}");
        assert!(page.contains("not fitted to these returns"));
    }
    let method = assets
        .backtest(Method::MinimumVariance)
        .walk_forward(Years(5))
        .build()
        .unwrap();
    assert!(method.results.strategies[0].fitted);
    assert!(method.report.strategy.in_sample.is_some());
    assert!(
        method
            .report
            .to_string()
            .contains("Allocation at the last refit")
    );
}

#[test]
fn a_method_that_never_finds_weights_is_marked_as_holding_cash() {
    let assets = assets();
    // No mix of these assets reaches 50% a year.
    let report = assets
        .compare([Method::MinimumVariance, Method::TargetReturn])
        .target(0.5)
        .walk_forward(Years(5))
        .build()
        .unwrap()
        .report;
    let never = &report.strategies[1];
    assert!(!never.failures.is_empty());
    assert!(never.weights.iter().all(|w| w.weights.is_empty()));
    let text = report.to_string();
    assert!(text.contains("Target return (cash)"), "{text}");
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.contains(
            "Target return found no weights at 101 refits, from Oct 2001 to Sep 2026 \
             (infeasible), nor in its fit to the whole history (infeasible); it never held \
             anything but cash."
        ),
        "{text}"
    );
    let html = report.to_html(None);
    assert!(html.contains("Target return (cash)"));
    // The summary's best figures are of the method that invested.
    assert!(!html.contains("Smallest drawdown</div><div class=\"v\">0.0%"));
}

#[test]
fn a_portfolio_bought_and_held_has_no_rebalances_to_describe() {
    let report = assets()
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .rebalance(Rebalance::Never)
        .build()
        .unwrap()
        .report;
    assert_eq!(report.strategy.tested.rebalances, 0);
    let text = report.to_string();
    let html = report.to_html(None);
    for page in [&text, &html] {
        assert!(page.contains("Bought at the start and held"), "{page}");
        assert!(!page.contains("Largest rebalance"));
        assert!(!page.contains("at each rebalance they return"));
    }
}

#[test]
fn fixed_weights_are_compared_with_methods_on_the_same_periods() {
    let assets = assets();
    let result = assets
        .compare([Method::MinimumVariance])
        .weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])
        .weights("Bonds", [("Bonds", 1.0)])
        .walk_forward(Years(5))
        .benchmark([("US Equity", 1.0)])
        .benchmark_name("Stocks")
        .build()
        .unwrap();
    let names: Vec<&str> = result
        .report
        .strategies
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, ["Minimum variance", "60/40", "Bonds"]);
    let tested = &result.report.strategies[0].tested;
    for s in &result.report.strategies[1..] {
        assert!(!s.fitted && s.in_sample.is_none());
        assert_eq!((s.tested.start, s.tested.end), (tested.start, tested.end));
    }
    assert_eq!(result.report.benchmark.as_ref().unwrap().name, "Stocks");
    let text = result.report.to_string();
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.contains("Fixed weights, not fitted, are scored on the same periods."),
        "{text}"
    );
    assert!(matches!(
        assets
            .compare([Method::MinimumVariance])
            .benchmark_name("x")
            .build(),
        Err(Error::Validation(_))
    ));
}

#[test]
fn the_minimum_cvar_report_shows_its_shortfall_and_its_groups() {
    let assets = assets();
    let result = assets
        .optimise(Method::MinimumCvar)
        .confidence(0.9)
        .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
        .build()
        .unwrap();
    let (report, raw) = (&result.report, &result.results);
    let es = report.summary.as_ref().unwrap().expected_shortfall.unwrap();
    assert_eq!(es.confidence, 0.9);
    close(
        es.value,
        raw.solution.objective_value.unwrap(),
        "the optimal CVaR",
    );
    // The shares of risk are of the expected shortfall, and add up to one.
    let shares: f64 = report.allocation.iter().map(|a| a.risk_share).sum();
    assert!((shares - 1.0).abs() < 1e-9, "{shares}");
    let [group] = &report.groups[..] else {
        panic!("{:?}", report.groups)
    };
    let weights = raw.weights.as_ref().unwrap();
    close(
        group.weight,
        weights["US Equity"] + weights["Intl Equity"],
        "group weight",
    );
    assert!(group.weight >= 0.3 - 1e-6 && group.weight <= 0.6 + 1e-6);
    let text = report.to_string();
    assert!(
        text.contains("CVaR 90%") && text.contains("Groups"),
        "{text}"
    );
    let html = report.to_html(None);
    assert!(html.contains("CVaR 90%") && html.contains("expected shortfall at 90%"));
    // Other methods keep the variance's shares, with no shortfall.
    let variance = assets.optimise(Method::MinimumVariance).build().unwrap();
    assert!(
        variance
            .report
            .summary
            .unwrap()
            .expected_shortfall
            .is_none()
    );
}
