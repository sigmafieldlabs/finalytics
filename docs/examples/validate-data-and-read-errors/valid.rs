use finalytics::prelude::*;

fn main() -> Result<()> {
    let data = finalytics::formats::portfolio_from_csv(
        None,
        None,
        "date,type,amount,currency\n2026-01-02,deposit,1000,USD\n",
        None,
        finalytics::formats::CsvOptions::default(),
    )?;
    println!("Issues: {:?}", data.validate().issues());
    Ok(())
}
