use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    for item in assets.statistics()? {
        let stats = item.statistics;
        let tail = item.tail_risk;
        println!(
            "{}: return={:.6} volatility={:.6} VaR={:.6} ES={:.6}",
            item.asset,
            stats.annualised_return,
            stats.volatility,
            tail.value_at_risk,
            tail.expected_shortfall,
        );
    }
    Ok(())
}
