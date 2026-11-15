use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets.optimise(Method::MinimumVariance).build()?;
    println!(
        "Expected return: {:.6}",
        result
            .results
            .expected_return
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Volatility: {:.6}",
        result
            .results
            .volatility
            .expect("the guide's fixed sample supplies this value")
    );
    print!("{result}");
    result.show()?;
    Ok(())
}
