use finalytics::prelude::*;

fn main() -> Result<()> {
    let series = datasets::portfolio().value_series()?;
    println!("Dates: {}", series.dates.len());
    for k in series.dates.len() - 3..series.dates.len() {
        println!(
            "{} {} {}",
            series.dates[k], series.values[k], series.flows[k]
        );
    }
    Ok(())
}
