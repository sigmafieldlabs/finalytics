use finalytics::prelude::*;

fn main() -> Result<()> {
    let transactions = concat!(
        "id,account,trade_date,type,instrument,quantity,price,fees,",
        "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit",
        ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n",
        "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n",
    );
    let portfolio = Portfolio::from_csv(&PortfolioCsv::new(transactions))?;
    for (name, method) in [
        ("fifo", finalytics::LotMethod::Fifo),
        ("highest_cost", finalytics::LotMethod::HighestCost),
    ] {
        let lots = portfolio.books().holdings().lots_for_sale(
            &finalytics::AccountId::new("main")
                .expect("the guide's fixed sample supplies this value"),
            &finalytics::InstrumentId::new("AAPL")
                .expect("the guide's fixed sample supplies this value"),
            finalytics::Decimal::from(5),
            method,
        )?;
        let text = lots
            .iter()
            .map(|lot| format!("{}:{}", lot.lot, lot.quantity))
            .collect::<Vec<_>>()
            .join(",");
        println!("{} {}", name, text);
    }
    println!("Open lots: {}", portfolio.books().holdings().lots().count());
    Ok(())
}
