use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
    let mut options = risk::StatisticsOptions::default();
    options.periods_per_year = 12.0;
    options.risk_free_rate = 0.03;
    let windows = risk::rolling_statistics(&returns, 6, &options)?;
    println!("Windows: {}", windows.len());
    for index in [0, windows.len() - 1] {
        let stats = &windows[index];
        println!(
            "Window {}: return={:.6} volatility={:.6} Sharpe={:.6}",
            index,
            stats.annualised_return,
            stats.volatility,
            stats.sharpe_ratio.unwrap(),
        );
    }
    Ok(())
}
