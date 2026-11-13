use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
    let benchmark = [
    0.015, -0.010, 0.025, 0.010, -0.030, 0.012,
    0.020, -0.005, 0.010, 0.022, -0.018, 0.011,
];
    let mut options = risk::StatisticsOptions::default();
    options.periods_per_year = 12.0;
    let windows = risk::rolling_relative_statistics(
        &returns, &benchmark, 6, &options,
    )?;
    for (index, stats) in windows.iter().enumerate() {
        println!(
            "Window {}: active={:.6} TE={:.6} beta={:.6} correlation={:.6}",
            index,
            stats.active_return,
            stats.tracking_error,
            stats.beta.unwrap(),
            stats.correlation.unwrap(),
        );
    }
    Ok(())
}
