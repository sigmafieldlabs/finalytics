//! The planning engine on histories whose returns are known, so that every figure can be worked
//! out by hand.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics_core::Error;
use finalytics_plan::{
    Flow, Frequency, Goal, History, Inflation, Mix, Model, Month, PlanOptions, Status, simulate,
};

fn month(s: &str) -> Month {
    s.parse().unwrap()
}

/// `months` months of constant returns from January 2000, one column per asset.
fn constant(returns: &[(&str, f64)], months: usize) -> History {
    History {
        first: month("2000-01"),
        assets: returns.iter().map(|(a, _)| (*a).to_owned()).collect(),
        returns: returns.iter().map(|(_, r)| vec![*r; months]).collect(),
        inflation: None,
        estimates: None,
    }
}

/// Random-looking returns from a fixed recurrence, so that simulations differ.
fn varied(months: usize) -> History {
    let mut x: u64 = 12345;
    let mut next = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((x >> 33) as f64 / (1u64 << 31) as f64) - 0.5
    };
    let stocks: Vec<f64> = (0..months).map(|_| 0.007 + 0.09 * next()).collect();
    let bonds: Vec<f64> = (0..months).map(|_| 0.003 + 0.02 * next()).collect();
    History {
        first: month("1990-01"),
        assets: vec!["Stocks".into(), "Bonds".into()],
        returns: vec![stocks, bonds],
        inflation: None,
        estimates: None,
    }
}

fn all_in(asset: &str) -> Mix {
    Mix::fixed([(asset, 1.0)])
}

fn saving(amount: f64) -> Flow {
    Flow::Saving {
        name: None,
        amount,
        on: None,
        every: Some(Frequency::Month),
        start: None,
        end: None,
        growth: None,
    }
}

fn base(goal: Goal, weights: Mix) -> PlanOptions {
    let mut o = PlanOptions::default();
    o.goal = Some(goal);
    o.weights = Some(weights);
    o.inflation = Some(Inflation::Rate { rate: 0.0 });
    o.start = Some(month("2010-01"));
    o.simulations = 200;
    o
}

fn grown(start: f64, save: f64, g: f64, months: usize) -> f64 {
    let mut v = start;
    for _ in 0..months {
        v = (v + save) * g;
    }
    v
}

#[test]
fn a_target_with_known_returns_is_met_exactly_when_the_value_reaches_it() {
    let history = constant(&[("A", 0.01)], 48);
    let reached = grown(1_000.0, 100.0, 1.01, 12);
    for (amount, met) in [(reached - 0.01, true), (reached + 0.01, false)] {
        let mut o = base(
            Goal::Target {
                name: Some("Car".into()),
                amount,
                by: month("2011-01"),
            },
            all_in("A"),
        );
        o.start_value = 1_000.0;
        o.flows = vec![saving(100.0)];
        let r = simulate(&history, &o).unwrap();
        assert_eq!(r.outcome.chance, Some(if met { 1.0 } else { 0.0 }));
        assert!((r.end_value.p50 - reached).abs() < 1e-9);
        assert_eq!(r.months.len(), 13);
        assert_eq!(r.months[0], month("2010-01"));
        assert_eq!(r.end, month("2011-01"));
        assert_eq!(r.sample.len(), 100);
        assert!((r.values[12].p10 - reached).abs() < 1e-9);
        assert_eq!(r.chance_by_month.len(), 13);
        let expected = if met {
            Status::RoomToSpare
        } else {
            Status::OffTrack
        };
        assert_eq!(r.status, Some(expected));
        if !met {
            assert!((r.shortfall.unwrap() - 0.01).abs() < 1e-6);
        }
    }
}

#[test]
fn inflation_and_fees_come_off_every_return() {
    let history = constant(&[("A", 0.01)], 48);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2012-01"),
        },
        all_in("A"),
    );
    o.start_value = 1_000.0;
    o.fee = 0.012;
    o.inflation = Some(Inflation::Rate { rate: 0.03 });
    let r = simulate(&history, &o).unwrap();
    let g = (1.0 + 0.01 - 0.001) / libm::pow(1.03, 1.0 / 12.0);
    assert!((r.end_value.p50 - 1_000.0 * g.powi(24)).abs() < 1e-9);
    assert!((r.risk.typical_return - (g.powi(12) - 1.0)).abs() < 1e-12);
    assert!((r.risk.worst_year.unwrap() - (g.powi(12) - 1.0)).abs() < 1e-12);
    assert!(r.risk.largest_fall.abs() < 1e-12);
}

