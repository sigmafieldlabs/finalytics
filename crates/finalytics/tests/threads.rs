//! With the `parallel` feature, the work that runs at once gives the same results, to the last
//! bit, on one thread as on several: comparisons with walk-forward validation, resampled
//! optimisation, the efficient frontier and screens.

#![cfg(feature = "parallel")]
#![allow(clippy::unwrap_used)]

use finalytics::backtest::{Rebalance, Years};
use finalytics::{Assets, Method, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

/// Runs `f` on a pool of `threads` threads.
fn on<R: Send>(threads: usize, f: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(f)
}

#[test]
fn a_walk_forward_comparison_is_the_same_on_any_number_of_threads() {
    let assets = assets();
    let run = || {
        let result = assets
            .compare([
                Method::EqualWeight,
                Method::MinimumVariance,
                Method::MaximumSharpe,
                Method::RiskParity,
                Method::HierarchicalRiskParity,
            ])
            .max_weight(0.6)
            .walk_forward(Years(5))
            .rebalance(Rebalance::Quarterly)
            .trading_cost_bps(10.0)
            .build()
            .unwrap();
        serde_json::to_string(&result.results).unwrap()
    };
    let one = on(1, run);
    for threads in [2, 4] {
        assert!(one == on(threads, run), "{threads} threads differ from one");
    }
}

#[test]
fn resampled_weights_are_the_same_on_any_number_of_threads() {
    let assets = assets();
    let run = || {
        let r = assets
            .optimise(Method::MaximumSharpe)
            .max_weight(0.6)
            .resample(40)
            .build()
            .unwrap()
            .results
            .resampled
            .unwrap();
        (r.weights, r.weight_deviations, r.solved, r.failed)
    };
    let one = on(1, run);
    assert_eq!(one.2, 40);
    for threads in [2, 4] {
        assert_eq!(one, on(threads, run), "{threads} threads");
    }
}

#[test]
fn the_frontier_is_the_same_on_any_number_of_threads() {
    let assets = assets();
    let run = || assets.optimise(Method::MaximumSharpe).frontier(12).unwrap();
    let one = on(1, run);
    assert!(one.len() > 2);
    for threads in [2, 4] {
        assert_eq!(one, on(threads, run), "{threads} threads");
    }
}

#[test]
fn a_screen_is_the_same_on_any_number_of_threads() {
    let universe = finalytics::Universe::builder()
        .prices_csv(include_str!("../../../tests/sp500/prices.csv"))
        .fields_csv(include_str!("../../../tests/sp500/companies.csv"))
        .risk_free_rate(0.03)
        .build()
        .unwrap()
        .with_benchmark("T-Bills")
        .unwrap();
    let screen = || {
        universe
            .screen()
            .above("return_1y", 0.0)
            .none_of("sector", ["Utilities"])
            .sort("sharpe_ratio", finalytics::screening::Order::Descending)
            .build()
            .unwrap()
    };
    let one = on(1, screen);
    let several = on(4, screen);
    assert!(one.results.matches.len() > 5);
    assert_eq!(one, several);
}
