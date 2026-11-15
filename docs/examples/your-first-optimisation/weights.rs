use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets.optimise(Method::MinimumVariance).build()?;
    for (name, weight) in result
        .results
        .weights
        .as_ref()
        .expect("the guide's fixed sample supplies this value")
    {
        println!("{} {:.6}", name, weight);
    }
    println!("Status: {}", result.results.status);
    Ok(())
}
