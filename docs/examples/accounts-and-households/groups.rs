use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = datasets::portfolio();
    let mut options = finalytics::performance::HouseholdOptions::default();
    options.group_by = vec!["owner".to_string(), "wrapper".to_string()];
    let household = finalytics::performance::household(
        portfolio.books(),
        portfolio.market(),
        &portfolio.dates(),
        &options,
    )?;
    for scheme in household.groups {
        for group in scheme.groups {
            let accounts = group
                .accounts
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "{} {} {} {:.6}",
                scheme.scheme, group.name, accounts, group.contribution
            );
        }
    }
    Ok(())
}
