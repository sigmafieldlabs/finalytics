use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(&PortfolioCsv::new(concat!(
        "id,account,trade_date,type,instrument,quantity,price,fees,",
        "amount,currency,lot_method,lots\n",
        "t1,main,2026-01-02,deposit,,,,,10000,USD,,\n",
        "t2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n",
        "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n",
        "t4,main,2026-06-01,sell,AAPL,5,190,0,,,average_cost,\n",
    )))?;
    for gain in portfolio.books().realised_gains() {
        println!(
            "{} {} {} {}",
            gain.lot, gain.quantity, gain.cost_local, gain.gain_local
        );
    }
    Ok(())
}
