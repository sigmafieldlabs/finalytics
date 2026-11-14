use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .formula("earnings_yield", "1 / trailing_pe")
        .formula("score", "return_1y / volatility")
        .above("earnings_yield", 0.04)
        .above("score", 0.0)
        .sort("score", Order::Descending)
        .limit(10)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
