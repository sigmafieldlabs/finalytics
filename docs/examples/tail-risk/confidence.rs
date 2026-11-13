use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
    for confidence in [0.90, 0.95, 0.99] {
        let result = risk::tail_risk(
            &returns, confidence, risk::TailMethod::Historical,
        )?;
        println!(
            "Confidence {:.2}: VaR={:.6} ES={:.6}",
            confidence, result.value_at_risk, result.expected_shortfall,
        );
    }
    Ok(())
}
