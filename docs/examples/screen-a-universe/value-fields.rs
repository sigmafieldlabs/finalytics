use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .between("trailing_pe", 5.0, 25.0)
        .below("price_to_book", 3.0)
        .above("market_cap", 1_000_000_000.0)
        .sort("market_cap", Order::Descending)
        .limit(10)
        .name("Large companies at moderate valuations")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
