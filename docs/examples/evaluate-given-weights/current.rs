use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let result = portfolio
        .assets()
        .build()?
        .evaluate(portfolio.weights()?)
        .name("Current household")
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
