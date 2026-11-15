use finalytics::{datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut window = risk::HistoricalWindow::new(
        "2025-02-19".parse()?,
        "2025-04-08".parse()?,
    );
    window.proxies.insert("EXSA".parse()?, "VTI".parse()?);
    let result = risk::historical_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio
            .as_of()
            .expect("the guide's fixed sample supplies this value"),
        &window,
        &risk::ScenarioOptions::default(),
    )?;
    for holding in &result.holdings {
        let name = holding.holding.to_string();
        if name == "VTI" || name == "EXSA" {
            println!(
                "{}: local={:.6}, currency={:.6}, base={:.6}",
                name,
                holding.return_local,
                holding.currency_return,
                holding.return_base,
            );
        }
    }
    Ok(())
}
