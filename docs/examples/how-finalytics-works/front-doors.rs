use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let portfolio = datasets::portfolio();
    let universe = datasets::universe();
    let labels = format!("{:?}", &assets.labels()[..2]).replace('"', "'");
    println!("Assets: {} {}", assets.labels().len(), labels);
    println!(
        "Portfolio: {} {}",
        portfolio.base_currency(),
        portfolio.data().transactions.len()
    );
    println!(
        "Universe: {} {}",
        universe.len(),
        universe
            .benchmark()
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
