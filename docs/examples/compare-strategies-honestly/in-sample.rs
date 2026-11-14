use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .compare([
            Method::EqualWeight,
            Method::MinimumVariance,
            Method::MaximumSharpe,
        ])
        .max_weight(0.4)
        .name("In-sample illustration")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
