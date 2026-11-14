use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .rebalance([
            ("VTI", 0.4),
            ("EXSA", 0.1),
            ("BND", 0.35),
            ("GLD", 0.1),
            ("VNQ", 0.05),
        ])
        .whole_units()
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
