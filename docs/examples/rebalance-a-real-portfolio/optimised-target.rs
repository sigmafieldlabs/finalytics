use finalytics::prelude::*;

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
        .whole_units()
        .fee_bps(5.0)
        .minimum_fee(1.0)
        .tax(0.37, 0.2)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
