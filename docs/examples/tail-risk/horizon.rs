use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    let monthly =
        risk::tail_risk(&returns, 0.95, risk::TailMethod::Parametric)?;
    let root_time = 12.0_f64.sqrt();
    println!("Monthly VaR: {:.6}", monthly.value_at_risk);
    println!("Monthly ES: {:.6}", monthly.expected_shortfall);
    println!(
        "Root-time annual VaR: {:.6}",
        monthly.value_at_risk * root_time,
    );
    println!(
        "Root-time annual ES: {:.6}",
        monthly.expected_shortfall * root_time,
    );
    Ok(())
}
