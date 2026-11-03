//! The weights with the largest Sharpe ratio, at most 40% in any asset class, with the
//! efficient frontier.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;

    let result = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.40)
        .with_frontier()
        .build()?;

    result.show()?;
    Ok(())
}
