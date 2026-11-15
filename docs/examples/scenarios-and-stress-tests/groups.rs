use finalytics::{datasets, prelude::*, risk};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut shocks = risk::Shocks::default();
    shocks.groups = vec![
        risk::GroupShock {
            scheme: "asset_class".into(),
            value: "Equity".into(),
            shock: -0.25,
        },
        risk::GroupShock {
            scheme: "asset_class".into(),
            value: "Real assets".into(),
            shock: -0.10,
        },
        risk::GroupShock {
            scheme: "asset_class".into(),
            value: "Fixed income".into(),
            shock: 0.03,
        },
    ];
    let mut options = risk::ScenarioOptions::default();
    options.group_by = vec!["asset_class".into()];
    let result = risk::shock_scenario(
        portfolio.books(),
        portfolio.market(),
        portfolio
            .as_of()
            .expect("the guide's fixed sample supplies this value"),
        &shocks,
        &options,
    )?;
    println!("Value before: {:.2}", result.value_before);
    println!("Value after: {:.2}", result.value_after);
    println!("Portfolio return: {:.6}", result.portfolio_return);
    for group in &result.groups[0].groups {
        println!("{}: contribution={:.6}", group.group, group.contribution);
    }
    Ok(())
}
