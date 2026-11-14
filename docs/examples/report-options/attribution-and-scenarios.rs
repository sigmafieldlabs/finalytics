use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut options = datasets::report_options();
    options.start = Some("2025-01-01".parse()?);
    let mut result = portfolio.report_with(&options)?;
    result.name = Some("Attribution and scenarios since 2025".to_owned());
    println!("{result}");
    result.show()?;
    Ok(())
}
