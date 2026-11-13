use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let classes = portfolio
        .data()
        .instruments
        .iter()
        .filter(|i| i.classifications.contains_key("asset_class"))
        .map(|i| {
            (
                i.id.to_string(),
                i.classifications["asset_class"].clone(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let dates = portfolio.dates();
    let start = *dates.first().unwrap();
    let end = *dates.last().unwrap();
    let performance = portfolio.performance()?;
    let holdings = portfolio
        .books()
        .holding_series(&dates, portfolio.market())?;
    let contributions = performance.contributions(&holdings, start, end)?;
    let groups = finalytics::performance::group_contributions(
        &contributions,
        |key| match key {
            finalytics::HoldingKey::Cash(_) => "Cash".to_string(),
            finalytics::HoldingKey::Instrument(id) => {
                classes[&id.to_string()].clone()
            }
        },
    );
    for item in groups {
        println!("{} {:.6}", item.group, item.contribution);
    }
    Ok(())
}
