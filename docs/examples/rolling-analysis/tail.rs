use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    let windows = risk::rolling_tail_risk(
        &returns,
        6,
        0.90,
        risk::TailMethod::Historical,
    )?;
    for (index, result) in windows.iter().enumerate() {
        println!(
            "Window {}: VaR={:.6} ES={:.6}",
            index, result.value_at_risk, result.expected_shortfall,
        );
    }
    Ok(())
}
