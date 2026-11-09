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

/// Weights that hold `label` at 0.1 and spread the rest over the risky assets.
fn holding(assets: &Assets, label: &str) -> std::collections::BTreeMap<String, f64> {
    let risky: Vec<&str> = assets
        .risky_assets()
        .into_iter()
        .filter(|r| *r != label)
        .collect();
    let mut w: std::collections::BTreeMap<String, f64> = risky
        .iter()
        .map(|r| ((*r).to_owned(), 0.9 / risky.len() as f64))
        .collect();
    *w.entry(label.to_owned()).or_default() += 0.1;
    w
}

/// Each kind of label of `assets`: a risky asset, the risk-free asset, cash in another
/// currency, and `Cash` at the risk-free rate.
fn labels_of(assets: &Assets) -> Vec<String> {
    let mut out = vec![assets.risky_assets()[0].to_owned()];
    out.extend(assets.cash_assets().iter().map(|c| (*c).to_owned()));
    if assets.risk_free_asset().is_none() {
        out.push("Cash".to_owned());
    }
    out
}

fn done<T>(result: Result<T, finalytics::Error>) -> Result<(), finalytics::Error> {
    result.map(|_| ())
}

/// Every input that takes weights by asset, run with `weights`.
fn weights_inputs(
    assets: &Assets,
    weights: &std::collections::BTreeMap<String, f64>,
) -> Vec<(&'static str, Result<(), finalytics::Error>)> {
    let w = || weights.clone();
    let risky = assets.risky_assets();
    vec![
        (
            "optimise current_weights",
            done(
                assets
                    .optimise(Method::MinimumVariance)
                    .current_weights(w())
                    .build(),
            ),
        ),
        (
            "optimise current_weights with cash",
            done(
                assets
                    .optimise(Method::MinimumVariance)
                    .cash(0.0, 0.3)
                    .current_weights(w())
                    .build(),
            ),
        ),
        (
            "optimise benchmark_weights",
            done(
                assets
                    .optimise(Method::MinimumVariance)
                    .benchmark_weights(w())
                    .max_tracking_error(0.5)
                    .build(),
            ),
        ),
        (
            "frontier current_weights",
            done(
                assets
                    .optimise(Method::MinimumVariance)
                    .current_weights(w())
                    .max_turnover(0.5)
                    .frontier(5),
            ),
        ),
        ("evaluate weights", done(assets.evaluate(w()).build())),
        (
            "backtest weights",
            done(
                assets
                    .backtest(finalytics::backtest::Strategy::weights("Mix", w()))
                    .build(),
            ),
        ),
        (
            "backtest benchmark",
            done(
                assets
                    .backtest(Method::MinimumVariance)
                    .benchmark(finalytics::backtest::Benchmark::Weights {
                        name: None,
                        weights: w(),
                    })
                    .build(),
            ),
        ),
        (
            "compare weights",
            done(
                assets
                    .compare([Method::MinimumVariance])
                    .weights("Mix", w())
                    .build(),
            ),
        ),
        (
            "plan weights",
            done(
                assets
                    .plan()
                    .weights(w())
                    .projection("2030-12")
                    .inflation(0.02)
                    .simulations(100)
                    .build(),
            ),
        ),
        (
            "plan alternatives",
            done(
                assets
                    .plan()
                    .weights([(risky[0], 1.0)])
                    .alternative("Alt", w())
                    .projection("2030-12")
                    .inflation(0.02)
                    .simulations(100)
                    .build(),
            ),
        ),
    ]
}

#[test]
fn every_input_of_weights_takes_every_label_of_the_assets() {
    let portfolio = datasets::portfolio();
    for (shape, assets) in shapes() {
        for label in labels_of(&assets) {
            let weights = holding(&assets, &label);
            for (input, result) in weights_inputs(&assets, &weights) {
                assert!(
                    result.is_ok(),
                    "{shape} {input} {label}: {:?}",
                    result.err()
                );
            }
            if shape == "household" {
                portfolio
                    .rebalance(weights.clone())
                    .build()
                    .unwrap_or_else(|e| panic!("rebalance {label}: {e}"));
            }
        }
    }
}

#[test]
fn every_input_of_weights_refuses_an_unknown_label_in_one_message() {
    let portfolio = datasets::portfolio();
    for (shape, assets) in shapes() {
        let weights = holding(&assets, "Nope");
        for (input, result) in weights_inputs(&assets, &weights) {
            let Err(finalytics::Error::Validation(report)) = result else {
                panic!("{shape} {input}: {result:?}");
            };
            assert!(
                report
                    .issues()
                    .iter()
                    .any(|i| i.path.ends_with(".Nope") && i.message == "unknown asset Nope"),
                "{shape} {input}: {report}"
            );
        }
    }
    let Err(finalytics::Error::Validation(report)) =
        portfolio.rebalance(holding(&household(), "Nope")).build()
    else {
        panic!("rebalance took Nope");
    };
    assert!(
        report
            .issues()
            .iter()
            .any(|i| i.path == "targets.Nope" && i.message == "unknown asset Nope"),
        "{report}"
    );
}

