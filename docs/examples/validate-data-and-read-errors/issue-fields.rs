use finalytics::prelude::*;

fn main() -> Result<()> {
    let files = PortfolioCsv::new(
        "date,type,amount,currency\nnot-a-date,deposit,1000,USD\n",
    );
    match Portfolio::from_csv(&files) {
        Err(Error::Validation(report)) => {
            for issue in report.issues() {
                let severity = format!("{:?}", issue.severity).to_lowercase();
                println!("{} {}", severity, issue.path);
                println!("{}", issue.message);
            }
        }
        other => {
            other?;
        }
    }
    Ok(())
}
