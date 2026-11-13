use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .evaluate([("US Equity", 0.6), ("Bonds", 0.4)])
        .max_weight(0.5)
        .with_frontier()
        .name("Sixty forty")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
