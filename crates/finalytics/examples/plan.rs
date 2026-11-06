//! A retirement plan: saving until November 2046, then an income of $4,000 a
//! month for 30 years, in today's money.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;

    let plan = assets
        .plan()
        .payments(4_000.0, "2046-11", "2076-10")
        .name("Retirement income")
        .chance(0.85)
        .start_value(250_000.0)
        .saving(1_500.0, Frequency::Month)
        .weights([("US Equity", 0.60), ("Bonds", 0.40)])
        .mix_name("Balanced 60/40")
        .alternative("Cautious 30/70", [("US Equity", 0.30), ("Bonds", 0.70)])
        .alternative("Growth 80/20", [("US Equity", 0.80), ("Bonds", 0.20)])
        .inflation(0.025)
        .fee(0.0025)
        .currency("USD")
        .build()?;

    plan.show()?;
    Ok(())
}
