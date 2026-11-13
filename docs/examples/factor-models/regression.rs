use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let columns = [
        [0.0176, -0.0114, 0.0205, -0.0052, 0.0097,
         -0.0133, 0.0078, 0.0016, -0.0058, 0.0120],
        [0.0110, -0.0060, 0.0150, -0.0060, 0.0100,
         -0.0110, 0.0070, 0.0010, -0.0060, 0.0110],
        [0.0084, -0.0076, 0.0125, -0.0018, 0.0063,
         -0.0087, 0.0062, 0.0014, -0.0052, 0.0080],
        [0.0034, -0.0016, 0.0020, 0.0012, 0.0008,
         -0.0012, -0.0008, 0.0014, -0.0012, 0.0010],
    ]
    .map(|column| column.to_vec())
    .to_vec();
    let returns = risk::ReturnsTable::new(
        vec![
            "tech".into(),
            "industrials".into(),
            "utilities".into(),
            "bonds".into(),
        ],
        columns,
    )?;
    let market = [
        0.012, -0.008, 0.015, -0.004, 0.009,
        -0.011, 0.006, 0.002, -0.006, 0.010,
    ]
    .to_vec();
    let factors = risk::ReturnsTable::new(
        vec!["market".into()],
        vec![market],
    )?;
    let estimate = risk::regression_model(&returns, &factors)?;
    println!("Method: regression");
    println!("Observations: {}", estimate.observations);
    for (fit, exposures) in estimate
        .fit
        .iter()
        .zip(estimate.model.exposures())
    {
        println!(
            "{}: beta={:.6} alpha={:.6} R2={:.6}",
            fit.asset,
            exposures[0],
            fit.intercept.unwrap(),
            fit.r_squared.unwrap(),
        );
    }
    Ok(())
}
