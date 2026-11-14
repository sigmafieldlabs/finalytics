use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    match assets
        .optimise(Method::TargetReturn)
        .max_weight(1.5)
        .build()
    {
        Err(Error::Validation(report)) => {
            for issue in report.issues() {
                println!("{} | {}", issue.path, issue.message);
            }
        }
        other => {
            other?;
        }
    }
    Ok(())
}
