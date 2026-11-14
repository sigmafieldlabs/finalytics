use finalytics::InstrumentId;
use finalytics::prelude::*;
use finalytics::reports::ReportOptions;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut options = ReportOptions::default();
    options.benchmark = Some("BENCH".parse::<InstrumentId>()?.into());
    let result = portfolio.report_with(&options)?;
    println!("{result}");
    for table in result.tables() {
        println!("Table: {} ({} rows)", table.name, table.rows.len());
    }
    Ok(())
}
