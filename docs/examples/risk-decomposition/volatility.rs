use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0100, -0.0060, 0.0100, 0.0900, -0.0090, -0.0060, -0.0090,
            0.0100,
        ],
    )?;
    let result = risk::risk_decomposition(&[0.5, 0.3, 0.2], &covariance)?;
    println!("Volatility: {:.6}", result.volatility);
    for item in &result.contributions {
        println!(
            "{}: marginal={:.6} component={:.6} share={:.6}",
            item.label,
            item.marginal
                .expect("the guide's fixed sample supplies this value"),
            item.component,
            item.share
                .expect("the guide's fixed sample supplies this value"),
        );
    }
    let total: f64 = result.contributions.iter().map(|c| c.component).sum();
    let shares: f64 = result.contributions.iter().filter_map(|c| c.share).sum();
    println!("Component sum: {:.6}", total);
    println!("Share sum: {:.6}", shares);
    Ok(())
}
