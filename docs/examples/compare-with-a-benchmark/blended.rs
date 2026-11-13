use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .report()
        .benchmark_blend([("VTI", 0.60), ("BND", 0.40)])
        .build()?;
    print!("{result}");
    result.show()?;
    Ok(())
}
