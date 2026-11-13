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
    let exposures = [
        [1.0, 0.0], [1.0, 0.0],
        [0.0, 1.0], [0.0, 1.0],
    ]
    .map(|row| row.to_vec())
    .to_vec();
    let estimate = risk::fundamental_model(
        &returns,
        &["cyclical".into(), "defensive".into()],
        &exposures,
    )?;
    println!("Method: fundamental");
    for (label, column) in estimate
        .model
        .factors()
        .iter()
        .zip(&estimate.factor_returns)
    {
        println!(
            "{}: first={:.6} last={:.6}",
            label,
            column[0],
            column[column.len() - 1],
        );
    }
    for fit in &estimate.fit {
        println!("{}: R2={:.6}", fit.asset, fit.r_squared.unwrap());
    }
    Ok(())
}
