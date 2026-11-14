use finalytics::{performance::Period, prelude::*};

fn main() -> Result<()> {
    let performance = datasets::portfolio().performance()?;
    for (name, period) in [
        ("MTD", Period::MonthToDate),
        ("QTD", Period::QuarterToDate),
        ("YTD", Period::YearToDate),
        ("1Y", Period::Years { years: 1 }),
        ("ITD", Period::SinceInception),
    ] {
        match performance.period(period, None) {
            None => println!("{} insufficient history", name),
            Some(found) => {
                println!("{} {} {:.6}", name, found.start, found.cumulative)
            }
        }
    }
    Ok(())
}
