//! Properties of solutions, infeasible problems and input validation.

#![allow(clippy::unwrap_used)]

use finalytics_core::Error;
use finalytics_opt::*;
use finalytics_risk::{Matrix, ReturnsTable, risk_decomposition};

fn labels() -> Vec<String> {
    ["equity", "credit", "bonds", "gold"]
        .map(String::from)
        .to_vec()
}

fn covariance() -> Matrix {
    Matrix::new(
        labels(),
        vec![
            0.0400, 0.0120, 0.0010, 0.0020, //
            0.0120, 0.0144, 0.0015, 0.0010, //
            0.0010, 0.0015, 0.0036, 0.0005, //
            0.0020, 0.0010, 0.0005, 0.0256,
        ],
    )
    .unwrap()
}

fn problem() -> Problem {
    Problem::from_covariance(covariance()).with_expected_returns(vec![0.08, 0.05, 0.03, 0.04])
}

fn paths(error: Error) -> Vec<String> {
    match error {
        Error::Validation(report) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, found {other}"),
    }
}

#[test]
fn risk_parity_equalises_risk_shares() {
    let solution = problem()
        .solve(&Objective::RiskBudgeting { budgets: None })
        .unwrap();
    assert_eq!(solution.method, SolveMethod::Newton);
    assert!(solution.objective_value.unwrap() < 1e-12);
    let weights = solution.weights.unwrap();
    let risk = risk_decomposition(&weights, &covariance()).unwrap();
    for c in &risk.contributions {
        assert!((c.share.unwrap() - 0.25).abs() < 1e-12);
    }

    let budgets = vec![0.4, 0.3, 0.2, 0.1];
    let solution = problem()
        .solve(&Objective::RiskBudgeting {
            budgets: Some(budgets.clone()),
        })
        .unwrap();
    let risk = risk_decomposition(&solution.weights.unwrap(), &covariance()).unwrap();
    for (c, b) in risk.contributions.iter().zip(&budgets) {
        assert!((c.share.unwrap() - b).abs() < 1e-12);
    }

    let constrained = problem().with_constraints({
        let mut constraints = Constraints::default();
        constraints.leverage = Some(1.0);
        constraints
    });
    assert_eq!(
        paths(
            constrained
                .solve(&Objective::RiskBudgeting { budgets: None })
                .unwrap_err()
        ),
        ["constraints"]
    );
}

#[test]
fn equal_weight_and_inverse_volatility_follow_their_formulas() {
    let equal = problem().solve(&Objective::EqualWeight).unwrap();
    assert_eq!(equal.status, SolveStatus::Optimal);
    assert_eq!(equal.method, SolveMethod::ClosedForm);
    assert_eq!(equal.weights.as_deref(), Some(&[0.25; 4][..]));
    // Equal weight needs no estimates at all.
    let mut bare = Problem::from_covariance(covariance());
    bare.covariance = None;
    assert_eq!(
        bare.solve(&Objective::EqualWeight)
            .unwrap()
            .weights
            .unwrap(),
        [0.25; 4]
    );

    let inverse = problem().solve(&Objective::InverseVolatility).unwrap();
    let w = inverse.weights.unwrap();
    // Volatilities 20%, 12%, 6% and 16%: weights in proportion to 5, 8.33, 16.67 and 6.25.
    let raw = [1.0 / 0.20, 1.0 / 0.12, 1.0 / 0.06, 1.0 / 0.16];
    let total: f64 = raw.iter().sum();
    for (got, r) in w.iter().zip(raw) {
        assert!((got - r / total).abs() < 1e-12);
    }
    assert!(
        (inverse.volatility.unwrap() - risk_decomposition(&w, &covariance()).unwrap().volatility)
            .abs()
            < 1e-12
    );

    let mut limited = problem();
    limited.constraints.bounds = Bound { min: 0.0, max: 0.3 };
    for objective in [Objective::EqualWeight, Objective::InverseVolatility] {
        assert_eq!(
            paths(limited.solve(&objective).unwrap_err()),
            ["constraints"]
        );
    }
    let riskless = Matrix::new(labels(), {
        let mut values = covariance().values().to_vec();
        values[15] = 0.0;
        values[3] = 0.0;
        values[7] = 0.0;
        values[11] = 0.0;
        values[12] = 0.0;
        values[13] = 0.0;
        values[14] = 0.0;
        values
    })
    .unwrap();
    let error = Problem::from_covariance(riskless)
        .solve(&Objective::InverseVolatility)
        .unwrap_err();
    assert_eq!(paths(error), ["covariance[3]"]);
}

