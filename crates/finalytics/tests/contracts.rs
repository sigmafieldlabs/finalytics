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

#[test]
fn a_refit_after_fits_that_held_only_cash_enters_from_cash() {
    let classes = with_bills();
    let column = classes
        .labels()
        .iter()
        .position(|l| l == "US Equity")
        .unwrap();
    let returns = finalytics::risk::ReturnsTable::new(
        vec!["US Equity".to_owned()],
        vec![classes.returns().columns()[column].clone()],
    )
    .unwrap();
    let one = Assets::builder()
        .returns(classes.dates().to_vec(), returns)
        .risk_free_rate(0.02)
        .build()
        .unwrap();
    // A target the early windows cannot reach: their fits find no weights, and hold cash.
    let target = one.mean_returns()[0];
    let backtest = one
        .backtest(Method::TargetReturn)
        .target(target)
        .max_turnover(0.5)
        .walk_forward(Years(5))
        .build()
        .unwrap();
    let run = backtest.results.strategies[0]
        .out_of_sample
        .as_ref()
        .unwrap();
    assert!(!backtest.results.strategies[0].failures.is_empty());
    // Once a window reaches the target, the refit enters from cash, beyond the turnover limit.
    let entered = run
        .trades
        .iter()
        .find(|t| t.refit && t.turnover > 0.5 + 1e-9);
    assert!(entered.is_some(), "{:?}", run.trades);
}

/// The household's own weights, its euro cash counted with the dollar cash.
fn folded(portfolio: &finalytics::Portfolio, assets: &Assets) -> Vec<(String, f64)> {
    let mut out: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for (key, w) in portfolio.weights().unwrap() {
        let label = key.to_string();
        let to = if assets.risky_assets().contains(&label.as_str()) {
            label
        } else {
            assets.risk_free_asset().unwrap().to_owned()
        };
        *out.entry(to).or_default() += w;
    }
    out.into_iter().collect()
}

#[test]
fn rebalancing_to_an_optimisation_trades_what_it_traded_and_nothing_else() {
    let portfolio = datasets::portfolio();
    let assets = household();
    let optimisation = assets
        .optimise(Method::MinimumVariance)
        .current_weights(folded(&portfolio, &assets))
        .max_turnover(0.1)
        .max_trades(2)
        .min_trade(0.02)
        .build()
        .unwrap();
    let traded: Vec<String> = optimisation
        .report
        .trades
        .as_ref()
        .unwrap()
        .trades
        .iter()
        .filter(|t| !t.cash)
        .map(|t| t.asset.clone())
        .collect();
    assert_eq!(traded.len(), 2, "{traded:?}");
    let rebalancing = portfolio.rebalance(&optimisation).build().unwrap();
    let value: f64 = rebalancing.results.value_base.try_into().unwrap();
    let mut instruments: Vec<String> = rebalancing
        .results
        .accounts
        .iter()
        .flat_map(|a| a.trades.iter().map(|t| t.instrument.to_string()))
        .collect();
    instruments.sort_unstable();
    instruments.dedup();
    let mut expected = traded.clone();
    expected.sort_unstable();
    assert_eq!(instruments, expected);
    for account in &rebalancing.results.accounts {
        for trade in &account.trades {
            let share = f64::try_from(trade.value_base).unwrap() / value;
            assert!(share > 1e-6, "{} {}: {share}", trade.side, trade.instrument);
        }
    }
    // Every instrument ends at the optimised weight.
    let after = &rebalancing.results.weights_after;
    for (asset, w) in optimisation.results.weights.as_ref().unwrap() {
        let key: finalytics::HoldingKey = asset.parse().unwrap();
        let held = after.get(&key).copied().unwrap_or(0.0);
        assert!((held - w).abs() < 1e-9, "{asset}: {held} for {w}");
    }
}

#[test]
fn rebalancing_to_the_weights_held_trades_nothing() {
    let portfolio = datasets::portfolio();
    let rebalancing = portfolio
        .rebalance(&portfolio.weights().unwrap())
        .build()
        .unwrap();
    for account in &rebalancing.results.accounts {
        assert!(account.trades.is_empty(), "{:?}", account.trades);
        assert!(account.conversions.is_empty(), "{:?}", account.conversions);
    }
}

#[test]
fn a_change_too_small_to_settle_is_no_trade() {
    // The weights held, each moved by noise of 1e-11, as weights computed elsewhere can be.
    let portfolio = datasets::portfolio();
    let mut sign = 1.0;
    let nudged: std::collections::BTreeMap<finalytics::HoldingKey, f64> = portfolio
        .weights()
        .unwrap()
        .into_iter()
        .map(|(k, w)| {
            sign = -sign;
            let moved = if matches!(k, finalytics::HoldingKey::Instrument(_)) {
                w + sign * 1e-11
            } else {
                w
            };
            (k, moved)
        })
        .collect();
    let rebalancing = portfolio.rebalance(&nudged).build().unwrap();
    for account in &rebalancing.results.accounts {
        assert!(account.trades.is_empty(), "{:?}", account.trades);
    }
    // A real change, worth more than a cent, is still traded.
    let mut moved = portfolio.weights().unwrap();
    let vti: finalytics::HoldingKey = "VTI".parse().unwrap();
    let bnd: finalytics::HoldingKey = "BND".parse().unwrap();
    *moved.get_mut(&vti).unwrap() -= 1e-6;
    *moved.get_mut(&bnd).unwrap() += 1e-6;
    let rebalancing = portfolio.rebalance(&moved).build().unwrap();
    let traded: usize = rebalancing
        .results
        .accounts
        .iter()
        .map(|a| a.trades.len())
        .sum();
    assert!(traded >= 2);
}
