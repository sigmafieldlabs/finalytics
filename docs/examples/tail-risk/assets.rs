use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let assets = Assets::from_prices(&datasets::asset_class_prices())?;
    for (label, column) in
        assets.labels().iter().zip(assets.returns().columns())
    {
        let result =
            risk::tail_risk(column, 0.95, risk::TailMethod::CornishFisher)?;
        println!(
            "{}: VaR={:.6} ES={:.6} clamped={}",
            label,
            result.value_at_risk,
            result.expected_shortfall,
            result.shortfall_at_var,
        );
    }
    Ok(())
}
