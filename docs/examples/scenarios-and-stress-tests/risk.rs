use finalytics::{datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let labels = vec![
        "VTI".into(),
        "EXSA".into(),
        "BND".into(),
        "GLD".into(),
        "VNQ".into(),
        "EUR cash".into(),
    ];
    let covariance = risk::Matrix::new(
        labels,
        vec![
            0.0400, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0625, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0036, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0225, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0400, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0001,
        ],
    )?;
    let mut model = risk::ScenarioRiskModel::from_covariance(covariance);
    model.confidence = 0.99;
    model.stress = Some(risk::CorrelationStress {
        volatility_scale: 1.5,
        correlation_shift: 0.5,
    });
    let mut options = risk::ScenarioOptions::default();
    options.risk = Some(model);
    let mut shocks = risk::Shocks::default();
    shocks.groups = vec![risk::GroupShock {
        scheme: "asset_class".into(),
        value: "Equity".into(),
        shock: -0.20,
    }];
    let result = risk::shock_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio.as_of().unwrap(),
        &shocks,
        &options,
    )?;
    for (name, scenario_risk) in [
        ("Before", result.risk_before.as_ref().unwrap()),
        ("After", result.risk_after.as_ref().unwrap()),
    ] {
        println!("{}: volatility={:.6}", name, scenario_risk.volatility);
        println!(
            "{}: value at risk={:.2}",
            name,
            scenario_risk.value_at_risk_base,
        );
    }
    Ok(())
}
