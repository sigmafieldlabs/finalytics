//! Checks the performance budgets in `docs/performance.md` on this machine, natively:
//!
//! - building the books for 100,000 transactions over 10 years for 2,000 instruments in under
//!   2 seconds;
//! - a 500-asset quadratic programme with linear constraints in under 0.5 seconds;
//! - a walk-forward comparison of seven methods on the S&P 500 prices in `tests/sp500` (55
//!   stocks, daily, 5-year window, 67 quarterly refits each), with its optimisation report, in
//!   under 5 seconds;
//! - a screen of every measure on 5,000 securities with up to 10 years of daily closes, volumes
//!   and dividends (about 12 million closes), with its report, in under 1 second.
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

use finalytics::backtest::Years;
use finalytics::formats::PortfolioDocument;
use finalytics::opt::Objective;
use finalytics::{Assets, Books, Method};
use finalytics_bench::{large_portfolio, large_problem, large_universe};

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

    let assets = Assets::builder()
        .prices_csv(include_str!("../../../../tests/sp500/prices.csv"))
        .risk_free_asset("T-Bills")
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    let comparison = assets
        .compare([
            Method::EqualWeight,
            Method::InverseVolatility,
            Method::MinimumVariance,
            Method::MaximumSharpe,
            Method::MaximumDiversification,
            Method::RiskParity,
            Method::HierarchicalRiskParity,
        ])
        .max_weight(0.1)
        .walk_forward(Years(5))
        .trading_cost_bps(10.0);
    let comparison = time(3, || {
        comparison.build().unwrap_or_else(|e| panic!("{e}"));
    });

    let universe = large_universe(5_000, 10);
    let mut matched = 0;
    let screen = time(5, || {
        matched = universe
            .screen()
            .build()
            .unwrap_or_else(|e| panic!("{e}"))
            .results
            .coverage
            .matched;
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
        (
            "screen of 5,000 securities by 10 years of daily closes",
            screen,
            1.0,
        ),
    ];
    let mut failed = false;
    for (name, seconds, budget) in checks {
        let verdict = if seconds <= budget { "within" } else { "OVER" };
        println!("{name}: {seconds:.3} s ({verdict} the budget of {budget} s)");
        failed |= seconds > budget;
    }
    if matched == 0 {
        println!("the screen kept no security");
        failed = true;
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
