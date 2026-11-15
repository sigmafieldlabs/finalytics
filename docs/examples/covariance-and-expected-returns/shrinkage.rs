use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = risk::ReturnsTable::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            vec![0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
            vec![0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
            vec![-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
        ],
    )?;
    for (name, method) in [
        ("ledoit_wolf", risk::CovarianceMethod::LedoitWolf),
        ("oas", risk::CovarianceMethod::Oas),
        (
            "constant_correlation",
            risk::CovarianceMethod::ConstantCorrelation,
        ),
    ] {
        let estimate = risk::covariance(&returns, method)?;
        println!(
            "{}: shrinkage={:.6} equity_variance={:.8}",
            name,
            estimate
                .shrinkage
                .expect("the guide's fixed sample supplies this value"),
            estimate.matrix.get(0, 0),
        );
    }
    Ok(())
}
