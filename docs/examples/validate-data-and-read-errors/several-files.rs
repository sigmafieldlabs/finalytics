use finalytics::prelude::*;

fn main() -> Result<()> {
    let files = PortfolioCsv::new(concat!(
        "id,account,trade_date,type,instrument,quantity,price\n",
        "t1,main,2026-01-02,buy,AAPL,ten,150\n",
    ))
    .instruments("id,kind,currency\nAAPL,equity,usd\n")
    .base_currency("USD");
    match Portfolio::from_csv(&files) {
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
