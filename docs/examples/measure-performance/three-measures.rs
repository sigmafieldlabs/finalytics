use finalytics::{
    performance::{FlowTiming, Performance},
    prelude::*,
};

fn main() -> Result<()> {
    let performance = Performance::from_parts(
        ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"]
            .map(|d| {
                d.parse()
                    .expect("the guide's fixed sample supplies this value")
            })
            .to_vec(),
        vec![1000.0, 1050.0, 1600.0, 1580.0],
        vec![0.0, 0.0, 500.0, 0.0],
        FlowTiming::StartOfDay,
    )?;
    let start = *performance
        .dates()
        .first()
        .expect("the guide's fixed sample supplies this value");
    let end = *performance
        .dates()
        .last()
        .expect("the guide's fixed sample supplies this value");
    println!(
        "Time-weighted: {:.6}",
        performance
            .time_weighted(start, end)
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Money-weighted: {:.6}",
        performance
            .money_weighted(start, end)
            .expect("the guide's fixed sample supplies this value")
    );
    println!(
        "Modified Dietz: {:.6}",
        performance
            .modified_dietz(start, end)
            .expect("the guide's fixed sample supplies this value")
    );
    Ok(())
}
