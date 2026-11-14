use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0060, 0.0020, 0.0060, 0.0144, 0.0015, 0.0020, 0.0015,
            0.0036,
        ],
    )?;
    let mut constraints = opt::Constraints::default();
    let mut turnover = opt::Turnover::new(vec![0.6, 0.2, 0.2]);
    turnover.costs = Some(vec![0.003, 0.003, 0.003]);
    turnover.quadratic_costs = Some(vec![0.05, 0.10, 0.20]);
    constraints.turnover = Some(turnover);
    let problem = opt::Problem::from_covariance(covariance)
        .with_expected_returns(vec![0.07, 0.045, 0.03])
        .with_constraints(constraints);
    let objective = opt::Objective::MeanVariance { risk_aversion: 4.0 };
    let solution = problem.solve(&objective)?;
    println!("Status: {}", solution.status);
    let weights = solution.weights.as_ref().unwrap();
    for (label, weight) in solution.labels.iter().zip(weights) {
        println!("{}: {:.6}", label, weight);
    }
    println!("Largest violation: {:.9}", solution.max_violation.unwrap(),);
    Ok(())
}
