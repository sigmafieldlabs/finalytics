use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let dates = portfolio.dates();
    let performance = portfolio.performance()?;
    let start = *dates.first().unwrap();
    let end = *dates.last().unwrap();
    let holdings = portfolio
        .books()
        .holding_series(&dates, portfolio.market())?;
    let contributions = performance.contributions(&holdings, start, end)?;
    let total: f64 = contributions.iter().map(|c| c.contribution).sum();
    let expenses = performance.expenses(&holdings, start, end)?;
    println!("Expenses: {:.6}", expenses);
    println!("With expenses: {:.6}", total + expenses);
    let start = *performance.dates().first().unwrap();
    let end = *performance.dates().last().unwrap();
    println!(
        "Return: {:.6}",
        performance.time_weighted(start, end).unwrap()
    );
    Ok(())
}
