use finalytics::prelude::*;

fn main() -> Result<()> {
    let valuation = datasets::portfolio().valuation()?;
    let positions: finalytics::Decimal = valuation
        .positions
        .iter()
        .map(|p| p.market_value_base)
        .sum();
    let cash: finalytics::Decimal = valuation
        .cash
        .iter()
        .map(|c| c.value_base)
        .sum();
    println!("Positions: {:.2}", positions.round_dp(2));
    println!("Cash: {:.2}", cash.round_dp(2));
    println!("Reconciles: {}", positions + cash == valuation.total_base);
    Ok(())
}
