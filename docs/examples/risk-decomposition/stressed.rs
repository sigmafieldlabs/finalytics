use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "bonds".into()],
        vec![0.04, 0.01, 0.01, 0.09],
    )?;
    let stress = risk::CorrelationStress {
        volatility_scale: 1.5,
        correlation_shift: 0.5,
    };
    let stressed = risk::stressed_covariance(&covariance, &stress)?;
    for (name, matrix) in [("Before", &covariance), ("After", &stressed)] {
        let result = risk::risk_decomposition(&[0.6, 0.4], matrix)?;
        println!("{}: volatility={:.6}", name, result.volatility);
        for item in &result.contributions {
            println!("  {}: share={:.6}", item.label, item.share.unwrap());
        }
    }
    Ok(())
}
