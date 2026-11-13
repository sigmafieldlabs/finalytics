use finalytics::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = "saved-portfolio.json";
    std::fs::write(path, datasets::portfolio().to_json())?;
    let restored = Portfolio::from_json(&std::fs::read_to_string(path)?)?;
    println!(
        "Restored: {} {}",
        restored.base_currency(),
        restored.data().transactions.len()
    );
    std::fs::remove_file(path)?;
    Ok(())
}
