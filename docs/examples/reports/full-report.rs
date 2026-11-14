use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut result = portfolio.report_with(&datasets::report_options())?;
    result.name = Some("Morgan family portfolio".to_owned());
    println!("{result}");
    result.show()?;
    Ok(())
}
