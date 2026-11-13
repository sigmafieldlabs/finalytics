use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MinimumCvar)
        .confidence(0.95)
    .target(0.06)
    .max_weight(0.5)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
