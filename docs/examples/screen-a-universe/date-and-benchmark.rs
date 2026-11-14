use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .as_of("2025-12-31")
        .measures(["last_price", "return_1y", "beta"])
        .above("return_1y", 0.0)
        .sort("beta", Order::Ascending)
        .include_benchmark()
        .limit(10)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
