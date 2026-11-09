//! Group limits by a classification of the assets, such as at most 60% in equity.

#![allow(clippy::unwrap_used)]

use finalytics::{Assets, Error, Method, datasets};

fn classified() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .classes(
            "asset_class",
            [
                ("US Equity", "Equity"),
                ("Intl Equity", "Equity"),
                ("Bonds", "Fixed income"),
                ("Gold", "Real assets"),
                ("Real Estate", "Real assets"),
            ],
        )
        .build()
        .unwrap()
}

fn paths(error: Error) -> Vec<(String, String)> {
    let Error::Validation(report) = error else {
        panic!("{error}")
    };
    report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect()
}

#[test]
fn a_class_limit_is_the_group_of_the_assets_in_the_class() {
    let assets = classified();
    let by_class = assets
        .optimise(Method::MaximumSharpe)
        .class_limit("asset_class", "Equity", 0.0, 0.3)
        .build()
        .unwrap();
    let weights = by_class.results.weights.as_ref().unwrap();
    assert!(weights["US Equity"] + weights["Intl Equity"] <= 0.3 + 1e-6);
    let listed = assets
        .optimise(Method::MaximumSharpe)
        .group("Equity", &["US Equity", "Intl Equity"], 0.0, 0.3)
        .build()
        .unwrap();
    assert_eq!(by_class.results.weights, listed.results.weights);
    let group = &by_class.report.groups[0];
    assert_eq!(group.name, "Equity");
    assert_eq!(group.assets, ["US Equity", "Intl Equity"]);
    assert_eq!(
        by_class.report.context.limits,
        ["Equity (asset_class): 0% to 30% together"]
    );
}

#[test]
fn a_portfolios_assets_carry_its_instruments_classes() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().build().unwrap();
    assert_eq!(assets.classes()["asset_class"]["VTI"], "Equity");
    let result = assets
        .optimise(Method::MaximumSharpe)
        .class_limit("asset_class", "Equity", 0.0, 0.5)
        .class_limit("region", "North America", 0.2, 0.8)
        .build()
        .unwrap();
    let w = result.results.weights.unwrap();
    assert!(w["VTI"] + w["EXSA"] <= 0.5 + 1e-6);
    // The same limit in a comparison.
    let compared = assets
        .compare([Method::MinimumVariance, Method::EqualWeight])
        .class_limit("asset_class", "Equity", 0.0, 0.5)
        .build()
        .unwrap();
    assert_eq!(compared.results.strategies.len(), 2);
}

#[test]
fn mistakes_with_classes_name_what_there_is() {
    let assets = classified();
    let found = paths(
        assets
            .optimise(Method::MaximumSharpe)
            .class_limit("sector", "Energy", 0.0, 0.3)
            .class_limit("asset_class", "Crypto", 0.0, 0.1)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [
            (
                "options.groups.Crypto".to_owned(),
                "no asset is classed Crypto under asset_class; the classes are Equity, Fixed \
                 income, Real assets"
                    .to_owned()
            ),
            (
                "options.groups.Energy.classification".to_owned(),
                "no asset is classified by sector; the classifications are asset_class".to_owned()
            ),
        ]
    );
    let Err(e) = Assets::builder()
        .prices(datasets::asset_class_prices())
        .classes("asset_class", [("Bitcoin", "Crypto")])
        .build()
    else {
        panic!("an unknown asset must be refused")
    };
    assert_eq!(paths(e)[0].0, "classes.asset_class.Bitcoin");
}
