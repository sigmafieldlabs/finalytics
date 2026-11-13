use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let minimum = assets.optimise(Method::MinimumVariance).build()?;
    let target = assets.optimise(Method::TargetReturn).target(0.06).build()?;
    let volatility = assets
        .optimise(Method::TargetVolatility)
        .target(0.08)
        .build()?;
    for (name, result) in [
        ("minimum_variance", minimum),
        ("target_return", target),
        ("target_volatility", volatility),
    ] {
        println!(
            "{} {:.6} {:.6}",
            name,
            result.results.expected_return.unwrap(),
            result.results.volatility.unwrap()
        );
    }
    Ok(())
}
