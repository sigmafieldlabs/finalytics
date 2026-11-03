//! The engine's bookkeeping, on small markets worked by hand.

#![allow(clippy::unwrap_used, clippy::single_range_in_vec_init)]

use std::cell::RefCell;
use std::ops::Range;

use finalytics_backtest::*;
use finalytics_core::{Date, Error};
use finalytics_risk::ReturnsTable;

/// Month ends from January 2025, `n` of them.
fn month_ends(n: usize) -> Vec<Date> {
    let ends = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    (0..n)
        .map(|i| {
            let (year, month) = (2025 + i as i32 / 12, i % 12);
            format!("{year}-{:02}-{:02}", month + 1, ends[month])
                .parse()
                .unwrap()
        })
        .collect()
}

/// Two assets over twelve months.
fn market_data() -> (Vec<Date>, ReturnsTable, Vec<f64>) {
    let a = [
        0.05, -0.02, 0.03, 0.04, -0.06, 0.02, 0.01, 0.03, -0.01, 0.02, 0.04, -0.03,
    ];
    let b = [
        0.00, 0.01, 0.00, -0.01, 0.02, 0.01, 0.00, -0.01, 0.01, 0.00, -0.02, 0.01,
    ];
    let returns =
        ReturnsTable::new(vec!["A".into(), "B".into()], vec![a.to_vec(), b.to_vec()]).unwrap();
    (month_ends(12), returns, vec![0.002; 12])
}

fn paths(error: Error) -> Vec<String> {
    match error {
        Error::Validation(report) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, found {other}"),
    }
}

fn fixed(name: &str, weights: &[f64]) -> Strategy<'static> {
    Strategy {
        name: name.into(),
        plan: Plan::Weights(weights.to_vec()),
        notes: vec![],
    }
}

/// The reference: weights drift with returns; at the end of each listed period they are reset
/// to the target, and the next period's return pays the cost.
fn by_hand(
    returns: &ReturnsTable,
    cash: &[f64],
    rows: Range<usize>,
    target: &[f64],
    trade_after: &[usize],
    cost: f64,
) -> Vec<f64> {
    let mut held = target.to_vec();
    let mut held_cash = 1.0 - target.iter().sum::<f64>();
    let mut pending = 0.0;
    let mut out = vec![];
    for t in rows {
        let r: Vec<f64> = returns.columns().iter().map(|c| c[t]).collect();
        let gross: f64 = held.iter().zip(&r).map(|(w, r)| w * r).sum::<f64>() + held_cash * cash[t];
        out.push((1.0 + gross) * (1.0 - pending) - 1.0);
        pending = 0.0;
        for (w, r) in held.iter_mut().zip(&r) {
            *w *= (1.0 + r) / (1.0 + gross);
        }
        held_cash *= (1.0 + cash[t]) / (1.0 + gross);
        if trade_after.contains(&t) {
            let traded: f64 = held.iter().zip(target).map(|(h, w)| (h - w).abs()).sum();
            pending = cost * traded;
            held = target.to_vec();
            held_cash = 1.0 - target.iter().sum::<f64>();
        }
    }
    out
}

fn close(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-14, "{x} against {y}");
    }
}

#[test]
fn fixed_weights_drift_and_trade_back_on_schedule_with_costs() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let quarterly = Settings::new(Validation::InSample).with_trading_cost_bps(10.0);
    let result = backtest(&market, vec![fixed("70/30", &[0.7, 0.3])], None, &quarterly).unwrap();
    let run = &result.strategies[0].in_sample;
    // Quarter ends: March, June and September; nothing trades after the last period.
    let ends = [2, 5, 8];
    close(
        &run.returns,
        &by_hand(&returns, &cash, 0..12, &[0.7, 0.3], &ends, 0.001),
    );
    let traded: Vec<String> = run.trades.iter().map(|t| t.date.to_string()).collect();
    assert_eq!(traded, ["2025-03-31", "2025-06-30", "2025-09-30"]);
    for trade in &run.trades {
        assert!((trade.cost - 0.001 * 2.0 * trade.turnover).abs() < 1e-15);
        assert_eq!(trade.weights["A"], 0.7);
        assert!(!trade.refit);
    }
    assert_eq!(run.start["B"], 0.3);
    assert!(result.strategies[0].out_of_sample.is_none());

    // Monthly trades after every period but the last; never trades not at all.
    let monthly = quarterly.clone().with_rebalance(Rebalance::Monthly);
    let run = &backtest(&market, vec![fixed("70/30", &[0.7, 0.3])], None, &monthly)
        .unwrap()
        .strategies[0]
        .in_sample;
    close(
        &run.returns,
        &by_hand(
            &returns,
            &cash,
            0..12,
            &[0.7, 0.3],
            &(0..11).collect::<Vec<_>>(),
            0.001,
        ),
    );
    let never = quarterly.clone().with_rebalance(Rebalance::Never);
    let run = &backtest(&market, vec![fixed("70/30", &[0.7, 0.3])], None, &never)
        .unwrap()
        .strategies[0]
        .in_sample;
    assert!(run.trades.is_empty());
    close(
        &run.returns,
        &by_hand(&returns, &cash, 0..12, &[0.7, 0.3], &[], 0.0),
    );
    assert!((run.growth() - run.returns.iter().map(|r| 1.0 + r).product::<f64>()).abs() < 1e-15);
}

