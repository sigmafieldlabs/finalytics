use finalytics::{Currency, datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut shocks = risk::Shocks::default();
    shocks.groups = vec![risk::GroupShock {
        scheme: "asset_class".into(),
        value: "Equity".into(),
        shock: -0.20,
    }];
    shocks.currencies.insert(Currency::EUR, -0.10);
    let result = risk::shock_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio
            .as_of()
            .expect("the guide's fixed sample supplies this value"),
        &shocks,
        &risk::ScenarioOptions::default(),
    )?;
    for holding in &result.holdings {
        if holding.return_local != 0.0 || holding.currency_return != 0.0 {
            println!(
                "{}: local={:.6}, currency={:.6}, base={:.6}",
                holding.holding,
                holding.return_local,
                holding.currency_return,
                holding.return_base,
            );
        }
    }
    Ok(())
}
