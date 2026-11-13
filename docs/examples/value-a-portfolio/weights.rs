use finalytics::prelude::*;

fn main() -> Result<()> {
    let weights = datasets::portfolio().weights()?;
    let mut rows = weights.iter().collect::<Vec<_>>();
    rows.sort_by_key(|(name, _)| name.to_string());
    for (name, weight) in rows {
        println!("{} {:.6}", name, weight);
    }
    println!("Total: {:.6}", weights.values().sum::<f64>());
    Ok(())
}
