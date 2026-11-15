use finalytics::{
    prelude::*,
    risk::{StatisticsOptions, relative_statistics},
};

fn main() -> Result<()> {
    let returns = [0.020, -0.010, 0.015, 0.005, -0.004, 0.012];
    let benchmark = [0.015, -0.008, 0.010, 0.006, -0.003, 0.009];
    let mut options = StatisticsOptions::default();
    options.periods_per_year = 12.0;
    let found = relative_statistics(&returns, &benchmark, &options)?;
    println!(
        "Upside capture: {:.6}",
        found
            .upside_capture
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Downside capture: {:.6}",
        found
            .downside_capture
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Beta: {:.6}",
        found
            .beta
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
