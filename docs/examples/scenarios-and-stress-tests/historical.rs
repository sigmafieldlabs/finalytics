use finalytics::{datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let window = risk::HistoricalWindow::new(
        "2025-02-19".parse()?,
        "2025-04-08".parse()?,
    );
    let result = risk::historical_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio
            .as_of()
            .expect("the guide's fixed sample supplies this value"),
        &window,
        &risk::ScenarioOptions::default(),
    )?;
    let path = result
        .path
        .as_ref()
        .expect("the guide's fixed sample supplies this value");
    let drawdown = path
        .worst_drawdown
        .as_ref()
        .expect("the guide's fixed sample supplies this value");
    println!("Value before: {:.2}", result.value_before);
    println!("Value after: {:.2}", result.value_after);
    println!("Portfolio return: {:.6}", result.portfolio_return);
    println!("Path observations: {}", path.dates.len());
    println!("Path start: {:.2}", path.values[0]);
    println!("Path end: {:.2}", path.values[path.values.len() - 1]);
    println!("Worst drawdown: {:.6}", drawdown.depth);
    println!("Trough: {}", drawdown.trough);
    Ok(())
}
