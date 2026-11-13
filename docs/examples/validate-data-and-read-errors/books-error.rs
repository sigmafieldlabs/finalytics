use finalytics::prelude::*;

fn main() -> Result<()> {
    let files = PortfolioCsv::new(concat!(
        "date,type,instrument,quantity,price,amount,currency\n",
        "2026-01-02,deposit,,,,1000,USD\n",
        "2026-01-02,sell,VOO,2,500,,\n",
    ));
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
