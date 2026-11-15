use std::collections::BTreeMap;

use finalytics::{datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let factor_covariance = risk::Matrix::new(
        vec!["equity".into(), "rates".into()],
        vec![0.04, -0.0016, -0.0016, 0.0001],
    )?;
    let model = risk::FactorModel::new(
        vec![
            "VTI".into(),
            "EXSA".into(),
            "BND".into(),
            "GLD".into(),
            "VNQ".into(),
        ],
        vec![
            vec![1.0, 0.0],
            vec![1.1, 0.0],
            vec![0.0, -6.0],
            vec![0.3, 0.0],
            vec![0.8, -1.0],
        ],
        factor_covariance,
        vec![0.01, 0.01, 0.0025, 0.01, 0.01],
    )?;
    let factor_shocks = BTreeMap::from([("equity".into(), -0.15)]);
    let mut factors = risk::FactorShocks::new(model, factor_shocks);
    factors.propagate = true;
    let mut shocks = risk::Shocks::default();
    shocks.factors = Some(factors);
    let result = risk::shock_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio
            .as_of()
            .expect("the guide's fixed sample supplies this value"),
        &shocks,
        &risk::ScenarioOptions::default(),
    )?;
    for (factor, value) in &result.factor_returns {
        println!("{}: {:.6}", factor, value);
    }
    for holding in result.holdings.iter().take(5) {
        println!("{}: local={:.6}", holding.holding, holding.return_local);
    }
    Ok(())
}
