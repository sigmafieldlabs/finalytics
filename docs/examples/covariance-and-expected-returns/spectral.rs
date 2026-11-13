use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let assets = Assets::from_prices(&datasets::asset_class_prices())?;
    for (name, method) in [
        ("denoised", risk::CovarianceMethod::Denoised),
        (
            "nonlinear_shrinkage",
            risk::CovarianceMethod::NonlinearShrinkage,
        ),
    ] {
        let estimate = risk::covariance(assets.returns(), method)?;
        println!(
            "{}: observations={} first_variance={:.10}",
            name,
            estimate.observations,
            estimate.matrix.get(0, 0),
        );
    }
    Ok(())
}
