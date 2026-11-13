use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(
            "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
        )
        .fx("date,EUR/USD\n2026-06-30,1.10\n"),
    )?;
    let rate = portfolio.market().fx_rate(
        finalytics::Currency::EUR,
        finalytics::Currency::USD,
        "2026-06-30".parse()?,
    );
    println!("EUR/USD: {:.2}", rate.value().unwrap());
    println!(
        "100 EUR in USD: {:.2}",
        rate.value().unwrap() * finalytics::Decimal::from(100)
    );
    Ok(())
}
