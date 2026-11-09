//! The trades that move a household's two accounts to the least risky mix of their funds, 5% to
//! 40% each, with fees and tax rates.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();

    let target = portfolio
        .assets()
        .build()?
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.40)
        .build()?;

    let result = portfolio
        .rebalance(&target)
        .whole_units()
        .fee_bps(5.0)
        .minimum_fee(1.0)
        .tax(0.37, 0.20)
        .build()?;

    result.show()?;
    Ok(())
}
