use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let dates = portfolio.dates();
    let start = *dates
        .first()
        .expect("the guide's fixed sample supplies this value");
    let end = *dates
        .last()
        .expect("the guide's fixed sample supplies this value");
    let performance = portfolio.performance()?;
    let holdings = portfolio
        .books()
        .holding_series(&dates, portfolio.market())?;
    let contributions = performance.contributions(&holdings, start, end)?;
    for item in contributions {
        if item.holding.to_string() == "VTI" {
            println!("Holding: {}", item.holding);
            println!("Contribution: {:.6}", item.contribution);
            println!(
                "Own return: {:.6}",
                item.return_base
                    .expect("the guide's fixed sample supplies this value")
            );
        }
    }
    println!("Events: {}", portfolio.books().events().len());
    Ok(())
}
