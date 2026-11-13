use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    match assets.optimise(Method::TargetReturn).build() {
        Err(Error::Validation(report)) => {
            println!("ValidationError");
            println!("Issues: {}", report.issues().len());
        }
        other => {
            other?;
        }
    }
    Ok(())
}
