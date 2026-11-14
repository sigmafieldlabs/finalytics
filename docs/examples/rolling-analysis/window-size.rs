use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    for window in [4, 6, 8] {
        let mut options = risk::StatisticsOptions::default();
        options.periods_per_year = 12.0;
        let windows = risk::rolling_statistics(&returns, window, &options)?;
        println!(
            "Window {}: count={} latest_volatility={:.6}",
            window,
            windows.len(),
            windows.last().unwrap().volatility,
        );
    }
    Ok(())
}
