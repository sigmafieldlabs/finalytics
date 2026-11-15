use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["us equity".into(), "intl equity".into(), "bonds".into()],
        vec![
            0.0256, 0.02304, 0.0008, 0.02304, 0.0324, 0.0009, 0.0008, 0.0009,
            0.0025,
        ],
    )?;
    let problem = opt::Problem::from_covariance(covariance)
        .with_expected_returns(vec![0.07, 0.08, 0.03]);
    let objective = opt::Objective::RobustMeanVariance {
        risk_aversion: 4.0,
        uncertainty: opt::ReturnUncertainty::Ellipsoid {
            size: 0.5,
            covariance: None,
        },
    };
    let solution = problem.solve(&objective)?;
    println!("Status: {}", solution.status);
    let weights = solution
        .weights
        .as_ref()
        .expect("the guide's fixed sample supplies this value");
    for (label, weight) in solution.labels.iter().zip(weights) {
        println!("{}: {:.6}", label, weight);
    }
    println!(
        "Objective: {:.6}",
        solution
            .objective_value
            .expect("the guide's fixed sample supplies this value"),
    );
    Ok(())
}
