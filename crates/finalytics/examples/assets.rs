//! The mix of six asset classes with the highest Sharpe ratio, with at most
//! 40% in any one and 15% in real assets, against the return on T-bills.

use finalytics::prelude::*;

fn main() -> Result<()> {
    // Thirty years of month-end levels of six asset-class indices.
    let prices = datasets::asset_class_prices();

    let assets = Assets::builder()
        .prices(prices)
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .group("Real assets", &["Gold", "Real Estate"], 0.0, 0.15)
        .build()?;
    for asset in assets.risky_assets() {
        let weight = result.results.weights.as_ref().map_or(0.0, |w| w[asset]);
        println!("{asset:12} {:3.0}%", weight * 100.0);
    }
    println!(
        "Sharpe ratio {:.2}",
        result.results.sharpe_ratio.unwrap_or_default()
    );
    Ok(())
}
