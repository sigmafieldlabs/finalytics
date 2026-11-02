//! Benchmarks of the engine's main operations, at the sizes of the performance budgets in
//! `docs/performance.md`.
//!
//! ```sh
//! cargo bench -p finalytics-bench
//! ```

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use std::collections::BTreeMap;

use finalytics::Books;
use finalytics::opt::{
    BlackLitterman, Linkage, Objective, Prior, Resampling, View, ViewUncertainty,
};
use finalytics::performance::{FlowTiming, Performance};
use finalytics::risk::{self, CovarianceMethod, StatisticsOptions, TailMethod};
use finalytics_bench::{factor_returns, large_factor_problem, large_portfolio, large_problem};

fn ledger(c: &mut Criterion) {
    let large = large_portfolio(2_000, 100_000, 10);
    let mut group = c.benchmark_group("ledger");
    group.sample_size(10);
    group.bench_function("books, 100,000 transactions", |b| {
        b.iter(|| Books::build(&large.portfolio, &large.market))
    });
    let books = Books::build(&large.portfolio, &large.market).unwrap_or_else(|e| panic!("{e}"));
    group.bench_function("value series, 120 month ends", |b| {
        b.iter(|| books.value_series(&large.month_ends, &large.market))
    });
    group.bench_function("holding series, 120 month ends", |b| {
        b.iter(|| books.holding_series(&large.month_ends, &large.market))
    });
    let values = books
        .value_series(&large.month_ends, &large.market)
        .unwrap_or_else(|e| panic!("{e}"));
    group.bench_function("performance, 120 month ends", |b| {
        b.iter(|| Performance::from_values(&values, FlowTiming::StartOfDay))
    });
    group.finish();
}

fn risk_measures(c: &mut Criterion) {
    let table = factor_returns(500, 1_000);
    let returns = table.columns()[0].clone();
    let mut group = c.benchmark_group("risk");
    group.bench_function("statistics, 1,000 returns", |b| {
        b.iter(|| risk::statistics(&returns, &StatisticsOptions::default()))
    });
    group.bench_function("Cornish-Fisher tail risk, 1,000 returns", |b| {
        b.iter(|| risk::tail_risk(&returns, 0.99, TailMethod::CornishFisher))
    });
    group.sample_size(10);
    group.bench_function("Ledoit-Wolf covariance, 500 assets by 1,000 returns", |b| {
        b.iter(|| risk::covariance(&table, CovarianceMethod::LedoitWolf))
    });
    group.bench_function(
        "statistical factor model, 10 factors, 500 assets by 1,000 returns",
        |b| b.iter(|| risk::statistical_model(&table, 10)),
    );
    let factors = factor_returns(3, 1_000);
    group.bench_function(
        "regression factor model, 3 factors, 500 assets by 1,000 returns",
        |b| b.iter(|| risk::regression_model(&table, &factors)),
    );
    group.finish();
}

fn optimisation(c: &mut Criterion) {
    let problem = large_problem(500);
    let mut group = c.benchmark_group("optimisation, 500 assets");
    group.sample_size(10);
    for (name, objective) in [
        ("minimum variance", Objective::MinimumVariance),
        (
            "maximum Sharpe ratio",
            Objective::MaximumSharpe {
                risk_free_rate: 0.02,
            },
        ),
        ("risk parity", Objective::RiskBudgeting { budgets: None }),
        (
            "hierarchical risk parity",
            Objective::HierarchicalRiskParity {
                linkage: Linkage::Single,
            },
        ),
    ] {
        let problem = if matches!(
            objective,
            Objective::RiskBudgeting { .. } | Objective::HierarchicalRiskParity { .. }
        ) {
            problem.clone().with_constraints(Default::default())
        } else {
            problem.clone()
        };
        group.bench_function(name, |b| {
            b.iter_batched(
                || problem.clone(),
                |p| p.solve(&objective),
                BatchSize::LargeInput,
            )
        });
    }
    let covariance = problem
        .covariance
        .clone()
        .unwrap_or_else(|| unreachable!("the budget's problem has a covariance matrix"));
    let views = (0..10)
        .map(|k| View {
            portfolio: BTreeMap::from([
                (problem.labels[k].clone(), 1.0),
                (problem.labels[k + 10].clone(), -1.0),
            ]),
            expected_return: 0.01,
            uncertainty: ViewUncertainty::Confidence { confidence: 0.5 },
        })
        .collect();
    let mut black_litterman = BlackLitterman::new(
        covariance,
        Prior::Market {
            weights: vec![1.0; 500],
            risk_aversion: 2.5,
            risk_free_rate: 0.0,
        },
    );
    black_litterman.views = views;
    group.bench_function("Black-Litterman, 10 views", |b| {
        b.iter(|| black_litterman.estimate())
    });
    let factor_problem = large_factor_problem(500);
    for (name, objective) in [
        (
            "minimum variance, 10-factor model",
            Objective::MinimumVariance,
        ),
        (
            "maximum Sharpe ratio, 10-factor model",
            Objective::MaximumSharpe {
                risk_free_rate: 0.02,
            },
        ),
    ] {
        group.bench_function(name, |b| {
            b.iter_batched(
                || factor_problem.clone(),
                |p| p.solve(&objective),
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();

    let small = large_problem(50);
    let mut group = c.benchmark_group("optimisation, 50 assets");
    group.sample_size(10);
    group.bench_function("resampled mean-variance, 100 samples", |b| {
        b.iter(|| {
            small.resampled(
                &Objective::MeanVariance { risk_aversion: 4.0 },
                &Resampling::new(100, 120, 1),
            )
        })
    });
    group.finish();
}

criterion_group!(benches, ledger, risk_measures, optimisation);
criterion_main!(benches);
