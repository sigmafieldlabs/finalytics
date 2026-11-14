use finalytics::prelude::*;
use finalytics::reports::ReportOptions;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let target = portfolio
        .assets()
        .build()?
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.4)
        .build()?;
    let result = portfolio
        .rebalance(&target)
        .keep_all_cash()
        .whole_units()
        .build()?;
    println!("{result}");
    result.show()?;

    let after = portfolio
        .with_rebalance(&result)?
        .report_with(&ReportOptions::default())?;
    println!("{after}");
    Ok(())
}
