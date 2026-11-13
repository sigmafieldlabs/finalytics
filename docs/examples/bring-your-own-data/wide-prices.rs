use finalytics::prelude::*;

fn main() -> Result<()> {
    let prices = concat!(
        "date,VOO,AGG\n",
        "2026-01-30,500,98\n",
        "2026-02-27,505,98.2\n",
        "2026-03-31,510,99.1\n",
    );
    let assets = Assets::from_csv(prices)?;
    println!("Labels: {}", assets.labels().join(", "));
    println!("Periods per year: {}", assets.periods_per_year());
    Ok(())
}
