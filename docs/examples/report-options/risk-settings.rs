use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .report()
        .benchmark("BENCH")
        .group_by(["asset_class", "region"])
        .confidences(&[0.9, 0.95, 0.99])
        .rolling_window(63)
        .histogram_bins(20)
        .top(5)
        .name("Risk presentation options")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
