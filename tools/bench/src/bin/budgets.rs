//! Checks the performance budgets in `docs/performance.md` on this machine, natively:
//!
//! - building the books for 100,000 transactions over 10 years for 2,000 instruments in under
//!   2 seconds;
//! - a 500-asset quadratic programme with linear constraints in under 0.5 seconds;
//! - a walk-forward comparison of seven methods on the S&P 500 dataset (55 stocks, daily, 5-year
//!   window, 67 quarterly refits each) in under 5 seconds.
//!
//! ```sh
//! cargo run --release -p finalytics-bench --bin budgets
//! cargo run --release -p finalytics-bench --bin budgets -- --write-document portfolio.json
//! ```
//!
//! With `--write-document`, it also writes the large portfolio as a portfolio document, for the
//! WebAssembly budget check in `bindings/wasm/test/budget.mjs`. Each measurement is the median of
//! five runs (three for the comparison).

// A command-line tool reports on standard output.
#![allow(clippy::print_stdout)]

use std::process::ExitCode;

use web_time::Instant;

use finalytics::backtest::{BacktestOptions, Strategy, Validation};
use finalytics::formats::PortfolioDocument;
use finalytics::opt::{Bound, Objective};
use finalytics::{Assets, Books, ObjectiveName, OptimiseOptions, datasets};
use finalytics_bench::{large_portfolio, large_problem};

fn median(mut seconds: Vec<f64>) -> f64 {
    seconds.sort_by(f64::total_cmp);
    seconds[seconds.len() / 2]
}

fn time(runs: usize, mut f: impl FnMut()) -> f64 {
    median(
        (0..runs)
            .map(|_| {
                let started = Instant::now();
                f();
                started.elapsed().as_secs_f64()
            })
            .collect(),
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let large = large_portfolio(2_000, 100_000, 10);

    if let Some(path) = args
        .iter()
        .position(|a| a == "--write-document")
        .and_then(|i| args.get(i + 1))
    {
        let mut document = PortfolioDocument::new(large.portfolio.clone());
        document.market = Some(large.market.clone());
        if let Err(e) = std::fs::write(path, document.to_json()) {
            eprintln!("could not write {path}: {e}");
            return ExitCode::FAILURE;
        }
        println!("wrote {path}");
    }

    let books = time(5, || {
        Books::build(&large.portfolio, &large.market).unwrap_or_else(|e| panic!("{e}"));
    });
    let problem = large_problem(500);
    let mut status = None;
    let qp = time(5, || {
        let solution = problem
            .solve(&Objective::MinimumVariance)
            .unwrap_or_else(|e| panic!("{e}"));
        status = Some(solution.status);
    });

    let assets = Assets::from_prices(&datasets::sp500())
        .and_then(|a| a.with_risk_free_asset("T-Bills"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut options = BacktestOptions::default();
    for objective in [
        ObjectiveName::EqualWeight,
        ObjectiveName::InverseVolatility,
        ObjectiveName::MinimumVariance,
        ObjectiveName::MaximumSharpe,
        ObjectiveName::MaximumDiversification,
        ObjectiveName::RiskBudgeting,
        ObjectiveName::HierarchicalRiskParity,
    ] {
        let mut method = OptimiseOptions::default();
        method.objective = objective;
        method.bounds = Some(Bound { min: 0.0, max: 0.1 });
        options.strategies.push(Strategy::optimise(method));
    }
    options.validation = Validation::walk_forward(5.0);
    options.trading_cost_bps = 10.0;
    let comparison = time(3, || {
        assets.simulate(&options).unwrap_or_else(|e| panic!("{e}"));
    });

    let checks = [
        (
            "books for 100,000 transactions, 2,000 instruments, 10 years",
            books,
            2.0,
        ),
        (
            "500-asset quadratic programme with bounds and groups",
            qp,
            0.5,
        ),
        (
            "walk-forward comparison of seven methods on 55 stocks, daily",
            comparison,
            5.0,
        ),
    ];
    let mut failed = false;
    for (name, seconds, budget) in checks {
        let verdict = if seconds <= budget { "within" } else { "OVER" };
        println!("{name}: {seconds:.3} s ({verdict} the budget of {budget} s)");
        failed |= seconds > budget;
    }
    if status.is_none_or(|s| !s.has_solution()) {
        println!("the quadratic programme was not solved: {status:?}");
        failed = true;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