#[test]
fn infeasible_constraints_give_a_status_not_weights() {
    let mut constraints = Constraints::default();
    constraints.bounds = Bound { min: 0.0, max: 0.2 };
    let solution = problem()
        .with_constraints(constraints)
        .solve(&Objective::MinimumVariance)
        .unwrap();
    assert_eq!(solution.status, SolveStatus::Infeasible);
    assert_eq!(solution.weights, None);
    assert_eq!(solution.max_violation, None);

    // No portfolio earns more than the risk-free rate, so the Sharpe ratio has no positive
    // maximum.
    let solution = problem()
        .solve(&Objective::MaximumSharpe {
            risk_free_rate: 0.10,
        })
        .unwrap();
    assert!(!solution.status.has_solution());
}

#[test]
fn a_riskless_asset_is_handled_by_the_cone_formulations() {
    let names = vec!["equity".to_owned(), "cash".to_owned()];
    let covariance = Matrix::new(names, vec![0.04, 0.0, 0.0, 0.0]).unwrap();
    let problem = Problem::from_covariance(covariance).with_expected_returns(vec![0.07, 0.02]);
    let solution = problem
        .solve(&Objective::TargetVolatility { target: 0.05 })
        .unwrap();
    let weights = solution.weights.unwrap();
    // A quarter in equity gives 5% volatility; the rest earns the cash rate.
    assert!((weights[0] - 0.25).abs() < 1e-6, "{weights:?}");
    assert!((solution.expected_return.unwrap() - (0.25 * 0.07 + 0.75 * 0.02)).abs() < 1e-7);
    assert!(solution.max_violation.unwrap() < 1e-7);

    // All in cash, to the solver's tolerance on the variance.
    let solution = problem.solve(&Objective::MinimumVariance).unwrap();
    assert!(
        solution.objective_value.unwrap() < 1e-8,
        "{:?}",
        solution.weights
    );
    assert!(
        problem
            .solve(&Objective::RiskBudgeting { budgets: None })
            .is_err()
    );
}

#[test]
fn the_frontier_rises_from_the_minimum_variance_portfolio() {
    let frontier = problem().frontier(8).unwrap();
    assert_eq!(frontier.len(), 8);
    let minimum = problem().solve(&Objective::MinimumVariance).unwrap();
    assert!((frontier[0].volatility - minimum.volatility.unwrap()).abs() < 1e-6);
    // The largest attainable return, long-only, is all in equity.
    assert!((frontier[7].expected_return - 0.08).abs() < 1e-6);
    for pair in frontier.windows(2) {
        assert!(pair[1].expected_return > pair[0].expected_return);
        assert!(pair[1].volatility >= pair[0].volatility - 1e-9);
    }
    // The tangency portfolio is no riskier than the frontier at its return.
    let sharpe = problem()
        .solve(&Objective::MaximumSharpe {
            risk_free_rate: 0.0,
        })
        .unwrap();
    let target = sharpe.expected_return.unwrap();
    let on_frontier = problem()
        .solve(&Objective::TargetReturn { target })
        .unwrap();
    assert!((on_frontier.volatility.unwrap() - sharpe.volatility.unwrap()).abs() < 1e-6);
    assert!(problem().frontier(1).is_err());
}