#[test]
fn a_flow_fixed_in_money_of_the_day_shrinks_in_todays_money() {
    let history = constant(&[("A", 0.0)], 48);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2011-01"),
        },
        all_in("A"),
    );
    o.inflation = Some(Inflation::Rate { rate: 0.12 });
    o.flows = vec![Flow::Saving {
        name: None,
        amount: 100.0,
        on: None,
        every: Some(Frequency::Month),
        start: None,
        end: None,
        growth: Some(0.0),
    }];
    let r = simulate(&history, &o).unwrap();
    let pi = libm::pow(1.12, 1.0 / 12.0);
    // Each payment of 100 in money of the day is worth 100 / pi^t today, and growth after
    // inflation is 1 / pi a month.
    let expected: f64 = (0..12).map(|t| 100.0 / pi.powi(t) / pi.powi(12 - t)).sum();
    assert!((r.end_value.p50 - expected).abs() < 1e-9);
}

#[test]
fn payments_run_short_when_the_money_runs_out() {
    let history = constant(&[("A", 0.0)], 48);
    let mut o = base(
        Goal::Payments {
            name: Some("Income".into()),
            amount: 1_000.0,
            every: Frequency::Month,
            start: month("2010-01"),
            end: month("2011-08"),
        },
        all_in("A"),
    );
    o.start_value = 10_500.0;
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.outcome.chance, Some(0.0));
    let p = r.payments.unwrap();
    // Ten payments in full, half of the eleventh, then nothing: 10.5 of 20.
    assert!((p.paid - 10.5 / 20.0).abs() < 1e-12);
    assert_eq!(p.runs_short, Some(month("2010-11")));
    assert_eq!(p.runs_out_weak, Some(month("2010-11")));
    assert_eq!(p.runs_out_average, Some(month("2010-11")));
    assert_eq!(p.lasts_until, Some(month("2010-10")));
    assert_eq!(r.outcome.runs_out, Some(month("2010-11")));
    // Starting the payments later, saving 500 a month until then, would pay them all.
    o.flows = vec![saving(500.0)];
    let r = simulate(&history, &o).unwrap();
    let changes = r.changes.unwrap();
    assert!(changes.to_reach);
    let date = changes.date.unwrap();
    assert_eq!(date.from, month("2010-01"));
    // From month k, 10,500 + 500 k pays 20 - k payments of 1,000: k = 7.
    assert_eq!(date.to, month("2010-08"));
    let amount = changes.amount.unwrap();
    assert_eq!(amount.from, 1_000.0);
    // The largest payment met is 10,500 / 20 = 525, to three figures 520.
    assert_eq!(amount.to, 520.0);
}

#[test]
fn spending_is_paid_as_far_as_the_money_goes() {
    let history = constant(&[("A", 0.0)], 48);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 100.0,
            by: month("2011-01"),
        },
        all_in("A"),
    );
    o.start_value = 500.0;
    o.flows = vec![Flow::Spending {
        name: Some("Car".into()),
        amount: 800.0,
        on: Some(month("2010-06")),
        every: None,
        start: None,
        end: None,
        growth: None,
    }];
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.flows[0].paid_in_full, Some(0.0));
    assert_eq!(r.flows[0].count, 1);
    assert_eq!(r.end_value.p50, 0.0);
    assert_eq!(r.outcome.chance, Some(0.0));
}

#[test]
fn the_saving_that_reaches_the_chance_is_found_to_three_figures() {
    let history = constant(&[("A", 0.005)], 48);
    let target = grown(0.0, 2_345.6, 1.005, 24);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: target,
            by: month("2012-01"),
        },
        all_in("A"),
    );
    o.flows = vec![saving(2_000.0)];
    let r = simulate(&history, &o).unwrap();
    let c = r.changes.unwrap();
    let s = c.saving.unwrap();
    assert_eq!(s.what, "Monthly saving");
    assert_eq!((s.from, s.to), (2_000.0, 2_350.0));
    // Saving as planned, the target is met by a later date.
    assert!(c.date.unwrap().to > month("2012-01"));
    // The largest target met in every simulation is the value reached, rounded down.
    let a = c.amount.unwrap();
    assert!(
        a.to <= grown(0.0, 2_000.0, 1.005, 24) && a.to > grown(0.0, 2_000.0, 1.005, 24) - 1_000.0
    );
}

