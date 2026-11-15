use finalytics::{performance::Period, prelude::*};

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let performance = portfolio.performance()?;
    let benchmark = portfolio
        .market()
        .prices(
            &finalytics::InstrumentId::new("BENCH")
                .expect("the guide's fixed sample supplies this value"),
        )
        .expect("the guide's fixed sample supplies this value");
    println!("Benchmark column: BENCH");
    let excess = performance
        .excess_return(
            Period::SinceInception,
            None,
            &benchmark.map(|value| {
                f64::try_from(*value)
                    .expect("the guide's fixed sample supplies this value")
            }),
            5,
        )
        .expect("the guide's fixed sample supplies this value");
    println!("Arithmetic: {:.6}", excess.arithmetic);
    println!("Geometric: {:.6}", excess.geometric);
    Ok(())
}
