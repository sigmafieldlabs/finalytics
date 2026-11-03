//! A portfolio from its CSV files, reported on against its benchmark. Run it
//! in a folder that holds the files.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::builder()
        .transactions("transactions.csv")
        .prices("prices.csv")
        .fx("fx.csv")
        .instruments("instruments.csv")
        .accounts("accounts.csv")
        .build()?;

    let report = portfolio
        .report()
        .benchmark("BENCH")
        .group_by(["asset_class", "region"])
        .build()?;

    report.show()?;
    Ok(())
}
