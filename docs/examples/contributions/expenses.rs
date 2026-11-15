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
    let total: f64 = contributions.iter().map(|c| c.contribution).sum();
    let expenses = performance.expenses(&holdings, start, end)?;
    println!("Expenses: {:.6}", expenses);
    println!("With expenses: {:.6}", total + expenses);
    let start = *performance
        .dates()
        .first()
        .expect("the guide's fixed sample supplies this value");
    let end = *performance
        .dates()
        .last()
        .expect("the guide's fixed sample supplies this value");
    println!(
        "Return: {:.6}",
        performance
            .time_weighted(start, end)
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
