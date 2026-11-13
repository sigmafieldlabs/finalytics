use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let holdings = portfolio.books().holdings_on("2026-10-30".parse()?);
    for position in holdings.positions() {
        println!(
            "{} {} {} {}",
            position.account, position.instrument,
            position.quantity, position.cost_base
        );
    }
    Ok(())
}
