use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::TargetVolatility)
        .target(0.10)
        .max_weight(0.40)
        .build()?;

    println!("{result}");
    Ok(())
}
