use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let current: Vec<(String, f64)> = portfolio
        .weights()?
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    let result = portfolio
        .assets()
        .build()?
        .optimise(Method::MaximumSharpe)
        .current_weights(current)
        .max_turnover(0.15)
        .keep_all_cash()
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
