//! The front doors: `Assets` for prices or returns, and `Portfolio` for a portfolio's records.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::opt::{Bound, RebalanceOptions};
use finalytics::{
    Assets, Error, HoldingKey, ObjectiveName, OptimiseOptions, Portfolio, PortfolioCsv, datasets,
};

fn asset_classes() -> Assets {
    Assets::from_prices(&datasets::asset_class_prices())
        .unwrap()
        .with_risk_free_rate(0.02)
}

fn paths(result: Result<impl std::fmt::Debug, Error>) -> Vec<String> {
    match result {
        Err(Error::Validation(report)) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn the_asset_class_dataset_is_thirty_years_of_months() {
    let assets = asset_classes();
    assert_eq!(
        assets.labels(),
        [
            "US Equity",
            "Intl Equity",
            "Bonds",
            "T-Bills",
            "Gold",
            "Real Estate"
        ]
    );
    assert_eq!(assets.dates().len(), 360);
    assert_eq!(assets.periods_per_year(), 12.0);
    assert!(assets.warnings().is_empty());
    let stats = assets.statistics().unwrap();
    assert_eq!(stats.len(), 6);
    let bills = &stats[3];
    assert_eq!(bills.asset, "T-Bills");
    assert!(bills.statistics.volatility < 0.01);
    let covariance = assets.covariance().unwrap();
    // Annualised: the equity variance is of the order of 0.15^2, not 0.15^2 / 12.
    assert!(covariance.matrix.get(0, 0) > 0.01);
    let correlation = assets.correlation().unwrap();
    assert!((correlation.get(0, 0) - 1.0).abs() < 1e-12);
}

#[test]
fn the_optimiser_keeps_to_its_limits() {
    let assets = asset_classes();
    let result = assets
        .optimiser()
        .maximum_sharpe()
        .bounds(0.0, 0.4)
        .asset_bounds("Gold", 0.0, 0.1)
        .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
        .solve()
        .unwrap();
    assert!(result.status.has_solution());
    let weights = result.weights.unwrap();
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-6);
    assert!(weights.values().all(|w| *w >= -1e-6 && *w <= 0.4 + 1e-6));
    assert!(weights["Gold"] <= 0.1 + 1e-6);
    let equity = weights["US Equity"] + weights["Intl Equity"];
    assert!((0.3 - 1e-6..=0.6 + 1e-6).contains(&equity));
    let (r, v) = (result.expected_return.unwrap(), result.volatility.unwrap());
    assert!((result.sharpe_ratio.unwrap() - (r - 0.02) / v).abs() < 1e-12);
    assert_eq!(result.risk_free_rate, 0.02);
}

#[test]
fn every_objective_can_be_named() {
    let assets = asset_classes();
    let budgets: BTreeMap<String, f64> = assets
        .labels()
        .iter()
        .map(|l| (l.clone(), 1.0 / 6.0))
        .collect();
    let runs = [
        assets.optimiser().minimum_variance(),
        assets.optimiser().mean_variance(4.0),
        assets.optimiser().target_return(0.06),
        assets.optimiser().target_volatility(0.08),
        assets.optimiser().maximum_sharpe(),
        assets.optimiser().maximum_diversification(),
        assets.optimiser().minimum_cvar(0.95),
        assets.optimiser().risk_budgeting(budgets),
        assets
            .optimiser()
            .hierarchical_risk_parity(Default::default()),
    ];
    for run in runs {
        let name = run.options().objective;
        let result = run.solve().unwrap();
        assert!(result.status.has_solution(), "{name:?}");
        let total: f64 = result.weights.unwrap().values().sum();
        assert!((total - 1.0).abs() < 1e-5, "{name:?}: {total}");
    }
}

#[test]
fn options_read_from_json() {
    let options: OptimiseOptions = serde_json::from_str(
        r#"{
            "objective": "target_volatility",
            "target": 0.08,
            "bounds": {"min": 0.0, "max": 0.5},
            "groups": {"Equity": {"assets": ["US Equity", "Intl Equity"], "min": 0.2, "max": 0.7}}
        }"#,
    )
    .unwrap();
    assert_eq!(options.objective, ObjectiveName::TargetVolatility);
    let result = asset_classes().optimise(&options).unwrap();
    assert!(result.volatility.unwrap() <= 0.08 + 1e-6);
    let defaults: OptimiseOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults, OptimiseOptions::default());
    assert_eq!(defaults.objective, ObjectiveName::MaximumSharpe);
}

