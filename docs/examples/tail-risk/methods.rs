use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
    for (name, method) in [
        ("historical", risk::TailMethod::Historical),
        ("parametric", risk::TailMethod::Parametric),
        ("cornish_fisher", risk::TailMethod::CornishFisher),
    ] {
        let result = risk::tail_risk(&returns, 0.90, method)?;
        println!(
            "{}: VaR={:.6} ES={:.6}",
            name, result.value_at_risk, result.expected_shortfall,
        );
    }
    Ok(())
}
