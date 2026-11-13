use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let frontier = assets
        .optimise(Method::MinimumVariance)
        .bounds(0.0, 0.5)
        .group("Equity", &["US Equity", "Intl Equity"], 0.2, 0.7)
        .cash(0.0, 0.2)
        .frontier(5)?;
    for point in frontier {
        println!(
            "return={:.6}, volatility={:.6}",
            point.expected_return,
            point.volatility,
        );
        for (asset, weight) in &point.weights {
            println!("  {}: {:.6}", asset, weight);
        }
    }
    Ok(())
}