#[test]
fn cash_earns_the_risk_free_return_and_drift_bands_trade_only_when_crossed() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    // Half in A, the rest in cash.
    let settings =
        Settings::new(Validation::InSample).with_rebalance(Rebalance::Drift { band: 0.02 });
    let result = backtest(&market, vec![fixed("half", &[0.5, 0.0])], None, &settings).unwrap();
    let run = &result.strategies[0].in_sample;
    assert!((run.returns[0] - (0.5 * 0.05 + 0.5 * 0.002)).abs() < 1e-15);
    // Replay the band by hand.
    let mut held = 0.5;
    let mut trades = vec![];
    for t in 0..11 {
        let gross = held * returns.columns()[0][t] + (1.0 - held) * cash[t];
        held *= (1.0 + returns.columns()[0][t]) / (1.0 + gross);
        if (held - 0.5f64).abs() > 0.02 {
            trades.push(dates[t]);
            held = 0.5;
        }
    }
    assert!(!trades.is_empty());
    assert_eq!(
        run.trades.iter().map(|t| t.date).collect::<Vec<_>>(),
        trades
    );
}

#[test]
fn a_holdout_fits_before_the_start_and_tests_after_it() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let seen = RefCell::new(vec![]);
    let strategy = Strategy {
        name: "fitted".into(),
        plan: Plan::Fit(Box::new(|rows: Range<usize>| {
            seen.borrow_mut().push(rows);
            Ok(Fit::Weights(vec![0.4, 0.6]))
        })),
        notes: vec!["a note".into()],
    };
    let start: Date = "2025-07-15".parse().unwrap();
    let settings = Settings::new(Validation::Holdout { start });
    let result = backtest(&market, vec![strategy], None, &settings).unwrap();
    // The test starts with the first period ending on or after the start: July.
    assert_eq!(*seen.borrow(), [0..6]);
    let s = &result.strategies[0];
    assert_eq!(s.in_sample.dates, dates[..6]);
    let out = s.out_of_sample.as_ref().unwrap();
    assert_eq!(out.dates, dates[6..]);
    close(
        &out.returns,
        &by_hand(&returns, &cash, 6..12, &[0.4, 0.6], &[8], 0.0),
    );
    assert_eq!(s.notes, ["a note"]);
}

#[test]
fn walk_forward_refits_on_rolling_or_expanding_windows_and_keeps_weights_without_a_solution() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    for expanding in [false, true] {
        let seen = RefCell::new(vec![]);
        let strategy = Strategy {
            name: "fitted".into(),
            plan: Plan::Fit(Box::new(|rows: Range<usize>| {
                seen.borrow_mut().push(rows.clone());
                // No solution on the window that ends in June.
                Ok(if rows.end == 6 {
                    Fit::NoSolution("infeasible".into())
                } else {
                    Fit::Weights(vec![0.1 * rows.end as f64 / 12.0, 0.5])
                })
            })),
            notes: vec![],
        };
        let walk = Validation::walk_forward_periods(3);
        let settings = Settings::new(if expanding { walk.expanding() } else { walk });
        let result = backtest(&market, vec![strategy], None, &settings).unwrap();
        let from = |end: usize| if expanding { 0 } else { end - 3 };
        // The first fit on three months, refits at the quarter ends in June and September, and
        // the in-sample fit on everything.
        assert_eq!(*seen.borrow(), [from(3)..3, from(6)..6, from(9)..9, 0..12]);
        let s = &result.strategies[0];
        let out = s.out_of_sample.as_ref().unwrap();
        assert_eq!(out.dates, dates[3..]);
        assert_eq!(s.in_sample.dates, dates[3..]);
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].date.to_string(), "2025-06-30");
        assert_eq!(s.failures[0].reason, "infeasible");
        // June kept the weights held; September refitted.
        let traded: Vec<(String, bool)> = out
            .trades
            .iter()
            .map(|t| (t.date.to_string(), t.refit))
            .collect();
        assert_eq!(traded, [("2025-09-30".to_owned(), true)]);
        assert!((out.trades[0].weights["A"] - 0.1 * 9.0 / 12.0).abs() < 1e-15);
        assert!((out.start["A"] - 0.1 * 3.0 / 12.0).abs() < 1e-15);
        // In sample, the weights fitted to all twelve months, held from the same start.
        assert!((s.in_sample.start["A"] - 0.1).abs() < 1e-15);
    }
}

