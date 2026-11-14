use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0100, 0.0000, 0.0100, 0.0900, 0.0000, 0.0000, 0.0000,
            0.0025,
        ],
    )?;
    let result = covariance.correlation()?;
    for row in 0..result.dimension() {
        let values = (0..result.dimension())
            .map(|column| format!("{:.6}", result.get(row, column)))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{} {}", result.labels()[row], values);
    }
    Ok(())
}
