use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let current = [
        ("US Equity", 0.40),
        ("Intl Equity", 0.10),
        ("Bonds", 0.30),
        ("Gold", 0.10),
        ("Real Estate", 0.05),
        ("T-Bills", 0.05),
    ];
    let result = assets
        .optimise(Method::MaximumSharpe)
        .current_weights(current)
        .max_turnover(0.15)
        .min_trade(0.02)
        .max_trades(3)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
