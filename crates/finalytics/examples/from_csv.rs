//! A portfolio from its CSV files: run it in a folder that holds
//! `transactions.csv` and `prices.csv`.

use finalytics::reports::ReportOptions;
use finalytics::{Portfolio, PortfolioCsv};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transactions = std::fs::read_to_string("transactions.csv")?;
    let prices = std::fs::read_to_string("prices.csv")?;
    let files = PortfolioCsv::new(&transactions).prices(&prices);
    let portfolio = Portfolio::from_csv(&files)?;
    print!("{}", portfolio.holdings_report(&ReportOptions::default())?);
    Ok(())
}
