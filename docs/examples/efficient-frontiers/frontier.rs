use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let frontier = assets
        .optimise(Method::MinimumVariance)
        .max_weight(0.5)
        .frontier(6)?;
    for point in frontier {
        println!(
            "return={:.6}, volatility={:.6}",
            point.expected_return, point.volatility,
        );
    }
    Ok(())
}