#[test]
fn room_to_spare_shows_what_the_plan_could_absorb() {
    let history = constant(&[("A", 0.005)], 48);
    let target = grown(0.0, 1_000.0, 1.005, 24);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: target,
            by: month("2012-01"),
        },
        all_in("A"),
    );
    o.flows = vec![saving(1_500.0)];
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.status, Some(Status::RoomToSpare));
    let c = r.changes.unwrap();
    assert!(!c.to_reach);
    assert_eq!(c.saving.unwrap().to, 1_000.0);
    assert!(c.date.unwrap().to < month("2012-01"));
}

#[test]
fn a_glide_path_moves_in_a_straight_line_between_its_months() {
    let history = constant(&[("Cash", 0.0), ("Growth", 0.01)], 48);
    let mut points = BTreeMap::new();
    points.insert(month("2010-01"), BTreeMap::from([("Cash".to_owned(), 1.0)]));
    points.insert(
        month("2010-11"),
        BTreeMap::from([("Growth".to_owned(), 1.0)]),
    );
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2011-01"),
        },
        Mix::GlidePath { name: None, points },
    );
    o.start_value = 100.0;
    let r = simulate(&history, &o).unwrap();
    // Growth's weight is t/10 in month t up to 10, then 1.
    let expected: f64 = (0..12)
        .map(|t| 1.0 + 0.01 * (f64::from(t) / 10.0).min(1.0))
        .product::<f64>()
        * 100.0;
    assert!((r.end_value.p50 - expected).abs() < 1e-9);
    assert_eq!(r.weights, vec![("Cash".to_owned(), 1.0)]);
}

#[test]
fn the_same_seed_gives_the_same_plan_and_another_seed_another() {
    let history = varied(240);
    let mut o = base(
        Goal::Payments {
            name: None,
            amount: 1_800.0,
            every: Frequency::Month,
            start: month("2020-01"),
            end: month("2044-12"),
        },
        Mix::fixed([("Stocks", 0.6), ("Bonds", 0.4)]),
    );
    o.start_value = 100_000.0;
    o.flows = vec![saving(500.0)];
    o.simulations = 500;
    o.alternatives = vec![Mix::fixed([("Bonds", 1.0)]).named("All bonds")];
    let a = simulate(&history, &o).unwrap();
    let b = simulate(&history, &o).unwrap();
    assert_eq!(a, b);
    o.seed = 1;
    let c = simulate(&history, &o).unwrap();
    assert_ne!(a.end_value, c.end_value);
    let chance = a.outcome.chance.unwrap();
    assert!(chance > 0.0 && chance < 1.0, "{chance}");
    assert_eq!(a.alternatives.len(), 1);
    assert_eq!(a.alternatives[0].name, "All bonds");
    let met = a.sample.iter().filter(|p| p.met).count() as f64 / 100.0;
    assert!((met - chance).abs() <= 0.011, "{met} against {chance}");
    // Lower returns lower the chance.
    assert!(a.what_if.lower_returns.chance.unwrap() <= chance);
    let window = a.what_if.crash_window.unwrap();
    assert_eq!(window.placed, month("2020-01"));
    assert!(window.loss < 0.0);
}

#[test]
fn the_historical_model_runs_every_stretch_of_the_history_once() {
    let history = varied(120);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1_000.0,
            by: month("2012-01"),
        },
        all_in("Stocks"),
    );
    o.model = Model::Historical;
    o.start_value = 1_000.0;
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.simulations, 120 - 24 + 1);
    o.goal = Some(Goal::Target {
        name: None,
        amount: 1_000.0,
        by: month("2025-01"),
    });
    let Err(Error::Validation(report)) = simulate(&history, &o) else {
        panic!("a plan longer than the history");
    };
    assert!(report.to_string().contains("history has 120"));
}

#[test]
fn every_problem_is_reported_together() {
    let history = constant(&[("A", 0.0)], 48);
    let mut o = PlanOptions::default();
    o.goal = Some(Goal::Target {
        name: None,
        amount: -5.0,
        by: month("2010-01"),
    });
    o.start = Some(month("2010-01"));
    o.chance = 1.5;
    o.weights = Some(Mix::fixed([("A", 0.5), ("Z", 0.4)]));
    o.flows = vec![Flow::Saving {
        name: None,
        amount: 10.0,
        on: Some(month("2009-01")),
        every: None,
        start: None,
        end: None,
        growth: None,
    }];
    let Err(Error::Validation(report)) = simulate(&history, &o) else {
        panic!("invalid options");
    };
    let text = report.to_string();
    for path in [
        "chance",
        "inflation",
        "goal.amount",
        "goal.by",
        "flows[0].on",
        "weights.weights.Z",
        "weights.weights",
    ] {
        assert!(text.contains(path), "{path} missing from {text}");
    }
    let short = constant(&[("A", 0.0)], 12);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2011-01"),
        },
        all_in("A"),
    );
    o.start = None;
    let Err(Error::Validation(report)) = simulate(&short, &o) else {
        panic!("a short history");
    };
    assert!(report.to_string().contains("at least 36"));
}

