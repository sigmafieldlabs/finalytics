use finalytics::prelude::*;

fn main() -> Result<()> {
    let transactions = concat!(
        "id,date,type,instrument,new_instrument,quantity,price,from",
        ",to,ratio,cost_allocation,cash_per_unit,amount,currency\nt1",
        ",2026-01-02,deposit,,,,,,,,,,10000,USD\nt2,2026-01-02,buy,A",
        "AA,,10,100,,,,,,,\nt3,2026-06-01,merger,AAA,BBB,,,,,0.5,,10",
        ",,\n",
    );
    let portfolio = Portfolio::from_csv(&PortfolioCsv::new(transactions))?;
    for lot in portfolio.books().holdings().lots() {
        println!(
            "{} {} {} {}",
            lot.id, lot.instrument, lot.quantity, lot.cost_local
        );
    }
    for gain in portfolio.books().realised_gains() {
        println!("Gain: {} {}", gain.lot, gain.gain_local);
    }
    Ok(())
}
