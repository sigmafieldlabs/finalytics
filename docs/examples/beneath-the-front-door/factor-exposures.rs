use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let factor_covariance = risk::Matrix::new(
        vec!["market".into(), "spread".into()],
        vec![0.04, 0.0, 0.0, 0.01],
    )?;
    let model = risk::FactorModel::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![vec![1.0, 0.0], vec![0.8, 1.0], vec![0.0, 0.5]],
        factor_covariance,
        vec![0.0025, 0.0025, 0.001],
    )?;
    let mut constraints = opt::Constraints::default();
    constraints.factor_exposures = vec![opt::FactorExposureLimit {
        factor: "market".into(),
        min: 0.4,
        max: 0.6,
        benchmark: None,
    }];
    let problem =
        opt::Problem::from_factor_model(model).with_constraints(constraints);
    let solution = problem.solve(&opt::Objective::MinimumVariance)?;
    println!("Status: {}", solution.status);
    let weights = solution
        .weights
        .as_ref()
        .expect("the guide's fixed sample supplies this value");
    for (label, weight) in solution.labels.iter().zip(weights) {
        println!("{}: {:.6}", label, weight);
    }
    println!("Market exposure: {:.6}", weights[0] + 0.8 * weights[1]);
    Ok(())
}
