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
    constraints.bounds = opt::Bound { min: 0.0, max: 0.2 };
    let problem = opt::Problem::from_covariance(covariance)
        .with_expected_returns(vec![0.07, 0.045, 0.03])
        .with_constraints(constraints);
    let solution = problem.solve(&opt::Objective::MinimumVariance)?;
    println!("Status: {}", solution.status);
    match solution.weights.as_ref() {
        None => println!("Weights: none"),
        Some(weights) => {
            for (label, weight) in solution.labels.iter().zip(weights) {
                println!("{}: {:.6}", label, weight);
            }
        }
    }
    Ok(())
}
