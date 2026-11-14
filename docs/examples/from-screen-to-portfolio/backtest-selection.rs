use finalytics::PriceTable;
use finalytics::prelude::*;
use finalytics::screening::Series;

fn main() -> Result<()> {
    let csv = datasets::file("portfolio/prices.csv").expect("sample prices");
    let universe = Universe::from_csv(csv)?
        .with_benchmark("BENCH")?
        .with_risk_free_rate(0.04);
    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .sort("sharpe_ratio", Order::Descending)
        .build()?;
    let table = |ids: &[String]| -> Result<PriceTable> {
        let series: Vec<Series> = ids
            .iter()
            .map(|id| universe.series(id).expect("an id is in the universe"))
            .collect();
        let dates = series[0].dates.clone();
        let labels = series.iter().map(|s| s.id.clone()).collect();
        let columns = series
            .iter()
            .map(|s| s.close.iter().map(|&v| Some(v)).collect())
            .collect();
        PriceTable::new(dates, labels, columns)
    };
    let assets = Assets::builder()
        .prices(table(&screen.results.matches)?)
        .risk_free_rate(0.04)
        .build()?;
    let target = assets.optimise(Method::EqualWeight).build()?;
    let result = assets
        .backtest(
            target
                .results
                .weights
                .clone()
                .expect("equal weight finds weights"),
        )
        .benchmark(table(&["BENCH".to_owned()])?)
        .rebalance(Rebalance::Monthly)
        .trading_cost_bps(10.0)
        .name("Screened funds, rebalanced monthly")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
