use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .compare([Method::MinimumVariance, Method::MaximumSharpe])
        .weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])
        .holdout_from("2016-11-01")
        .rebalance(Rebalance::Quarterly)
        .trading_cost_bps(10.0)
        .max_weight(0.6)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
