use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let series = portfolio
        .books()
        .account_series(&portfolio.dates(), portfolio.market())?;
    println!("Dates: {}", series.dates.len());
    for line in &series.lines {
        println!(
            "{} {} {}",
            line.account,
            line.values.last().unwrap(),
            line.flows.last().unwrap()
        );
    }
    let total = series.combined(|_| true);
    println!("Household: {}", total.values.last().unwrap());
    Ok(())
}
