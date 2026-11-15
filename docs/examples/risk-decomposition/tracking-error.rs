use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0100, -0.0060, 0.0100, 0.0900, -0.0090, -0.0060, -0.0090,
            0.0100,
        ],
    )?;
    let result = risk::tracking_error_decomposition(
        &[0.6, 0.2, 0.2],
        &[0.4, 0.4, 0.2],
        &covariance,
    )?;
    println!("Tracking error: {:.6}", result.volatility);
    for item in &result.contributions {
        println!(
            "{}: active_weight={:.6} component={:.6} share={:.6}",
            item.label,
            item.weight,
            item.component,
            item.share
                .expect("the guide's fixed sample supplies this value"),
        );
    }
    Ok(())
}
