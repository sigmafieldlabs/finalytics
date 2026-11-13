use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let dates = portfolio.dates();
    let start = *dates.first().unwrap();
    let end = *dates.last().unwrap();
    let performance = portfolio.performance()?;
    let holdings = portfolio
        .books()
        .holding_series(&dates, portfolio.market())?;
    let contributions = performance.contributions(&holdings, start, end)?;
    for item in contributions {
        if item.holding.to_string() == "EXSA" {
            println!("Local: {:.6}", item.return_local.unwrap());
            println!("Base: {:.6}", item.return_base.unwrap());
            println!("Currency effect: {:.6}", item.currency_effect.unwrap());
        }
    }
    Ok(())
}
