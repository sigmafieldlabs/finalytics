use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    let dates = [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
        "2026-08-31",
        "2026-09-30",
        "2026-10-30",
        "2026-11-30",
        "2026-12-31",
    ];
    let window = 6;
    let mut options = risk::StatisticsOptions::default();
    options.periods_per_year = 12.0;
    let windows = risk::rolling_statistics(&returns, window, &options)?;
    for offset in 0..3 {
        let stats = &windows[windows.len() - 3 + offset];
        let end = dates[window - 1 + windows.len() - 3 + offset];
        println!("Ending {}: volatility={:.6}", end, stats.volatility);
    }
    Ok(())
}
