use finalytics::{
    performance::{FlowTiming, Performance, Period},
    prelude::*,
};

fn main() -> Result<()> {
    let performance = Performance::from_parts(
        vec!["2023-03-31".parse()?, "2026-03-31".parse()?],
        vec![100.0, 133.1],
        vec![0.0, 0.0],
        FlowTiming::StartOfDay,
    )?;
    let found = performance
        .period(Period::Years { years: 3 }, None)
        .unwrap();
    println!("Cumulative: {:.6}", found.cumulative);
    println!("Annualised: {:.6}", found.annualised.unwrap());
    let short = datasets::portfolio()
        .performance()?
        .period(Period::Years { years: 1 }, None);
    println!("One year: {:.6}", short.unwrap().cumulative);
    Ok(())
}
