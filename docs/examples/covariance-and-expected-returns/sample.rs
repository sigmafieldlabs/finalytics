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
    let estimate = risk::covariance(&returns, risk::CovarianceMethod::Sample)?;
    println!("Observations: {}", estimate.observations);
    for row in 0..estimate.matrix.dimension() {
        let values = (0..estimate.matrix.dimension())
            .map(|column| format!("{:.8}", estimate.matrix.get(row, column)))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{} {}", estimate.matrix.labels()[row], values);
    }
    let annual = estimate.matrix.scaled(252.0);
    println!("Annual equity variance: {:.8}", annual.get(0, 0));
    Ok(())
}
