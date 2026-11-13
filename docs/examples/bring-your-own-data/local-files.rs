use finalytics::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let transactions = std::fs::read_to_string("transactions.csv")?;
    let prices = std::fs::read_to_string("prices.csv")?;
    let fx = std::fs::read_to_string("fx.csv")?;
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(&transactions).prices(&prices).fx(&fx),
    )?;
    println!("Transactions: {}", portfolio.data().transactions.len());
    println!("Value: {}", portfolio.valuation()?.total_base);
    Ok(())
}
