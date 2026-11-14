use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    for (name, method) in [
        ("historical", risk::MeanMethod::Historical),
        ("james_stein", risk::MeanMethod::JamesStein),
        ("ewma", risk::MeanMethod::Ewma { decay: 0.97 }),
    ] {
        let assets = Assets::builder()
            .prices(datasets::asset_class_prices())
            .risk_free_asset("T-Bills")
            .mean_returns(method)
            .build()?;
        println!("{}", name);
        for (label, value) in assets.labels().iter().zip(assets.mean_returns())
        {
            println!("  {}: {:.6}", label, value);
        }
    }
    Ok(())
}
