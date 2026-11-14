use finalytics::prelude::*;

fn main() -> Result<()> {
    let transactions = concat!(
        "date,type,instrument,quantity,price,amount,currency\n",
        "2026-01-02,deposit,,,,1000,USD\n",
        "2026-01-02,buy,VOO,1,500,,\n",
    );
    let prices = concat!(
        "date,instrument,price\n",
        "2026-01-02,VOO,500\n",
        "2026-01-05,VOO,505\n",
    );
    let portfolio =
        Portfolio::from_csv(&PortfolioCsv::new(transactions).prices(prices))?;
    println!("{}", portfolio.valuation()?.total_base);
    Ok(())
}
