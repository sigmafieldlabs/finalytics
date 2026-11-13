use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0060, 0.0020,
            0.0060, 0.0144, 0.0015,
            0.0020, 0.0015, 0.0036,
        ],
    )?;
    let mut constraints = opt::Constraints::default();
    constraints.bounds = opt::Bound { min: -0.5, max: 1.0 };
    constraints.leverage = Some(1.5);
    let problem = opt::Problem::from_covariance(covariance)
        .with_expected_returns(vec![0.07, 0.045, 0.03])
        .with_constraints(constraints);
    let objective = opt::Objective::TargetReturn { target: 0.07 };
    let solution = problem.solve(&objective)?;
    println!("Status: {}", solution.status);
    let weights = solution.weights.as_ref().unwrap();
    for (label, weight) in solution.labels.iter().zip(weights) {
        println!("{}: {:.6}", label, weight);
    }
    println!(
        "Largest violation: {:.9}",
        solution.max_violation.unwrap(),
    );
    let gross: f64 = weights.iter().map(|weight| weight.abs()).sum();
    println!("Gross exposure: {:.6}", gross);
    Ok(())
}
