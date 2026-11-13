use finalytics::{OptimiseOptions, prelude::*};

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let mut options = OptimiseOptions::default();
    options.max_weight = Some(0.50);
    for point in assets.frontier(5, &options)? {
        println!("{:.6} {:.6}", point.expected_return, point.volatility);
    }
    Ok(())
}
