use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::MaximumSharpe)
        .benchmark_weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .max_tracking_error(0.08)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