#[test]
fn bounds_and_groups_refuse_the_cash_in_favour_of_cash_limits() {
    let assets = household();
    for (label, says) in [
        ("USD cash", "limit it with cash"),
        ("EUR cash", "cash in another currency"),
    ] {
        let Err(finalytics::Error::Validation(report)) = assets
            .optimise(Method::MinimumVariance)
            .asset_bounds(label, 0.0, 0.1)
            .group("G", &[label, "VTI"], 0.0, 0.9)
            .build()
        else {
            panic!("{label} accepted");
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert!(
            paths.contains(&format!("options.asset_bounds.{label}").as_str()),
            "{report}"
        );
        assert!(paths.contains(&"options.groups.G"), "{report}");
        assert!(
            report.issues().iter().all(|i| i.message.contains(says)),
            "{report}"
        );
    }
}

#[test]
fn the_cash_a_mix_holds_in_another_currency_is_measured_in_every_verb() {
    let portfolio = datasets::portfolio();
    let assets = household();
    let mix: std::collections::BTreeMap<String, f64> = portfolio
        .weights()
        .unwrap()
        .into_iter()
        .map(|(k, w)| (k.to_string(), w))
        .collect();
    assert!(mix.contains_key("EUR cash"));
    // A backtest of the mix holds the euro cash as evaluate's history does.
    let valued = assets.evaluate(mix.clone()).build().unwrap();
    let backtest = assets
        .backtest(finalytics::backtest::Strategy::weights("Mix", mix.clone()))
        .build()
        .unwrap();
    let history = valued.report.history.as_ref().unwrap();
    let tested = &backtest.report.strategy.tested.statistics;
    assert!((history.weights.statistics.volatility - tested.volatility).abs() < 1e-12);
    assert!(backtest.results.assets.contains(&"EUR cash".to_owned()));
    // The tracking error against the mix counts the euro cash's risk.
    let optimised = assets
        .optimise(Method::MinimumVariance)
        .benchmark_weights(mix.clone())
        .build()
        .unwrap();
    let weights = optimised.results.weights.as_ref().unwrap();
    assert!(!weights.contains_key("EUR cash"), "{weights:?}");
    let covariance = assets.covariance().unwrap().matrix;
    let labels = covariance.labels().to_vec();
    let active: Vec<f64> = labels
        .iter()
        .map(|l| weights.get(l).copied().unwrap_or(0.0) - mix.get(l).copied().unwrap_or(0.0))
        .collect();
    let n = labels.len();
    let variance: f64 = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| active[i] * covariance.get(i, j) * active[j])
                .sum::<f64>()
        })
        .sum();
    let reported = optimised
        .report
        .summary
        .as_ref()
        .unwrap()
        .tracking_error
        .unwrap();
    assert!(
        (reported - variance.sqrt()).abs() < 1e-12,
        "{reported} {}",
        variance.sqrt()
    );
    // From the mix, the optimisation invests the cash, and its trades say so.
    let traded = assets
        .optimise(Method::MinimumVariance)
        .current_weights(mix.clone())
        .max_turnover(0.3)
        .build()
        .unwrap();
    let trades = traded.report.trades.as_ref().unwrap();
    let eur = trades
        .trades
        .iter()
        .find(|t| t.asset == "EUR cash")
        .unwrap();
    assert!(eur.cash && eur.proposed == 0.0 && (eur.current - mix["EUR cash"]).abs() < 1e-15);
    let turnover: f64 = assets
        .labels()
        .iter()
        .map(|l| {
            (traded
                .results
                .weights
                .as_ref()
                .unwrap()
                .get(l)
                .copied()
                .unwrap_or(0.0)
                - mix.get(l).copied().unwrap_or(0.0))
            .abs()
        })
        .sum::<f64>()
        / 2.0;
    assert!((trades.turnover - turnover).abs() < 1e-12);
    assert!(trades.turnover <= 0.3 + 1e-9);
}

