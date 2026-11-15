use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let returns = [
        0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012,
        0.031, -0.024, 0.015,
    ];
    let mut options = risk::StatisticsOptions::default();
    options.periods_per_year = 12.0;
    options.risk_free_rate = 0.03;
    options.target_return = 0.02;
    let stats = risk::statistics(&returns, &options)?;
    println!("Observations: {}", stats.observations);
    println!("Total return: {:.6}", stats.total_return);
    println!("Annualised return: {:.6}", stats.annualised_return);
    println!("Volatility: {:.6}", stats.volatility);
    println!("Downside deviation: {:.6}", stats.downside_deviation);
    println!(
        "Sharpe ratio: {:.6}",
        stats
            .sharpe_ratio
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Sortino ratio: {:.6}",
        stats
            .sortino_ratio
            .expect("the guide's fixed sample supplies this value")
    );
    println!("Maximum drawdown: {:.6}", stats.max_drawdown);
    println!("Ulcer index: {:.6}", stats.ulcer_index);
    println!(
        "Skewness: {:.6}",
        stats
            .skewness
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Excess kurtosis: {:.6}",
        stats
            .excess_kurtosis
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
