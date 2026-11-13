use finalytics::prelude::*;

fn main() -> Result<()> {
    let transactions = concat!(
        "date,type,instrument,quantity,price,amount,currency\n",
        "2026-01-02,deposit,,,,20000,USD\n",
        "2026-01-02,buy,VOO,20,500,,\n",
        "2026-01-02,buy,AGG,100,98,,\n",
    );
    let prices = concat!(
        "date,VOO,AGG\n",
        "2026-01-02,500,98\n",
        "2026-01-05,505,98.2\n",
    );
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(transactions).prices(prices),
    )?;
    println!("Base currency: {}", portfolio.base_currency());
    println!("Value: {}", portfolio.valuation()?.total_base);
    Ok(())
}
