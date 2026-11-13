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
    println!("Assets: {}", model.assets().join(", "));
    println!("Factors: {}", model.factors().join(", "));
    let implied = model.covariance();
    for row in 0..implied.dimension() {
        let values = (0..implied.dimension())
            .map(|column| format!("{:.6}", implied.get(row, column)))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{} {}", implied.labels()[row], values);
    }
    Ok(())
}
