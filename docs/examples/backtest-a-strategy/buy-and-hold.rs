use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .backtest([("US Equity", 0.7), ("Gold", 0.3)])
        .name("Buy and hold")
        .rebalance(Rebalance::Never)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
