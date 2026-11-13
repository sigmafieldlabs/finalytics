use finalytics::{
    performance::{brinson_fachler, AttributionPeriod, Linking, Segment},
    prelude::*,
};

fn main() -> Result<()> {
    let segments = [
        ("UK", 0.4, 0.20, 0.4, 0.10),
        ("Japan", 0.3, -0.05, 0.2, -0.04),
        ("US", 0.3, 0.06, 0.4, 0.08),
    ]
    .map(|(name, pw, pr, bw, br)| Segment {
        name: name.to_string(),
        portfolio_weight: pw,
        portfolio_return: pr,
        benchmark_weight: bw,
        benchmark_return: br,
    })
    .to_vec();
    let period = AttributionPeriod {
        start: "2026-01-01".parse()?,
        end: "2026-12-31".parse()?,
        segments,
    };
    let result = brinson_fachler(&[period], Linking::Carino)?;
    println!("Excess: {:.6}", result.excess_return);
    for segment in result.segments {
        println!(
            "{} {:.6} {:.6} {:.6}",
            segment.name, segment.allocation,
            segment.selection, segment.interaction
        );
    }
    Ok(())
}
