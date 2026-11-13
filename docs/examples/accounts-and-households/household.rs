use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut options = finalytics::performance::HouseholdOptions::default();
    options.group_by = vec!["owner".to_string()];
    let household = finalytics::performance::household(
        portfolio.books(),
        portfolio.market(),
        &portfolio.dates(),
        &options,
    )?;
    println!(
        "Household: {:.6}",
        household.household.time_weighted.unwrap()
    );
    for account in household.accounts {
        println!(
            "{} {:.6} {:.6}",
            account.name, account.time_weighted.unwrap(),
            account.contribution
        );
    }
    Ok(())
}
