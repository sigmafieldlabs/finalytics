use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .compare([
            Method::EqualWeight,
            Method::MinimumVariance,
            Method::MaximumSharpe,
        ])
        .walk_forward(Years(5))
        .rebalance(Rebalance::Quarterly)
        .trading_cost_bps(10.0)
        .max_weight(0.4)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
