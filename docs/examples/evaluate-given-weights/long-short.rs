use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let weights = [
        ("US Equity", 1.2),
        ("Intl Equity", -0.3),
        ("Bonds", 0.3),
        ("Gold", -0.1),
        ("Real Estate", -0.1),
    ];
    let result = assets
        .evaluate(weights)
        .name("Long short evaluation")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
