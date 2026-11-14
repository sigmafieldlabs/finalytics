use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = datasets::universe();
    let result = universe
        .screen()
        .above("from_52w_high", -0.1)
        .above("from_52w_low", 0.1)
        .above("volume_ratio", 1.0)
        .above("dollar_volume_20d", 1_000_000.0)
        .sort("volume_ratio", Order::Descending)
        .limit(10)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