#[test]
fn mistakes_in_the_options_are_located() {
    let assets = asset_classes();
    let mut options = OptimiseOptions::default();
    options.objective = ObjectiveName::TargetReturn;
    assert_eq!(paths(assets.optimise(&options)), ["options.target"]);
    let mut options = OptimiseOptions::default();
    options
        .asset_bounds
        .insert("Crypto".into(), Bound { min: 0.0, max: 0.1 });
    options.expected_returns.insert("Stocks".into(), 0.07);
    assert_eq!(
        paths(assets.optimise(&options)),
        [
            "options.expected_returns.Stocks",
            "options.asset_bounds.Crypto"
        ]
    );
}

#[test]
fn a_risk_free_asset_is_left_out_of_optimisation() {
    let assets = Assets::from_prices(&datasets::asset_class_prices())
        .unwrap()
        .with_risk_free_asset("T-Bills")
        .unwrap();
    assert_eq!(assets.risk_free_asset(), Some("T-Bills"));
    assert_eq!(assets.risky_assets().len(), 5);
    let bills = assets.mean_returns()[3];
    assert_eq!(assets.risk_free_rate(), bills);
    let result = assets.optimiser().maximum_sharpe().solve().unwrap();
    let weights = result.weights.unwrap();
    assert!(!weights.contains_key("T-Bills"));
    assert_eq!(result.risk_free_rate, bills);
    assert_eq!(
        paths(assets.optimiser().asset_bounds("T-Bills", 0.0, 0.2).solve()),
        ["options.asset_bounds.T-Bills"]
    );
    assert_eq!(
        paths(asset_classes().with_risk_free_asset("Cash")),
        ["risk_free_asset"]
    );
}

#[test]
fn the_frontier_rises_with_risk() {
    let assets = asset_classes();
    let points = assets.optimiser().bounds(0.0, 0.5).frontier(8).unwrap();
    assert_eq!(points.len(), 8);
    for pair in points.windows(2) {
        assert!(pair[1].expected_return >= pair[0].expected_return - 1e-9);
        assert!(pair[1].volatility >= pair[0].volatility - 1e-9);
    }
}

#[test]
fn assets_read_from_csv_in_the_order_of_the_file() {
    let csv = "date,ZZZ,AAA\n2026-01-30,100,50\n2026-02-27,101,49\n2026-03-31,99,52\n\
               2026-04-30,,53\n2026-05-29,103,51\n";
    let assets = Assets::from_csv(csv).unwrap();
    assert_eq!(assets.labels(), ["ZZZ", "AAA"]);
    // The date without a price for ZZZ is left out, with a warning.
    assert_eq!(assets.dates().len(), 3);
    assert_eq!(assets.warnings().len(), 1);
    assert_eq!(assets.periods_per_year(), 12.0);
}

#[test]
fn the_portfolio_dataset_reports_in_full() {
    let portfolio = datasets::portfolio();
    assert_eq!(portfolio.data().transactions.len(), 148);
    assert_eq!(portfolio.data().accounts.len(), 2);
    assert_eq!(portfolio.as_of().unwrap().to_string(), "2026-10-30");
    let report = portfolio.report(&datasets::report_options()).unwrap();
    assert!(report.attribution.is_some());
    assert_eq!(report.scenarios.unwrap().scenarios.len(), 4);
    let weights = portfolio.weights(None).unwrap();
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-9);
    let performance = portfolio.performance().unwrap();
    assert_eq!(performance.dates().len(), portfolio.dates().len());
}

