use finalytics::prelude::*;

fn main() -> Result<()> {
    let base = concat!(
        "id,account,trade_date,type,instrument,quantity,price,fees,",
        "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit",
        ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n",
        "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n",
    );
    for method in ["fifo", "lifo", "highest_cost", "lowest_cost"] {
        let sale = format!(
            "t4,main,2026-06-01,sell,AAPL,5,190,0,,,{method},"
        );
        let mut lines = base.lines();
        let header = lines.next().unwrap();
        let transactions = format!(
            "{}\n{}\n{}\n",
            header,
            sale,
            lines.collect::<Vec<_>>().join("\n")
        );
        let portfolio = Portfolio::from_csv(&PortfolioCsv::new(&transactions))?;
        let lots = portfolio.books().realised_gains().iter()
            .map(|gain| gain.lot.to_string())
            .collect::<Vec<_>>().join(",");
        println!("{} {}", method, lots);
    }
    Ok(())
}
