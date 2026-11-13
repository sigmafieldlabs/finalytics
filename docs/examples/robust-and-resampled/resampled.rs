use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.6)
        .resample(50)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