#[test]
fn a_portfolio_hands_its_holdings_to_the_optimiser() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    assert_eq!(assets.labels(), ["BOND", "GOLD", "INTL", "REIT", "USEQ"]);
    assert_eq!(assets.periods_per_year(), 252.0);
    let result = assets
        .optimiser()
        .minimum_variance()
        .bounds(0.05, 0.5)
        .solve()
        .unwrap();
    let targets: BTreeMap<HoldingKey, f64> = result
        .weights
        .unwrap()
        .into_iter()
        .map(|(id, w)| (id.parse().unwrap(), w))
        .collect();
    let proposal = portfolio
        .rebalance(&targets, &RebalanceOptions::default())
        .unwrap();
    assert_eq!(proposal.accounts.len(), 2);
    let traded = portfolio.with_rebalance(&proposal).unwrap();
    let after = traded.weights(None).unwrap();
    for (key, target) in &targets {
        assert!((after[key] - target).abs() < 0.01, "{key}: {}", after[key]);
    }
}

#[test]
fn a_portfolio_from_its_transactions_alone() {
    let transactions = "date,type,instrument,quantity,price,amount,currency
2026-01-02,deposit,,,,20000,USD
2026-01-02,buy,VOO,20,500,,
2026-01-02,buy,AGG,100,98,,
";
    let prices = "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n";
    let portfolio = Portfolio::from_csv(&PortfolioCsv::new(transactions).prices(prices)).unwrap();
    assert_eq!(portfolio.base_currency().code(), "USD");
    assert_eq!(portfolio.data().accounts[0].id.as_str(), "main");
    let value = portfolio.valuation(None).unwrap().total_base;
    assert_eq!(value.to_string(), "20120.0");
    let bad = "date,type,amount,currency\n2026-01-02,deposit,-5,USD\n";
    let found = paths(Portfolio::from_csv(
        &PortfolioCsv::new(bad).prices("date,VOO\n2026-13-01,1\n"),
    ));
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn base_currency_cash_is_riskless() {
    let portfolio = datasets::portfolio();
    let report = portfolio
        .overview_report(&datasets::report_options())
        .unwrap();
    let risk = report.risk.unwrap();
    let labels = risk.covariance.labels();
    let position = |name: &str| labels.iter().position(|l| l == name).unwrap();
    let (usd, eur) = (position("USD cash"), position("EUR cash"));
    for j in 0..labels.len() {
        assert_eq!(risk.covariance.get(usd, j), 0.0);
        assert_eq!(risk.covariance.get(j, usd), 0.0);
    }
    // Cash in another currency carries the exchange rate's risk.
    assert!(risk.covariance.get(eur, eur) > 0.0);
    let frontier = report.frontier.unwrap();
    assert!(frontier.iter().all(|p| p.weights[usd] == 0.0));
    let invested: f64 = frontier[0].weights.iter().sum();
    assert!((invested - 1.0).abs() < 1e-6);
    // The least risky invested portfolio still has risk: it is not all cash.
    assert!(frontier[0].volatility > 0.01);
}

#[test]
fn fees_are_their_own_line_in_contribution() {
    let portfolio = datasets::portfolio();
    let report = portfolio
        .performance_report(&datasets::report_options())
        .unwrap();
    assert!(report.expenses < 0.0);
    let usd = report
        .contributions
        .iter()
        .find(|c| c.holding.to_string() == "USD cash")
        .unwrap();
    // The advisory fee is not a loss on the dollars it was paid from.
    assert!(usd.return_base.unwrap().abs() < 1e-12);
    let total: f64 = report.contributions.iter().map(|c| c.contribution).sum();
    let itd = report.periods.iter().find(|p| p.label == "ITD").unwrap();
    let twr = itd.cumulative.unwrap();
    assert!(
        (total + report.expenses - twr).abs() < 1e-12,
        "{total} {twr}"
    );
    let tables = report.tables();
    let table = tables.iter().find(|t| t.name == "contributions").unwrap();
    let last = table.rows.last().unwrap();
    assert_eq!(
        last[0],
        finalytics::reports::Cell::Text("Fees and taxes".into())
    );
}
