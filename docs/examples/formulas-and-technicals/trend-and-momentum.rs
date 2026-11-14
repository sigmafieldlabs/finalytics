use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .above("price_to_sma_200", 1.0)
        .at_least("sma_50_above_200", 1.0)
        .between("rsi_14", 40.0, 70.0)
        .above("macd_histogram", 0.0)
        .sort("return_6m", Order::Descending)
        .limit(10)
        .name("Positive trend without extreme RSI")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
