use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .optimise(Method::RiskBudgeting)
        .budget("US Equity", 0.30)
        .budget("Intl Equity", 0.20)
        .budget("Bonds", 0.20)
        .budget("Gold", 0.15)
        .budget("Real Estate", 0.15)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
