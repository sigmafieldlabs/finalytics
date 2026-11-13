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
    let weighted = risk::covariance(
        &returns,
        risk::CovarianceMethod::Ewma { decay: 0.90 },
    )?;
    let statistical = risk::covariance(
        &returns,
        risk::CovarianceMethod::Statistical { factors: 2 },
    )?;
    println!("EWMA equity variance: {:.8}", weighted.matrix.get(0, 0));
    println!("EWMA credit variance: {:.8}", weighted.matrix.get(1, 1));
    println!(
        "Statistical equity variance: {:.8}",
        statistical.matrix.get(0, 0),
    );
    println!(
        "Statistical bond variance: {:.8}",
        statistical.matrix.get(2, 2),
    );
    Ok(())
}