#[test]
fn fixed_weights_come_back_as_given() {
    use finalytics::backtest::{Benchmark, Strategy};
    let portfolio = datasets::portfolio();
    for (shape, assets) in shapes() {
        let risky = assets.risky_assets();
        let cash = assets.risk_free_asset().unwrap_or("Cash");
        let mut mixes: Vec<std::collections::BTreeMap<String, f64>> = vec![
            [(risky[0], 0.6), (risky[1], 0.3), (cash, 0.1)]
                .into_iter()
                .map(|(a, w)| (a.to_owned(), w))
                .collect(),
            [(risky[0], 0.5), (risky[1], 0.5)]
                .into_iter()
                .map(|(a, w)| (a.to_owned(), w))
                .collect(),
        ];
        if shape == "household" {
            mixes.push(
                portfolio
                    .weights()
                    .unwrap()
                    .into_iter()
                    .map(|(k, w)| (k.to_string(), w))
                    .collect(),
            );
        }
        for mix in mixes {
            let fixed = || Strategy::weights("Mix", mix.clone());
            let benchmark = || Benchmark::Weights {
                name: None,
                weights: mix.clone(),
            };
            let backtest = assets.backtest(fixed()).build().unwrap();
            assert_eq!(backtest.report.strategy.weights[0].weights, mix, "{shape}");
            let compared = assets
                .compare([Strategy::from(Method::MinimumVariance), fixed()])
                .build()
                .unwrap();
            assert_eq!(
                compared.report.strategies[1].weights[0].weights, mix,
                "{shape}"
            );
            let against = assets
                .backtest(Method::MinimumVariance)
                .benchmark(benchmark())
                .build()
                .unwrap();
            let b = against.report.benchmark.as_ref().unwrap();
            assert_eq!(b.weights[0].weights, mix, "{shape}");
            let compared = assets
                .compare([Method::MinimumVariance])
                .benchmark(benchmark())
                .build()
                .unwrap();
            let b = compared.report.benchmark.as_ref().unwrap();
            assert_eq!(b.weights[0].weights, mix, "{shape}");
            // The report prints its allocation with the cash as given.
            assert!(
                backtest
                    .to_string()
                    .contains(&mix.keys().next().unwrap()[..3])
            );
        }
    }
}

#[test]
fn a_backtest_is_titled_with_the_name_given() {
    let assets = with_bills();
    let method = assets
        .backtest(Method::MinimumVariance)
        .name("Mine")
        .build()
        .unwrap();
    assert_eq!(method.report.name.as_deref(), Some("Mine"));
    assert_eq!(method.report.strategy.name, "Mine");
    let fixed = assets
        .backtest([("US Equity", 0.5), ("Bonds", 0.5)])
        .name("Mine")
        .build()
        .unwrap();
    assert_eq!(fixed.report.name.as_deref(), Some("Mine"));
    assert!(fixed.to_string().contains("Mine"));
    // Without one, the report has none, and the strategy's own name titles it.
    let unnamed = assets.backtest(Method::MinimumVariance).build().unwrap();
    assert_eq!(unnamed.report.name, None);
}

#[test]
fn different_inputs_give_different_digests() {
    let assets = with_bills();
    let backtest =
        |b: finalytics::backtest::BacktestBuilder<'_>| b.build().unwrap().report.context.digest;
    let sixty = [("US Equity", 0.6), ("Bonds", 0.4)];
    let forty = [("US Equity", 0.4), ("Bonds", 0.6)];
    let pairs = [
        (
            backtest(assets.backtest(Method::MinimumVariance)),
            backtest(assets.backtest(Method::RiskParity)),
        ),
        (
            backtest(assets.backtest(sixty)),
            backtest(assets.backtest(forty)),
        ),
        (
            backtest(assets.backtest(Method::MinimumVariance)),
            backtest(assets.backtest(Method::MinimumVariance).benchmark(sixty)),
        ),
        (
            assets
                .compare([Method::MinimumVariance])
                .build()
                .unwrap()
                .report
                .context
                .digest,
            assets
                .compare([Method::MinimumVariance])
                .max_weight(0.3)
                .build()
                .unwrap()
                .report
                .context
                .digest,
        ),
        (
            assets
                .optimise(Method::TargetReturn)
                .target(0.06)
                .build()
                .unwrap()
                .report
                .context
                .digest,
            assets
                .optimise(Method::TargetReturn)
                .target(0.07)
                .build()
                .unwrap()
                .report
                .context
                .digest,
        ),
        (
            assets
                .optimise(Method::MinimumVariance)
                .build()
                .unwrap()
                .report
                .context
                .digest,
            assets
                .optimise(Method::MinimumVariance)
                .current_weights(holding(&assets, "T-Bills"))
                .build()
                .unwrap()
                .report
                .context
                .digest,
        ),
        (
            assets
                .evaluate(sixty)
                .build()
                .unwrap()
                .report
                .context
                .digest,
            assets
                .evaluate(forty)
                .build()
                .unwrap()
                .report
                .context
                .digest,
        ),
    ];
    for (k, (a, b)) in pairs.iter().enumerate() {
        assert_ne!(a, b, "pair {k}");
    }
    // The same inputs give the same digest.
    assert_eq!(
        backtest(assets.backtest(Method::MinimumVariance)),
        backtest(assets.backtest(Method::MinimumVariance))
    );
}

