use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .one_of("sector", ["Utilities"])
        .above("dividend_yield", 0.03)
        .sort("sharpe_ratio", Order::Descending)
        .limit(10)
        .name("Utility income")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
