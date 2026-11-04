//! The estimators, downside methods and limits of `Assets`: the estimates optimisation uses,
//! what the downside methods minimise, the limit on concentration, resampling and rebalancing
//! every so many periods, each checked against a calculation made here from the returns.

#![allow(clippy::unwrap_used)]

use finalytics::backtest::{Rebalance, Years};
use finalytics::risk::{CovarianceMethod, MeanMethod, ReturnsTable};
use finalytics::{Assets, Error, Method, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

fn paths(result: Result<impl std::fmt::Debug, Error>) -> Vec<String> {
    match result {
        Err(Error::Validation(report)) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

/// Each period's return of `weights` on the risky assets.
fn portfolio_returns(
    assets: &Assets,
    weights: &std::collections::BTreeMap<String, f64>,
) -> Vec<f64> {
    let columns = assets.returns().columns();
    (0..assets.dates().len())
        .map(|t| {
            assets
                .labels()
                .iter()
                .zip(columns)
                .map(|(label, column)| weights.get(label).copied().unwrap_or(0.0) * column[t])
                .sum()
        })
        .collect()
}

#[test]
fn optimisation_uses_the_chosen_mean_estimator() {
    let historical = assets();
    let shrunk = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .mean_returns(MeanMethod::JamesStein)
        .build()
        .unwrap();
    assert_eq!(shrunk.mean_method(), MeanMethod::JamesStein);
    // The risky assets' means shrink towards their average; T-bills keep their own.
    let risky = shrunk.risky_assets().len() as f64;
    let mean_of = |a: &Assets| {
        a.labels()
            .iter()
            .zip(a.mean_returns())
            .filter(|(l, _)| l.as_str() != "T-Bills")
            .map(|(_, m)| m)
            .sum::<f64>()
            / risky
    };
    let grand = mean_of(&historical);
    assert!((mean_of(&shrunk) - grand).abs() < 1e-12);
    for (before, after) in historical.mean_returns().iter().zip(shrunk.mean_returns()) {
        assert!((after - grand).abs() <= (before - grand).abs() + 1e-15);
    }
    let bills = historical
        .labels()
        .iter()
        .position(|l| l == "T-Bills")
        .unwrap();
    assert_eq!(
        shrunk.mean_returns()[bills],
        historical.mean_returns()[bills]
    );
    assert_eq!(shrunk.risk_free_rate(), historical.risk_free_rate());

    // The same estimates as the estimator gives on the risky assets' returns, annualised.
    let labels: Vec<String> = shrunk
        .risky_assets()
        .iter()
        .map(|l| (*l).to_owned())
        .collect();
    let columns = labels
        .iter()
        .map(|l| {
            let i = shrunk.labels().iter().position(|x| x == l).unwrap();
            shrunk.returns().columns()[i].clone()
        })
        .collect();
    let direct = finalytics::risk::mean_returns(
        &ReturnsTable::new(labels.clone(), columns).unwrap(),
        MeanMethod::JamesStein,
    )
    .unwrap();
    let result = shrunk.optimise(Method::MaximumSharpe).build().unwrap();
    let weights = result.results.weights.as_ref().unwrap();
    let expected: f64 = labels
        .iter()
        .zip(&direct.means)
        .map(|(l, m)| weights[l] * m * shrunk.periods_per_year())
        .sum();
    assert!((result.results.expected_return.unwrap() - expected).abs() < 1e-12);
    let text = result.report.to_text(None);
    assert!(text.contains("James–Stein mean expected returns"), "{text}");
}

#[test]
fn estimators_the_returns_cannot_support_are_reported_when_the_assets_are_built() {
    let prices = datasets::asset_class_prices();
    let few = finalytics::risk::PriceTable::new(
        prices.dates()[..10].to_vec(),
        prices.labels().to_vec(),
        prices.columns().iter().map(|c| c[..10].to_vec()).collect(),
    )
    .unwrap();
    let build = |covariance: CovarianceMethod, mean: MeanMethod| {
        Assets::builder()
            .prices(few.clone())
            .covariance(covariance)
            .mean_returns(mean)
            .build()
    };
    // Nine returns: too few for nonlinear shrinkage, and enough for James–Stein with six assets.
    assert_eq!(
        paths(build(
            CovarianceMethod::NonlinearShrinkage,
            MeanMethod::Historical
        )),
        ["covariance"]
    );
    assert!(build(CovarianceMethod::LedoitWolf, MeanMethod::JamesStein).is_ok());
    assert_eq!(
        paths(build(
            CovarianceMethod::Statistical { factors: 0 },
            MeanMethod::Ewma { decay: 1.5 }
        )),
        ["covariance.factors", "mean_returns.decay"]
    );
}

#[test]
fn the_downside_methods_minimise_what_they_say() {
    let assets = assets();
    let periods = assets.dates().len() as f64;

    let semi = assets
        .optimise(Method::MinimumSemivariance)
        .build()
        .unwrap();
    let returns = portfolio_returns(&assets, semi.results.weights.as_ref().unwrap());
    let below = returns.iter().map(|r| r.min(0.0).powi(2)).sum::<f64>() / periods;
    let value = semi.results.solution.objective_value.unwrap();
    assert!((value - below).abs() < 1e-12, "{value} against {below}");
    // Equal weight has no less.
    let equal = assets.optimise(Method::EqualWeight).build().unwrap();
    let equal_returns = portfolio_returns(&assets, equal.results.weights.as_ref().unwrap());
    let equal_below = equal_returns
        .iter()
        .map(|r| r.min(0.0).powi(2))
        .sum::<f64>()
        / periods;
    assert!(below <= equal_below);

    // With a floor, the expected return reaches it.
    for method in [
        Method::MinimumCvar,
        Method::MinimumSemivariance,
        Method::MinimumCdar,
    ] {
        let floored = assets.optimise(method).target(0.08).build().unwrap();
        assert_eq!(floored.results.status.to_string(), "optimal", "{method:?}");
        assert!(floored.results.expected_return.unwrap() >= 0.08 - 1e-9);
        let text = floored.report.to_text(None);
        assert!(
            text.contains("expected return at least 8% a year"),
            "{text}"
        );
    }

    // CDaR: the tail mean of the drawdowns of the summed returns, from a start of zero.
    let cdar = assets
        .optimise(Method::MinimumCdar)
        .confidence(0.9)
        .build()
        .unwrap();
    let returns = portfolio_returns(&assets, cdar.results.weights.as_ref().unwrap());
    let (mut sum, mut peak) = (0.0f64, 0.0f64);
    let mut falls: Vec<f64> = returns
        .iter()
        .map(|r| {
            sum += r;
            peak = peak.max(sum);
            peak - sum
        })
        .collect();
    falls.sort_by(f64::total_cmp);
    let alpha = 0.1;
    let tail = falls
        .iter()
        .map(|z| z + falls.iter().map(|d| (d - z).max(0.0)).sum::<f64>() / (alpha * periods))
        .fold(f64::INFINITY, f64::min);
    let value = cdar.results.solution.objective_value.unwrap();
    assert!((value - tail).abs() < 1e-12, "{value} against {tail}");
}

#[test]
fn the_effective_number_of_assets_is_held_at_its_floor() {
    let assets = assets();
    let concentrated = assets.optimise(Method::MinimumVariance).build().unwrap();
    let effective =
        |w: &std::collections::BTreeMap<String, f64>| 1.0 / w.values().map(|x| x * x).sum::<f64>();
    assert!(effective(concentrated.results.weights.as_ref().unwrap()) < 3.0);
    for method in [Method::MinimumVariance, Method::MaximumSharpe] {
        let spread = assets
            .optimise(method)
            .min_effective_assets(4.0)
            .build()
            .unwrap();
        assert_eq!(spread.results.status.to_string(), "optimal", "{method:?}");
        let k = effective(spread.results.weights.as_ref().unwrap());
        assert!(k >= 4.0 - 1e-6, "{method:?}: {k}");
        assert!(
            spread
                .report
                .to_text(None)
                .contains("at least 4 effective assets")
        );
    }
    // Limits that leave fewer effective assets are reported before solving.
    assert_eq!(
        paths(
            assets
                .optimise(Method::MinimumVariance)
                .asset_bounds("Gold", 0.6, 1.0)
                .min_effective_assets(3.0)
                .build()
        ),
        ["options.min_effective_assets"]
    );
    assert_eq!(
        paths(
            assets
                .optimise(Method::RiskParity)
                .min_effective_assets(3.0)
                .build()
        ),
        ["options.min_effective_assets"]
    );
}

#[test]
fn resampled_weights_are_the_average_of_the_simulated_optima() {
    let assets = assets();
    let once = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.6)
        .build()
        .unwrap();
    let resampled = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.6)
        .resample(50)
        .build()
        .unwrap();
    let r = resampled.results.resampled.as_ref().unwrap();
    assert_eq!(r.solved, 50);
    let average = r.weights.as_ref().unwrap();
    let weights = resampled.results.weights.as_ref().unwrap();
    for (label, w) in r.labels.iter().zip(average) {
        assert_eq!(weights[label], *w);
    }
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-9);
    assert!(weights.values().all(|w| *w <= 0.6 + 1e-6));
    // The solution is the optimum for the estimates themselves.
    assert_eq!(
        resampled.results.solution.weights,
        once.results.solution.weights
    );
    assert_ne!(resampled.results.weights, once.results.weights);
    // The same inputs give the same weights.
    let again = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.6)
        .resample(50)
        .build()
        .unwrap();
    assert_eq!(again.results.weights, resampled.results.weights);
    let text = resampled.report.to_text(None);
    assert!(text.contains("50 histories of 360 monthly"), "{text}");
    assert_eq!(
        paths(assets.optimise(Method::MinimumCvar).resample(50).build()),
        ["options.resample"]
    );
}

#[test]
fn walk_forward_refits_every_so_many_periods() {
    let assets = assets();
    let result = assets
        .backtest(Method::MinimumVariance)
        .walk_forward(Years(5))
        .rebalance(Rebalance::every(12))
        .build()
        .unwrap();
    let run = result.results.strategies[0].out_of_sample.as_ref().unwrap();
    let first = run.dates[0];
    let positions: Vec<usize> = run
        .trades
        .iter()
        .map(|t| run.dates.iter().position(|d| *d == t.date).unwrap())
        .collect();
    assert!(!positions.is_empty() && run.trades.iter().all(|t| t.refit));
    for (k, p) in positions.iter().enumerate() {
        assert_eq!(*p, 12 * (k + 1) - 1, "trade {k} after {first}");
    }
    let text = result.report.to_text(None);
    assert!(text.contains("rebalanced every 12 periods"), "{text}");
    assert_eq!(
        paths(
            assets
                .backtest(Method::MinimumVariance)
                .rebalance(Rebalance::every(0))
                .build()
        ),
        ["rebalance.periods"]
    );
}
