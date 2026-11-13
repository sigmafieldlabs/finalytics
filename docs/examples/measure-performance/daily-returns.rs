use finalytics::{performance::{FlowTiming, Performance}, prelude::*};

fn main() -> Result<()> {
    let performance = Performance::from_parts(
        ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"]
            .map(|d| d.parse().unwrap())
            .to_vec(),
        vec![1000.0, 1050.0, 1600.0, 1580.0],
        vec![0.0, 0.0, 500.0, 0.0],
        FlowTiming::StartOfDay,
    )?;
    let returns = performance.returns();
    for (date, value) in returns.dates().iter().zip(returns.values()) {
        println!("{} {:.6}", date, value);
    }
    Ok(())
}
