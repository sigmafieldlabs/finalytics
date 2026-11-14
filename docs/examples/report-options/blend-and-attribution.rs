use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .report()
        .benchmark_blend([("VTI", 0.55), ("BND", 0.35), ("GLD", 0.1)])
        .attribution_by("asset_class")
        .group_by(["asset_class"])
        .name("Against a blended benchmark")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
