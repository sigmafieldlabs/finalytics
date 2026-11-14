use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    for target in [0.0, 0.04] {
        let mut options = risk::StatisticsOptions::default();
        options.periods_per_year = 12.0;
        options.target_return = target;
        let stats = risk::statistics(&returns, &options)?;
        println!(
            "Target {:.2}: downside={:.6} Sortino={:.6}",
            target,
            stats.downside_deviation,
            stats.sortino_ratio.unwrap(),
        );
    }
    let constant = risk::statistics(&[0.01; 12], &Default::default())?;
    println!("Constant Sharpe: {:?}", constant.sharpe_ratio);
    println!("Constant Sortino: {:?}", constant.sortino_ratio);
    Ok(())
}
