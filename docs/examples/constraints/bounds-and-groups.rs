use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MaximumSharpe)
        .bounds(0.0, 0.4)
        .asset_bounds("Gold", 0.0, 0.1)
        .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
