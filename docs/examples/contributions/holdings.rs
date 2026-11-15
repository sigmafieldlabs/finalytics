use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let dates = portfolio.dates();
    let performance = portfolio.performance()?;
    let start = *dates
        .first()
        .expect("the guide's fixed sample supplies this value");
    let end = *dates
        .last()
        .expect("the guide's fixed sample supplies this value");
    let holdings = portfolio
        .books()
        .holding_series(&dates, portfolio.market())?;
    let contributions = performance.contributions(&holdings, start, end)?;
    let mut total = 0.0;
    for item in contributions {
        println!("{} {:.6}", item.holding, item.contribution);
        total += item.contribution;
    }
    println!("Holdings total: {:.6}", total);
    let start = *performance
        .dates()
        .first()
        .expect("the guide's fixed sample supplies this value");
    let end = *performance
        .dates()
        .last()
        .expect("the guide's fixed sample supplies this value");
    println!(
        "Portfolio return: {:.6}",
        performance
            .time_weighted(start, end)
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
