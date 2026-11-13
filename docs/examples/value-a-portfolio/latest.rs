use finalytics::prelude::*;

fn main() -> Result<()> {
    let valuation = datasets::portfolio().valuation()?;
    println!("Date: {}", valuation.date);
    println!("Positions: {}", valuation.positions.len());
    println!("Cash balances: {}", valuation.cash.len());
    println!("Total: {}", valuation.total_base);
    Ok(())
}
