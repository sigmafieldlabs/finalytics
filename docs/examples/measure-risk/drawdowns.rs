use finalytics::{TimeSeries, prelude::*, risk};

fn main() -> Result<()> {
    let dates = [
        "2026-01-02",
        "2026-01-05",
        "2026-01-06",
        "2026-01-07",
        "2026-01-08",
        "2026-01-09",
    ]
    .iter()
    .map(|d| d.parse().expect("sample dates are valid"))
    .collect();
    let levels =
        TimeSeries::new(dates, vec![100.0, 95.0, 101.0, 90.0, 92.0, 104.0])?;
    for item in risk::drawdowns(&levels)? {
        let recovery = item.recovery.map_or("open".into(), |d| d.to_string());
        let days = item
            .days_to_recovery
            .map_or("None".into(), |d| d.to_string());
        println!(
            "{} {} {} {:.6} {} {}",
            item.peak,
            item.trough,
            recovery,
            item.depth,
            item.days_to_trough,
            days,
        );
    }
    Ok(())
}
