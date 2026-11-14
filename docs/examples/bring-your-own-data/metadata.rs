use finalytics::prelude::*;

fn main() -> Result<()> {
    let instruments = concat!(
        "id,kind,currency,name,class.asset_class\n",
        "VOO,fund,USD,US stock fund,Equity\n",
    );
    let accounts = concat!(
        "id,name,lot_method,taxable,class.owner\n",
        "main,Brokerage,fifo,false,Alice\n",
    );
    let transactions = concat!(
        "id,account,date,type,instrument,quantity,price,amount,currency\n",
        "t1,main,2026-01-02,deposit,,,,1000,USD\n",
        "t2,main,2026-01-02,buy,VOO,1,500,,\n",
    );
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(transactions)
            .instruments(instruments)
            .accounts(accounts)
            .prices("date,VOO\n2026-01-02,500\n2026-01-05,505\n"),
    )?;
    println!("{}", portfolio.data().instruments[0].name.as_ref().unwrap());
    let classes =
        format!("{:?}", portfolio.data().instruments[0].classifications)
            .replace(": ", ":");
    println!("{}", classes);
    println!(
        "{} {}",
        portfolio.data().accounts[0].name.as_ref().unwrap(),
        portfolio.data().accounts[0].taxable
    );
    Ok(())
}