#[test]
fn the_frontier_refuses_what_it_does_not_use() {
    let assets = with_bills();
    let refused = |result: Result<Vec<finalytics::FrontierPortfolio>, finalytics::Error>| {
        let Err(finalytics::Error::Validation(report)) = result else {
            panic!("accepted: {result:?}");
        };
        report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect::<Vec<_>>()
    };
    let found = refused(
        assets
            .optimise(Method::TargetReturn)
            .target(0.1)
            .risk_aversion(3.0)
            .resample(50)
            .frontier(5),
    );
    assert_eq!(
        found,
        [
            (
                "options.target".to_owned(),
                "the frontier does not use target".to_owned()
            ),
            (
                "options.risk_aversion".to_owned(),
                "the frontier does not use risk_aversion".to_owned()
            ),
            (
                "options.resample".to_owned(),
                "the frontier does not use resample".to_owned()
            ),
        ]
    );
    // The risk-free rate is the return of Cash, where cash limits hold it.
    let found = refused(
        assets
            .optimise(Method::MinimumVariance)
            .risk_free_rate(0.03)
            .frontier(5),
    );
    assert_eq!(found[0].0, "options.risk_free_rate");
    let rate = at_a_rate();
    let points = rate
        .optimise(Method::MinimumVariance)
        .risk_free_rate(0.03)
        .cash(0.0, 1.0)
        .frontier(5)
        .unwrap();
    assert!(
        (points[0].expected_return - 0.03).abs() < 1e-4,
        "{:?}",
        points[0]
    );
    // The method is not used, nor needed: target return draws the frontier without a target.
    let points = assets
        .optimise(Method::TargetReturn)
        .max_weight(0.5)
        .current_weights(holding(&assets, "T-Bills"))
        .max_turnover(0.5)
        .benchmark_weights(holding(&assets, "T-Bills"))
        .expected_return("Gold", 0.05)
        .frontier(5)
        .unwrap();
    assert_eq!(points.len(), 5);
}

#[test]
fn a_comparison_refuses_and_notes_options_in_the_words_of_one_optimisation() {
    let assets = with_bills();
    let issues = |r: Result<finalytics::Comparison, finalytics::Error>| -> Vec<(String, String)> {
        let Err(finalytics::Error::Validation(report)) = r else {
            panic!("accepted");
        };
        report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect()
    };
    let alone = assets
        .optimise(Method::MaximumSharpe)
        .cash(0.0, 0.2)
        .build()
        .unwrap_err()
        .to_string();
    let compared = issues(
        assets
            .compare([Method::MaximumSharpe])
            .cash(0.0, 0.2)
            .build(),
    );
    assert_eq!(compared.len(), 1);
    assert!(alone.contains(&compared[0].1), "{alone}\n{compared:?}");
    assert!(
        compared[0]
            .1
            .starts_with("maximum Sharpe does not take cash: ")
    );
    let compared = issues(
        assets
            .compare([Method::MinimumCvar])
            .max_trades(2)
            .walk_forward(Years(5))
            .build(),
    );
    assert_eq!(
        compared[0].1,
        "minimum CVaR does not take max_trades: its optimum on the periods' returns need not be \
         unique, so trades cannot be left out one by one"
    );
    let compared = issues(
        assets
            .compare([Method::MaximumSharpe, Method::MaximumDiversification])
            .cash(0.0, 0.2)
            .build(),
    );
    assert_eq!(
        compared,
        [(
            "options.cash".to_owned(),
            "maximum Sharpe and maximum diversification do not take cash: cash at the risk-free \
             rate leaves their ratios as they are, so they would be as good with any amount"
                .to_owned()
        )]
    );
    let compared = issues(
        assets
            .compare([Method::MaximumSharpe, Method::EqualWeight])
            .cash(0.0, 0.2)
            .build(),
    );
    assert_eq!(compared.len(), 2, "{compared:?}");
    assert!(
        compared[0]
            .1
            .starts_with("equal weight is fully invested and long only")
    );
    // A note names the options that do not apply to a method, and says no more.
    let comparison = assets
        .compare([
            Method::MaximumSharpe,
            Method::MinimumVariance,
            Method::TargetReturn,
        ])
        .cash(0.0, 0.2)
        .max_weight(0.5)
        .target(0.05)
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        comparison.report.strategies[0].notes,
        ["The target and the limits on cash do not apply."]
    );
    let comparison = assets
        .compare([Method::EqualWeight, Method::MinimumVariance])
        .max_weight(0.5)
        .build()
        .unwrap();
    assert_eq!(
        comparison.report.strategies[0].notes,
        ["Fully invested and long only; the limits on weights do not apply."]
    );
}
