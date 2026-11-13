use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(concat!(
            "date,type,instrument,quantity,price,amount,currency\n",
            "2026-06-30,deposit,,,,1000,USD\n",
            "2026-06-30,buy,VOO,1,500,,\n",
        ))
        .prices("date,VOO\n2026-06-30,500\n"),
    )?;
    let instrument = finalytics::InstrumentId::new("VOO").unwrap();
    let found = portfolio.market().price(&instrument, "2026-07-03".parse()?);
    println!("Value: {}", found.value().unwrap());
    if let finalytics::Observation::Carried { from, .. } = found {
        println!("Observed: {from}");
        println!("Carried: true");
    }
    let stale = portfolio.market().price(&instrument, "2026-07-08".parse()?);
    println!("Stale: {}", stale.value().is_none());
    Ok(())
}