#[test]
fn savings_are_paid_in_before_spending_whatever_their_order() {
    // Nothing to start with: a car of 500 in the first month is paid in full only if that
    // month's saving of 1,000 comes in first, as it does in either order.
    let history = constant(&[("Cash", 0.0)], 60);
    let car = Flow::Spending {
        name: Some("Car".into()),
        amount: 500.0,
        on: Some(month("2010-01")),
        every: None,
        start: None,
        end: None,
        growth: None,
    };
    let goal = Goal::Target {
        name: None,
        amount: 1.0,
        by: month("2010-06"),
    };
    for flows in [
        vec![car.clone(), saving(1_000.0)],
        vec![saving(1_000.0), car],
    ] {
        let mut o = base(goal.clone(), all_in("Cash"));
        o.flows = flows;
        let r = simulate(&history, &o).unwrap();
        let car = r.flows.iter().find(|f| f.name == "Car").unwrap();
        assert_eq!(car.paid_in_full, Some(1.0));
        assert_eq!(r.end_value.p50, 5.0 * 1_000.0 - 500.0);
    }
}

#[test]
fn an_earlier_first_payment_is_found_when_there_is_room() {
    // With no growth, 100,000 and 500 a month from 2010-01 make 130,000 by the first payment in
    // 2015-01, for 120 payments of 1,000: 10,000 to spare. Starting k months earlier leaves
    // 130,000 - 500 k for 120 + k payments, so k is at most 6.
    let history = constant(&[("Cash", 0.0)], 60);
    let mut o = base(
        Goal::Payments {
            name: None,
            amount: 1_000.0,
            every: Frequency::Month,
            start: month("2015-01"),
            end: month("2024-12"),
        },
        all_in("Cash"),
    );
    o.start_value = 100_000.0;
    o.flows = vec![saving(500.0)];
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.outcome.chance, Some(1.0));
    let c = r.changes.unwrap();
    assert!(!c.to_reach);
    assert_eq!(c.date.unwrap().to, month("2014-07"));
}

#[test]
fn malformed_inputs_are_errors_not_panics() {
    let mut history = constant(&[("Cash", 0.0)], 60);
    history.returns[0].pop();
    history.assets.push("Missing".into());
    let o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2011-01"),
        },
        all_in("Cash"),
    );
    let Err(Error::Validation(report)) = simulate(&history, &o) else {
        panic!("not a validation error")
    };
    assert_eq!(report.issues()[0].path, "assets");

    let history = constant(&[("Cash", 0.0)], 60);
    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2011-01"),
        },
        all_in("Cash"),
    );
    o.chance = 0.995;
    o.model = Model::Historical;
    o.expected_returns = Some(BTreeMap::from([("Cash".to_owned(), 0.02)]));
    o.flows = vec![Flow::Saving {
        name: None,
        amount: 1.0,
        on: None,
        every: Some(Frequency::Month),
        start: Some(month("2010-06")),
        end: Some(month("2010-03")),
        growth: None,
    }];
    let Err(Error::Validation(report)) = simulate(&history, &o) else {
        panic!("not a validation error")
    };
    let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
    for path in ["chance", "expected_returns", "flows[0].end"] {
        assert!(paths.contains(&path), "{paths:?}");
    }

    let mut o = base(
        Goal::Target {
            name: None,
            amount: 1.0,
            by: month("2109-12"),
        },
        all_in("Cash"),
    );
    o.simulations = 1_000_000;
    let Err(Error::Validation(report)) = simulate(&history, &o) else {
        panic!("not a validation error")
    };
    assert!(report.issues().iter().any(|i| i.path == "simulations"));
}

#[test]
fn a_plan_shorter_than_a_year_has_no_worst_year_and_a_shorter_crash() {
    let history = varied(240);
    let o = base(
        Goal::Target {
            name: None,
            amount: 1_000.0,
            by: month("2010-07"),
        },
        Mix::fixed([("Stocks", 0.6), ("Bonds", 0.4)]),
    );
    let mut o = o;
    o.start_value = 900.0;
    let r = simulate(&history, &o).unwrap();
    assert_eq!(r.risk.worst_year, None);
    let w = r.what_if.crash_window.unwrap();
    assert_eq!(w.from.months_until(w.to) + 1, 6);
    assert_eq!(w.placed, month("2010-01"));
}
