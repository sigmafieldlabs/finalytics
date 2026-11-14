use finalytics::{
    performance::{FlowTiming, Performance},
    prelude::*,
};

fn main() -> Result<()> {
    for (name, timing) in [
        ("start_of_day", FlowTiming::StartOfDay),
        ("end_of_day", FlowTiming::EndOfDay),
    ] {
        let performance = Performance::from_parts(
            vec!["2026-01-30".parse()?, "2026-02-27".parse()?],
            vec![1050.0, 1600.0],
            vec![0.0, 500.0],
            timing,
        )?;
        let start = *performance.dates().first().unwrap();
        let end = *performance.dates().last().unwrap();
        println!(
            "{} {:.6}",
            name,
            performance.time_weighted(start, end).unwrap()
        );
    }
    Ok(())
}
