use finalytics::prelude::*;
use finalytics::reports::ReportOptions;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut options = ReportOptions::default();
    options.group_by = vec!["asset_class".into(), "region".into()];
    let result = portfolio.holdings_report(&options)?;
    println!("{result}");
    let source = &result.provenance;
    println!("Report: {}", format!("{:?}", source.report).to_lowercase());
    println!("As of: {}", source.as_of);
    println!("Base currency: {}", source.base_currency);
    println!("Source digest: {}", source.source.digest);
    Ok(())
}
