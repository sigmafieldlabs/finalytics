//! The mix of six asset classes with the highest Sharpe ratio, with at most
//! 40% in any one and 15% in real assets, against the return on T-bills.

use finalytics::{Assets, datasets};

fn main() -> Result<(), finalytics::Error> {
    // Thirty years of month-end levels of six asset-class indices.
    let prices = datasets::asset_class_prices();

    let assets = Assets::from_prices(&prices)?.with_risk_free_asset("T-Bills")?;
    let result = assets
        .optimiser()
        .maximum_sharpe()
        .bounds(0.0, 0.4)
        .group("Real assets", &["Gold", "Real Estate"], 0.0, 0.15)
        .solve()?;
    for asset in assets.risky_assets() {
        let weight = result.weights.as_ref().map_or(0.0, |w| w[asset]);
        println!("{asset:12} {:3.0}%", weight * 100.0);
    }
    println!(
        "Sharpe ratio {:.2}",
        result.sharpe_ratio.unwrap_or_default()
    );
    Ok(())
}
