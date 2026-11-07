//! The 55 S&P 500 companies of tests/sp500 screened on return, risk, trend and
//! sector. Run it at the root of the repository.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = Universe::builder()
        .prices_file("tests/sp500/prices.csv")
        .fields_file("tests/sp500/companies.csv")
        .risk_free_rate(0.04)
        .build()?;

    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .below("volatility", 0.30)
        .above("price_to_sma_200", 1.0)
        .between("rsi_14", 30.0, 70.0)
        .above("max_drawdown_3y", -0.30)
        .none_of("sector", ["Utilities"])
        .sort("sharpe_ratio_3y", Order::Descending)
        .limit(10)
        .build()?;

    screen.show()?;
    Ok(())
}
