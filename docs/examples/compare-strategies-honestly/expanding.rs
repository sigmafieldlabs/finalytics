use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;
    let result = assets
        .compare([Method::MinimumVariance, Method::MaximumDiversification])
        .walk_forward_expanding(Years(10))
        .rebalance(Rebalance::Yearly)
        .trading_cost_bps(10.0)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
