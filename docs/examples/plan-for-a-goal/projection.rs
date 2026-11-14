use finalytics::planning::Model;
use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let result = assets
        .plan()
        .projection("2040-12")
        .start_value(10_000.0)
        .saving(500.0, Frequency::Month)
        .weights([("US Equity", 1.0)])
        .inflation(0.02)
        .model(Model::Bootstrap { block: 6.0 })
        .simulations(200)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
