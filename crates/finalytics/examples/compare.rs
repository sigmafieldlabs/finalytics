//! Six optimisation methods compared walk-forward on 30 years of six asset
//! classes.

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
            Method::MaximumDiversification,
            Method::RiskParity,
            Method::HierarchicalRiskParity,
        ])
        .max_weight(0.40)
        .rebalance(Rebalance::Quarterly)
        .walk_forward(Years(5))
        .trading_cost_bps(10.0)
        .build()?;

    result.show()?;
    Ok(())
}
