use finalytics::prelude::*;

fn main() {
    for account in &datasets::portfolio().data().accounts {
        let classes = format!("{:?}", account.classifications)
            .replace(": ", ":")
            .replace(", ", ",");
        println!(
            "{} {} {} {}",
            account.id,
            account.name.as_ref().unwrap(),
            account.taxable,
            classes
        );
    }
}
