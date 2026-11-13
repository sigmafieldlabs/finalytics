use finalytics::prelude::*;

fn main() {
    let portfolio = datasets::portfolio();
    let document = portfolio.to_document();
    println!("{}", finalytics::formats::FORMAT);
    println!("{}", finalytics::formats::FORMAT_VERSION);
    println!("Has market: {}", document.market.is_some());
}
