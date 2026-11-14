use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let options = datasets::report_options();
    println!("{}", portfolio.overview_report(&options)?);
    println!("{}", portfolio.holdings_report(&options)?);
    println!("{}", portfolio.performance_report(&options)?);
    println!("{}", portfolio.risk_report(&options)?);
    println!("{}", portfolio.attribution_report(&options)?);
    println!("{}", portfolio.scenarios_report(&options)?);
    Ok(())
}
