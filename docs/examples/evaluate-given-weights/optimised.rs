use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let optimised = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .build()?;
    let result = assets
        .evaluate(optimised.results.weights.clone().unwrap())
        .name("Optimised weights")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
