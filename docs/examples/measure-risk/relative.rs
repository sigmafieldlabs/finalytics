use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    let benchmark = [
        0.015, -0.010, 0.025, 0.010, -0.030, 0.012, 0.020, -0.005, 0.010,
        0.022, -0.018, 0.011,
    ];
    let mut options = risk::StatisticsOptions::default();
    options.periods_per_year = 12.0;
    options.risk_free_rate = 0.03;
    let stats = risk::relative_statistics(&returns, &benchmark, &options)?;
    println!("Active Return: {:.6}", stats.active_return);
    println!("Tracking Error: {:.6}", stats.tracking_error);
    println!(
        "Information Ratio: {:.6}",
        stats
            .information_ratio
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Beta: {:.6}",
        stats
            .beta
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Alpha: {:.6}",
        stats
            .alpha
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Correlation: {:.6}",
        stats
            .correlation
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Downside Beta: {:.6}",
        stats
            .downside_beta
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Upside Beta: {:.6}",
        stats
            .upside_beta
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Downside Capture: {:.6}",
        stats
            .downside_capture
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Upside Capture: {:.6}",
        stats
            .upside_capture
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
