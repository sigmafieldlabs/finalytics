use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .measures([
            "return_1y",
            "volatility",
            "max_drawdown",
            "sharpe_ratio",
            "relative_return_1y",
        ])
        .above("max_drawdown", -0.25)
        .sort("sharpe_ratio", Order::Descending)
        .limit(5)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
