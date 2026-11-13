use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets
        .evaluate([("US Equity", 0.60), ("Bonds", 0.40)])
        .build()?;
    println!("Expected return: {:.6}", result.results.expected_return);
    println!("Volatility: {:.6}", result.results.volatility);
    println!("Sharpe ratio: {:.6}", result.results.sharpe_ratio.unwrap());
    Ok(())
}
