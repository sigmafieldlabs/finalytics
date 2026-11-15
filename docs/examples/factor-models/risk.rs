use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let factor_covariance = risk::Matrix::new(
        vec!["equity".into(), "rates".into()],
        vec![0.04, -0.0016, -0.0016, 0.0001],
    )?;
    let model = risk::FactorModel::new(
        vec!["SPY".into(), "SAP".into(), "BOND".into()],
        vec![vec![1.0, 0.0], vec![1.2, 0.0], vec![0.0, -6.0]],
        factor_covariance,
        vec![0.01, 0.02, 0.0001],
    )?;
    let weights = [0.4, 0.3, 0.3];
    let benchmark = [0.3, 0.3, 0.4];
    let portfolio = risk::factor_risk(&weights, &model)?;
    let active = risk::factor_tracking_error(&weights, &benchmark, &model)?;
    for (name, result) in [("Portfolio", portfolio), ("Active", active)] {
        println!("{}: volatility={:.6}", name, result.volatility);
        for item in &result.factors {
            println!(
                "  {}: exposure={:.6} component={:.6} share={:.6}",
                item.factor,
                item.exposure,
                item.component,
                item.share
                    .expect("the guide's fixed sample supplies this value"),
            );
        }
        println!(
            "  specific: share={:.6}",
            result
                .specific_share
                .expect("the guide's fixed sample supplies this value")
        );
    }
    Ok(())
}
