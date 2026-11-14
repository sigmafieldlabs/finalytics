use finalytics::{
    performance::{AttributionPeriod, Linking, Segment, brinson_fachler},
    prelude::*,
};

fn period(
    start: &str,
    end: &str,
    rows: &[(&str, f64, f64, f64, f64)],
) -> Result<AttributionPeriod> {
    Ok(AttributionPeriod {
        start: start.parse()?,
        end: end.parse()?,
        segments: rows
            .iter()
            .map(|(name, pw, pr, bw, br)| Segment {
                name: name.to_string(),
                portfolio_weight: *pw,
                portfolio_return: *pr,
                benchmark_weight: *bw,
                benchmark_return: *br,
            })
            .collect(),
    })
}

fn main() -> Result<()> {
    let periods = vec![
        period(
            "2026-01-01",
            "2026-04-01",
            &[
                ("Equities", 0.6, 0.08, 0.5, 0.06),
                ("Bonds", 0.4, 0.01, 0.5, 0.015),
            ],
        )?,
        period(
            "2026-04-01",
            "2026-07-01",
            &[
                ("Equities", 0.65, -0.05, 0.5, -0.04),
                ("Bonds", 0.35, 0.02, 0.5, 0.02),
            ],
        )?,
    ];
    let methods =
        [("carino", Linking::Carino), ("menchero", Linking::Menchero)];
    for (name, method) in methods {
        let result = brinson_fachler(&periods, method)?;
        println!(
            "{} {:.6} {:.6}",
            name, result.allocation, result.excess_return
        );
    }
    Ok(())
}
