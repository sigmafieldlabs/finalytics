//! The contracts between features: each verb takes what the others give it, in every shape of
//! data. The shapes are the sample household's assets (five funds, euro cash, and dollar cash as
//! the risk-free asset), the asset classes with T-bills as the risk-free asset, and the asset
//! classes without them at a 2% rate.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::Years;
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

#[test]
fn the_engines_own_weights_pass_its_own_checks() {
    let portfolio = datasets::portfolio();
    let assets = household();
    let cases = [
        assets.optimise(Method::MinimumCdar),
        assets
            .optimise(Method::MeanVariance)
            .risk_aversion(3.0)
            .max_weight(0.35),
        assets.optimise(Method::MinimumVariance).cash(0.0, 0.0),
    ];
    for case in cases {
        let result = case.build().unwrap();
        let weights = result.results.weights.clone().unwrap();
        assert!(weights.values().all(|w| *w >= 0.0), "{weights:?}");
        assert!(
            weights.values().all(|w| *w == 0.0 || *w >= 1e-7),
            "{weights:?}"
        );
        let total: f64 = weights.values().sum();
        assert!((total - 1.0).abs() < 1e-12, "{total}");
        assert!(weights.values().all(|w| *w <= 1.0));
        // The figures are those of the weights returned.
        let valued = assets.evaluate(weights.clone()).build().unwrap();
        let volatility = result.results.volatility.unwrap();
        assert!((valued.results.volatility - volatility).abs() < 1e-12);
        // They are a target for rebalancing, and current weights for another optimisation.
        portfolio.rebalance(&result).build().unwrap();
        assets
            .optimise(Method::MinimumVariance)
            .current_weights(weights)
            .build()
            .unwrap();
    }
    // Within limits on the weights.
    let capped = with_bills()
        .optimise(Method::MaximumSharpe)
        .max_weight(0.3)
        .build()
        .unwrap();
    let weights = capped.results.weights.unwrap();
    assert!(weights.values().all(|w| *w <= 0.3), "{weights:?}");
}

#[test]
fn a_walk_forward_refits_from_the_weights_it_holds_without_checking_them_as_given() {
    // The refits start from weights the run holds, which drift with the returns.
    for (shape, assets) in shapes() {
        let window = if assets.periods_per_year() > 100.0 {
            Years(1)
        } else {
            Years(5)
        };
        for method in [Method::MeanVariance, Method::RobustMeanVariance] {
            let mut backtest = assets
                .backtest(method)
                .risk_aversion(3.0)
                .min_trade(0.01)
                .walk_forward(window);
            if method == Method::RobustMeanVariance {
                backtest = backtest
                    .uncertainty(finalytics::opt::ReturnUncertainty::Box {
                        half_widths: vec![0.01; assets.risky_assets().len()],
                    })
                    .max_turnover(0.5);
            }
            backtest
                .build()
                .unwrap_or_else(|e| panic!("{shape} {method:?}: {e}"));
        }
        assets
            .backtest(Method::MinimumCdar)
            .max_turnover(0.5)
            .walk_forward(window)
            .build()
            .unwrap_or_else(|e| panic!("{shape} minimum CDaR: {e}"));
    }
}
