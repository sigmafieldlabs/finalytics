use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(concat!(
            "date,type,instrument,quantity,price,amount,currency\n",
            "2026-06-30,deposit,,,,2000,USD\n",
            "2026-06-30,transfer_in,SAP,10,,,\n",
        ))
        .instruments("id,kind,currency\nSAP,equity,EUR\n")
        .prices("date,SAP\n2026-06-30,200\n")
        .fx("date,EUR/USD\n2026-06-30,1.10\n"),
    )?;
    let valuation = portfolio.valuation()?;
    let position = &valuation.positions[0];
    println!("Local: {}", position.market_value_local);
    println!("Base: {}", position.market_value_base);
    Ok(())
}
