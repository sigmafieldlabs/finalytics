use finalytics::planning::Frequency;
use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets
        .plan()
        .target(150_000.0, "2030-06")
        .name("House deposit")
        .start_value(60_000.0)
        .saving(2_500.0, Frequency::Month)
        .spending_on("Car", 25_000.0, "2028-03")
        .weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .inflation(0.025)
        .fee(0.0025)
        .simulations(1_000)
        .currency("USD")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
