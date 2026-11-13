use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio.report().accounts(["brokerage"]).build()?;
    print!("{result}");
    result.show()?;
    Ok(())
}
