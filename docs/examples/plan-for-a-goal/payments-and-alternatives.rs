use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets
        .plan()
        .payments(4_000.0, "2046-11", "2066-10")
        .name("Retirement income")
        .start_value(250_000.0)
        .saving(1_500.0, Frequency::Month)
        .weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .mix_name("Balanced 60/40")
        .alternative("Cautious 30/70", [("US Equity", 0.3), ("Bonds", 0.7)])
        .alternative("Growth 80/20", [("US Equity", 0.8), ("Bonds", 0.2)])
        .inflation(0.025)
        .chance(0.85)
        .spare(0.9)
        .simulations(500)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
