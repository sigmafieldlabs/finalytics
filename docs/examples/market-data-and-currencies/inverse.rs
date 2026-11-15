use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::from_csv(
        &PortfolioCsv::new(
            "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
        )
        .fx("date,USD/EUR\n2026-06-30,0.80\n"),
    )?;
    let rate = portfolio.market().fx_rate(
        finalytics::Currency::EUR,
        finalytics::Currency::USD,
        "2026-06-30".parse()?,
    );
    println!(
        "{}",
        rate.value()
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
