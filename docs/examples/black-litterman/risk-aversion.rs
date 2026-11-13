use finalytics::{opt, prelude::*};

fn main() -> Result<()> {
    let market_returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
        0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
    ];
    let estimate = opt::implied_risk_aversion(
        &market_returns,
        12.0,
        0.02,
    )?;
    println!("Implied risk aversion: {:.6}", estimate);
    Ok(())
}