#[test]
fn a_strategy_with_no_first_solution_waits_in_cash() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let strategy = Strategy {
        name: "late".into(),
        plan: Plan::Fit(Box::new(|rows: Range<usize>| {
            Ok(if rows.end <= 3 {
                Fit::NoSolution("infeasible".into())
            } else {
                Fit::Weights(vec![1.0, 0.0])
            })
        })),
        notes: vec![],
    };
    let settings = Settings::new(Validation::walk_forward_periods(3));
    let result = backtest(&market, vec![strategy], None, &settings).unwrap();
    let out = result.strategies[0].out_of_sample.as_ref().unwrap();
    assert!(out.start.is_empty());
    // April, May and June earn cash; the June refit buys A.
    for r in &out.returns[..3] {
        assert!((r - 0.002).abs() < 1e-15);
    }
    assert!((out.returns[3] - 0.01).abs() < 1e-15);
    assert!((out.trades[0].turnover - 1.0).abs() < 1e-15);
}

#[test]
fn benchmarks_are_weights_held_alike_or_an_index_series() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let settings = Settings::new(Validation::Holdout { start: dates[6] });
    let index: Vec<f64> = (0..12).map(|t| 0.001 * t as f64).collect();
    let result = backtest(
        &market,
        vec![fixed("70/30", &[0.7, 0.3])],
        Some(("Index".into(), Benchmark::Returns(index.clone()))),
        &settings,
    )
    .unwrap();
    let benchmark = result.benchmark.unwrap();
    assert_eq!(benchmark.name, "Index");
    assert_eq!(benchmark.in_sample.returns, index[..6]);
    assert_eq!(benchmark.out_of_sample.unwrap().returns, index[6..]);
    let result = backtest(
        &market,
        vec![fixed("70/30", &[0.7, 0.3])],
        Some(("50/50".into(), Benchmark::Weights(vec![0.5, 0.5]))),
        &settings,
    )
    .unwrap();
    let benchmark = result.benchmark.unwrap();
    close(
        &benchmark.out_of_sample.unwrap().returns,
        &by_hand(&returns, &cash, 6..12, &[0.5, 0.5], &[8], 0.0),
    );
}

#[test]
fn every_problem_with_the_inputs_is_reported_together() {
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let mut settings = Settings::new(Validation::walk_forward(1.0))
        .with_rebalance(Rebalance::Drift { band: 1.5 })
        .with_trading_cost_bps(-1.0);
    let strategies = vec![
        fixed("same", &[0.7, 0.3]),
        fixed("same", &[0.9, 0.3]),
        fixed("short", &[1.0]),
    ];
    let error = backtest(
        &market,
        strategies,
        Some(("index".into(), Benchmark::Returns(vec![0.0; 3]))),
        &settings,
    )
    .unwrap_err();
    assert_eq!(
        paths(error),
        [
            "settings.trading_cost_bps",
            "settings.rebalance.band",
            "settings.validation.years",
            "settings.rebalance",
            "strategies[1].name",
            "strategies[1].weights",
            "strategies[2].weights",
            "benchmark.returns",
        ]
    );
    settings.validation = Validation::Holdout {
        start: "2025-01-31".parse().unwrap(),
    };
    settings.rebalance = Rebalance::Quarterly;
    settings.trading_cost_bps = 0.0;
    let error = backtest(&market, vec![], None, &settings).unwrap_err();
    assert_eq!(paths(error), ["settings.validation.start", "strategies"]);
}

#[test]
fn settings_read_and_write_as_json() {
    let settings: Settings = serde_json::from_str(
        r#"{"validation": {"type": "walk_forward", "years": 5},
            "rebalance": {"type": "drift", "band": 0.05},
            "trading_cost_bps": 10}"#,
    )
    .unwrap();
    assert_eq!(settings.validation, Validation::walk_forward(5.0));
    assert_eq!(settings.rebalance, Rebalance::Drift { band: 0.05 });
    let defaults: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults.validation, Validation::InSample);
    assert_eq!(defaults.rebalance, Rebalance::Quarterly);
    let text = serde_json::to_string(&Validation::Holdout {
        start: "2016-11-01".parse().unwrap(),
    })
    .unwrap();
    assert_eq!(text, r#"{"type":"holdout","start":"2016-11-01"}"#);
    let both: Settings = serde_json::from_str(
        r#"{"validation": {"type": "walk_forward", "years": 5, "periods": 60}}"#,
    )
    .unwrap();
    let (dates, returns, cash) = market_data();
    let market = Market::new(&dates, &returns, &cash, 12.0);
    let error = backtest(&market, vec![fixed("70/30", &[0.7, 0.3])], None, &both).unwrap_err();
    assert_eq!(paths(error), ["settings.validation"]);
}
