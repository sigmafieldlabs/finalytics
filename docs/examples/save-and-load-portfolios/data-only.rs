use finalytics::prelude::*;

fn main() -> Result<()> {
    let original = datasets::portfolio().data().clone();
    let json = finalytics::PortfolioDocument::new(original.clone()).to_json();
    let restored = finalytics::PortfolioDocument::from_json(&json)?.portfolio;
    println!("Equal records: {}", original == restored);
    println!("Instruments: {}", restored.instruments.len());
    Ok(())
}
