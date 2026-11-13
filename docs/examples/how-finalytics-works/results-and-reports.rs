use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets.optimise(Method::MinimumVariance).build()?;
    println!(
        "Expected return: {:.6}",
        result.results.expected_return.unwrap()
    );
    println!("Volatility: {:.6}", result.results.volatility.unwrap());
    print!("{result}");
    result.show()?;
    Ok(())
}
