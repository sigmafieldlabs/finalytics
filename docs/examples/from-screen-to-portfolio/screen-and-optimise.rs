use finalytics::PriceTable;
use finalytics::prelude::*;
use finalytics::screening::Series;

fn main() -> Result<()> {
    let csv = datasets::file("portfolio/prices.csv").expect("sample prices");
    let fields = datasets::file("portfolio/instruments.csv")
        .expect("sample instruments");
    let universe = Universe::from_csv(csv)?
        .with_fields_csv(fields)?
        .with_benchmark("BENCH")?
        .with_risk_free_rate(0.04);
    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .sort("sharpe_ratio", Order::Descending)
        .build()?;
    println!("{screen}");
    screen.show()?;

    let series: Vec<Series> = screen
        .results
        .matches
        .iter()
        .map(|id| universe.series(id).expect("a match is in the universe"))
        .collect();
    let dates = series[0].dates.clone();
    let labels = series.iter().map(|s| s.id.clone()).collect();
    let columns = series
        .iter()
        .map(|s| s.close.iter().map(|&v| Some(v)).collect())
        .collect();
    let prices = PriceTable::new(dates, labels, columns)?;
    let assets = Assets::builder()
        .prices(prices)
        .risk_free_rate(0.04)
        .build()?;
    let result = assets
        .optimise(Method::EqualWeight)
        .name("Screened funds")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
