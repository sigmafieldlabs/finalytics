//! The engine's numerical layers run in a WebAssembly runtime, including the solver and its
//! timing.

use finalytics::opt::{Objective, Problem};
use finalytics::risk::{Matrix, StatisticsOptions, statistics};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn statistics_run() {
    let returns = [0.010, -0.004, 0.007, -0.012, 0.003];
    let stats = statistics(&returns, &StatisticsOptions::default()).expect("valid returns");
    assert!(stats.volatility > 0.0);
}

#[wasm_bindgen_test]
fn the_optimiser_runs() {
    let covariance = Matrix::new(
        vec!["equity".into(), "bonds".into()],
        vec![0.0400, 0.0020, 0.0020, 0.0049],
    )
    .expect("a valid matrix");
    let problem = Problem::from_covariance(covariance).with_expected_returns(vec![0.07, 0.03]);
    for objective in [
        Objective::MaximumSharpe {
            risk_free_rate: 0.02,
        },
        Objective::RiskBudgeting { budgets: None },
        Objective::HierarchicalRiskParity {
            linkage: Default::default(),
        },
    ] {
        let solution = problem.solve(&objective).expect("a valid problem");
        assert!(solution.status.has_solution());
        assert!(solution.solve_time >= 0.0);
    }
}

#[wasm_bindgen_test]
fn a_screen_runs() {
    use finalytics::screening::Series;
    let dates: Vec<finalytics::Date> = (0..300)
        .map(|k| finalytics::Date::from_days_since_epoch(20_000 + k).expect("a date"))
        .collect();
    let close: Vec<f64> = (0..300).map(|k| 100.0 + f64::from(k)).collect();
    let universe =
        finalytics::Universe::from_series(vec![Series::new("UP", dates, close)]).expect("valid");
    let screen = universe
        .screen()
        .above("return_6m", 0.0)
        .build()
        .expect("a valid screen");
    assert_eq!(screen.results.matches, ["UP"]);
}
