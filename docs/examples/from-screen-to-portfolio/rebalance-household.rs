use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let prices = datasets::file("portfolio/prices.csv").expect("sample prices");
    let fields = datasets::file("portfolio/instruments.csv")
        .expect("sample instruments");
    let universe = Universe::from_csv(prices)?
        .with_fields_csv(fields)?
        .with_benchmark("BENCH")?
        .with_risk_free_rate(0.04);
    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .sort("sharpe_ratio", Order::Descending)
        .build()?;
    let weight = 1.0 / screen.results.matches.len() as f64;
    let targets: std::collections::BTreeMap<String, f64> = screen
        .results
        .matches
        .iter()
        .map(|id| (id.clone(), weight))
        .collect();
    let result = portfolio
        .rebalance(targets)
        .whole_units()
        .fee_bps(5.0)
        .tax(0.37, 0.2)
        .name("Rebalance to screened funds")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