#[test]
fn minimum_cvar_beats_equal_weights() {
    let names = labels();
    // Equity loses heavily in the last two scenarios; bonds gain.
    let columns = vec![
        vec![0.02, 0.01, 0.03, -0.01, 0.02, -0.08, -0.12],
        vec![0.01, 0.01, 0.02, 0.00, 0.01, -0.04, -0.05],
        vec![0.00, 0.00, -0.01, 0.01, 0.00, 0.02, 0.03],
        vec![-0.01, 0.00, 0.01, 0.00, 0.01, 0.01, 0.02],
    ];
    let scenarios = ReturnsTable::new(names, columns).unwrap();
    let problem = Problem::from_scenarios(scenarios);
    let objective = Objective::MinimumCvar { confidence: 0.7 };
    let solution = problem.solve(&objective).unwrap();
    assert_eq!(solution.status, SolveStatus::Optimal);
    assert_eq!(solution.volatility, None);
    let cvar = solution.objective_value.unwrap();
    // Equal weights lose 4.75% and 3.5% in the two worst scenarios, and 0.75% in the third.
    let equal = (0.0475 + 0.035 + 0.1 * 0.0075) / 2.1;
    assert!(cvar < equal);
    assert!(
        cvar < 0.0,
        "a hedged portfolio gains even in the worst scenarios: {cvar}"
    );
}

#[test]
fn invalid_problems_list_every_problem() {
    let mut constraints = Constraints::default();
    constraints.bounds = Bound { min: 0.5, max: 0.2 };
    constraints.groups = vec![GroupLimit {
        name: "metals".into(),
        assets: vec!["gold".into(), "silver".into()],
        min: 0.0,
        max: 0.3,
    }];
    let mut turnover = Turnover::new(vec![0.25; 3]);
    turnover.limit = Some(0.1);
    turnover.costs = Some(vec![0.001; 4]);
    constraints.turnover = Some(turnover);
    let error = Problem::from_covariance(covariance())
        .with_constraints(constraints)
        .solve(&Objective::MinimumVariance)
        .unwrap_err();
    assert_eq!(
        paths(error),
        [
            "constraints.bounds",
            "constraints.groups[0].assets[1]",
            "constraints.turnover.current",
            "constraints.turnover.costs",
        ]
    );
    let error = Problem::from_covariance(covariance())
        .solve(&Objective::MaximumSharpe {
            risk_free_rate: 0.0,
        })
        .unwrap_err();
    assert_eq!(paths(error), ["expected_returns"]);

    let indefinite = Matrix::new(labels()[..2].to_vec(), vec![1.0, 2.0, 2.0, 1.0]).unwrap();
    assert_eq!(
        paths(
            Problem::from_covariance(indefinite)
                .solve(&Objective::MinimumVariance)
                .unwrap_err()
        ),
        ["covariance"]
    );
}

#[test]
fn problems_and_solutions_round_trip_through_json() {
    let problem = problem().with_constraints({
        let mut constraints = Constraints::default();
        constraints.groups = vec![GroupLimit {
            name: "risk assets".into(),
            assets: vec!["equity".into(), "credit".into()],
            min: 0.2,
            max: 0.6,
        }];
        constraints
    });
    let text = serde_json::to_string(&problem).unwrap();
    let back: Problem = serde_json::from_str(&text).unwrap();
    assert_eq!(back, problem);

    let objective: Objective =
        serde_json::from_str(r#"{"type": "mean_variance", "risk_aversion": 3}"#).unwrap();
    let solution = back.solve(&objective).unwrap();
    let json = serde_json::to_value(&solution).unwrap();
    assert_eq!(json["status"], "optimal");
    assert_eq!(json["method"], "interior_point");
    assert_eq!(json["objective"]["type"], "mean_variance");
}

#[test]
fn solving_twice_gives_equal_solutions() {
    // The time taken varies from run to run and is left out of the comparison.
    let objective = Objective::MaximumSharpe {
        risk_free_rate: 0.02,
    };
    let first = problem().solve(&objective).unwrap();
    let mut second = problem().solve(&objective).unwrap();
    second.solve_time = first.solve_time + 1.0;
    assert_eq!(first, second);
    second.iterations += 1;
    assert_ne!(first, second);
}
