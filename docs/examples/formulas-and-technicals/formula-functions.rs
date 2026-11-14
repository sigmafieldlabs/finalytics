use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .formula("log_size", "log(market_cap)")
        .formula("value_score", "sqrt(max(trailing_pe, 0)) / price_to_book")
        .formula("risk_score", "abs(return_1y) / max(volatility, 0.01)")
        .below("value_score", 5.0)
        .below("risk_score", 2.0)
        .sort("log_size", Order::Descending)
        .limit(10)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
