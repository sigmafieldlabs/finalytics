//! The contracts between features: each verb takes what the others give it, in every shape of
//! data. The shapes are the sample household's assets (five funds, euro cash, and dollar cash as
//! the risk-free asset), the asset classes with T-bills as the risk-free asset, and the asset
//! classes without them at a 2% rate.

#![allow(clippy::unwrap_used)]

use finalytics::{Assets, Method, datasets};

fn household() -> Assets {
    datasets::portfolio().assets().unwrap()
}

fn with_bills() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

fn at_a_rate() -> Assets {
    let prices = datasets::asset_class_prices();
    let keep: Vec<usize> = (0..prices.labels().len())
        .filter(|&i| prices.labels()[i] != "T-Bills")
        .collect();
    let prices = finalytics::PriceTable::new(
        prices.dates().to_vec(),
        keep.iter().map(|&i| prices.labels()[i].clone()).collect(),
        keep.iter().map(|&i| prices.columns()[i].clone()).collect(),
    )
    .unwrap();
    Assets::builder()
        .prices(prices)
        .risk_free_rate(0.02)
        .build()
        .unwrap()
}

fn shapes() -> [(&'static str, Assets); 3] {
    [
        ("household", household()),
        ("bills", with_bills()),
        ("rate", at_a_rate()),
    ]
}

#[test]
fn minimum_cvar_with_cash_reports_the_shortfall_of_every_weight() {
    for (shape, assets) in shapes() {
        let result = assets
            .optimise(Method::MinimumCvar)
            .cash(0.0, 0.2)
            .build()
            .unwrap_or_else(|e| panic!("{shape}: {e}"));
        let weights = result.results.weights.as_ref().unwrap();
        let cash = assets.risk_free_asset().unwrap_or("Cash");
        assert!(weights.contains_key(cash), "{shape}: {weights:?}");
        let summary = result.report.summary.as_ref().unwrap();
        assert!(summary.expected_shortfall.is_some(), "{shape}");
        // The shares are of the expected shortfall, over every weight with the cash.
        let shares: f64 = result.report.allocation.iter().map(|a| a.risk_share).sum();
        assert!(
            (shares - 1.0).abs() < 1e-6,
            "{shape}: shares add up to {shares}"
        );
        assert!(
            result.report.allocation.iter().any(|a| a.asset == cash),
            "{shape}"
        );
    }
}
