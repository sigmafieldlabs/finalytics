use finalytics::LotMethod;
use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .rebalance([
            ("VTI", 0.35),
            ("EXSA", 0.1),
            ("BND", 0.35),
            ("GLD", 0.1),
            ("VNQ", 0.1),
        ])
        .accounts(["brokerage"])
        .lot_method(LotMethod::HighestCost)
        .whole_units()
        .fee_per_trade(2.0)
        .fee_bps(5.0)
        .minimum_fee(1.0)
        .minimum_trade(100.0)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
