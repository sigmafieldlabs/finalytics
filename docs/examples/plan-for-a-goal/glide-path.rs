use std::collections::BTreeMap;

use finalytics::planning::{Mix, Model, Month};
use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;
    let path = Mix::GlidePath {
        name: None,
        points: BTreeMap::from([
            (
                "2026-11".parse::<Month>()?,
                BTreeMap::from([
                    ("US Equity".to_owned(), 0.8),
                    ("Bonds".to_owned(), 0.2),
                ]),
            ),
            (
                "2030-01".parse::<Month>()?,
                BTreeMap::from([
                    ("US Equity".to_owned(), 0.3),
                    ("Bonds".to_owned(), 0.7),
                ]),
            ),
        ]),
    };
    let result = assets
        .plan()
        .target(150_000.0, "2030-06")
        .start_value(60_000.0)
        .saving(2_500.0, Frequency::Month)
        .mix(path)
        .inflation(0.025)
        .model(Model::Normal)
        .simulations(500)
        .seed(7)
        .build()?;
    println!("{result}");
    result.show()?;
    Ok(())
}
