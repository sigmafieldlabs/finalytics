//! The portfolio dataset's funds screened against their benchmark. Run it in a
//! folder that holds the files.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = Universe::builder()
        .prices_file("prices.csv")
        .fields_file("instruments.csv")
        .benchmark("BENCH")
        .risk_free_rate(0.04)
        .build()?;

    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .below("volatility", 0.20)
        .one_of("class.asset_class", ["Equity", "Real assets"])
        .sort("sharpe_ratio", Order::Descending)
        .build()?;

    screen.show()?;
    Ok(())
}
