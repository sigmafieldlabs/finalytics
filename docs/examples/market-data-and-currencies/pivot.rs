use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(
            "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
        )
        .fx("date,EUR/USD,GBP/USD\n2026-06-30,1.10,1.25\n"),
    )?;
    let rate = portfolio.market().fx_rate(
        finalytics::Currency::EUR,
        finalytics::Currency::GBP,
        "2026-06-30".parse()?,
    );
    println!("Pivot: {}", portfolio.market().pivot());
    println!(
        "EUR/GBP: {}",
        rate.value()
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
