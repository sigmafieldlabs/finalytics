use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MinimumVariance)
        .min_effective_assets(3.5)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
