use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .name("Balanced")
        .rebalance(Rebalance::drift(0.05))
        .trading_cost_bps(25.0)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
