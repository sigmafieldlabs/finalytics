//! The maximum Sharpe ratio, fitted to the years before November 2016 and held from then on
//! against a 60/40 benchmark.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;

    let result = assets
        .backtest(Method::MaximumSharpe)
        .max_weight(0.40)
        .benchmark([("US Equity", 0.60), ("Bonds", 0.40)])
        .rebalance(Rebalance::Quarterly)
        .holdout_from("2016-11-01")
        .build()?;

    result.show()?;
    Ok(())
}
