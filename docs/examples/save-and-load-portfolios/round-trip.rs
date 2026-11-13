use finalytics::prelude::*;

fn main() -> Result<()> {
    let original = datasets::portfolio();
    let restored = Portfolio::from_json(&original.to_json())?;
    println!(
        "Equal values: {}",
        original.valuation()?.total_base == restored.valuation()?.total_base
    );
    println!("Transactions: {}", restored.data().transactions.len());
    Ok(())
}
