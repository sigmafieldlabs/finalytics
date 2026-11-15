use finalytics::{
    performance::{FlowTiming, Performance, Period},
    prelude::*,
};

fn main() -> Result<()> {
    let performance = Performance::from_parts(
        vec!["2025-12-31".parse()?, "2026-03-31".parse()?],
        vec![1000.0, 1070.323],
        vec![0.0, 0.0],
        FlowTiming::StartOfDay,
    )?;
    let benchmark = finalytics::TimeSeries::new(
        vec!["2025-12-31".parse()?, "2026-03-31".parse()?],
        vec![100.0, 104.0],
    )?;
    let excess = performance
        .excess_return(Period::SinceInception, None, &benchmark, 5)
        .expect("the guide's fixed sample supplies this value");
    println!("Portfolio: {:.6}", excess.portfolio);
    println!("Benchmark: {:.6}", excess.benchmark);
    println!("Arithmetic: {:.6}", excess.arithmetic);
    println!("Geometric: {:.6}", excess.geometric);
    Ok(())
}
