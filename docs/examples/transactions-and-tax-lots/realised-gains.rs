use finalytics::prelude::*;

fn main() -> Result<()> {
    let transactions = concat!(
        "id,account,trade_date,type,instrument,quantity,price,fees,",
        "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit",
        ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n",
        "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\nt4,main,2026-06-0",
        "1,sell,AAPL,12,190,6,,,highest_cost,\n",
    );
    let portfolio = Portfolio::from_csv(&PortfolioCsv::new(transactions))?;
    for gain in portfolio.books().realised_gains() {
        println!(
            "{} {} {} {} {}",
            gain.lot,
            gain.quantity,
            gain.cost_local,
            gain.proceeds_local,
            gain.gain_local
        );
    }
    Ok(())
}
