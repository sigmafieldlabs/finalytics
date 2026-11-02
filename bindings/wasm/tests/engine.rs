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
