use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
        .start_value(10_000.0)
        .flow("2020-01-31", 1_000.0)
        .flow("2021-06-15", -500.0)
        .rebalance(Rebalance::Yearly)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
