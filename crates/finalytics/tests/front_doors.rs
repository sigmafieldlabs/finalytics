//! The front doors: `Assets` for prices or returns, and `Portfolio` for a portfolio's records.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::backtest::Years;
use finalytics::opt::{Bound, RebalanceOptions};
use finalytics::{
    Assets, Error, HoldingKey, Method, OptimiseOptions, Portfolio, PortfolioCsv, datasets,
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
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .asset_bounds("Gold", 0.0, 0.1)
        .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
        .build()
        .unwrap();
    assert!(result.results.status.has_solution());
    let weights = result.results.weights.unwrap();
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-6);
    assert!(weights.values().all(|w| *w >= -1e-6 && *w <= 0.4 + 1e-6));
    assert!(weights["Gold"] <= 0.1 + 1e-6);
    let equity = weights["US Equity"] + weights["Intl Equity"];
    assert!((0.3 - 1e-6..=0.6 + 1e-6).contains(&equity));
    let (r, v) = (
        result.results.expected_return.unwrap(),
        result.results.volatility.unwrap(),
    );
    assert!((result.results.sharpe_ratio.unwrap() - (r - 0.02) / v).abs() < 1e-12);
    assert_eq!(result.results.risk_free_rate, 0.02);
}

#[test]
fn every_method_can_be_named() {
    let assets = asset_classes();
    let mut runs = vec![
        assets.optimise(Method::MinimumVariance),
        assets.optimise(Method::MeanVariance).risk_aversion(4.0),
        assets.optimise(Method::TargetReturn).target(0.06),
        assets.optimise(Method::TargetVolatility).target(0.08),
        assets.optimise(Method::MaximumSharpe),
        assets.optimise(Method::MaximumDiversification),
        assets.optimise(Method::MinimumCvar).confidence(0.95),
        assets.optimise(Method::EqualWeight),
        assets.optimise(Method::InverseVolatility),
        assets.optimise(Method::HierarchicalRiskParity),
    ];
    let mut budgeted = assets.optimise(Method::RiskBudgeting);
    for label in assets.labels() {
        budgeted = budgeted.budget(label, 1.0 / 6.0);
    }
    runs.push(budgeted);
    for run in runs {
        let name = run.options().method;
        let result = run.build().unwrap();
        assert!(result.results.status.has_solution(), "{name:?}");
        let total: f64 = result.results.weights.unwrap().values().sum();
        assert!((total - 1.0).abs() < 1e-5, "{name:?}: {total}");
    }
    // Limits do not apply to the methods that take none.
    assert_eq!(
        paths(assets.optimise(Method::EqualWeight).max_weight(0.3).build()),
        ["options.max_weight"]
    );
    assert_eq!(
        paths(
            assets
                .optimise(Method::MinimumVariance)
                .max_weight(0.3)
                .bounds(0.0, 0.5)
                .build()
        ),
        ["options.max_weight"]
    );
}

#[test]
fn options_read_from_json() {
    let options: OptimiseOptions = serde_json::from_str(
        r#"{
            "method": "target_volatility",
            "target": 0.08,
            "bounds": {"min": 0.0, "max": 0.5},
            "groups": {"Equity": {"assets": ["US Equity", "Intl Equity"], "min": 0.2, "max": 0.7}}
        }"#,
    )
    .unwrap();
    assert_eq!(options.method, Method::TargetVolatility);
    let result = asset_classes().optimise_with(&options).unwrap();
    assert!(result.results.volatility.unwrap() <= 0.08 + 1e-6);
    let defaults: OptimiseOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults, OptimiseOptions::default());
    assert_eq!(defaults.method, Method::MaximumSharpe);
}

#[test]
fn mistakes_in_the_options_are_located() {
    let assets = asset_classes();
    let mut options = OptimiseOptions::default();
    options.method = Method::TargetReturn;
    assert_eq!(paths(assets.optimise_with(&options)), ["options.target"]);
    let mut options = OptimiseOptions::default();
    options
        .asset_bounds
        .insert("Crypto".into(), Bound { min: 0.0, max: 0.1 });
    options.expected_returns.insert("Stocks".into(), 0.07);
    assert_eq!(
        paths(assets.optimise_with(&options)),
        [
            "options.asset_bounds.Crypto",
            "options.expected_returns.Stocks"
        ]
    );
}

#[test]
fn every_problem_with_the_options_is_found_before_solving() {
    let assets = asset_classes();
    let issues = |optimise: finalytics::Optimise<'_>| match optimise.build() {
        Err(Error::Validation(report)) => report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect::<Vec<_>>(),
        other => panic!("expected a validation error, got {other:?}"),
    };
    // Six assets at most 10% each cannot be fully invested.
    let found = issues(assets.optimise(Method::MinimumVariance).max_weight(0.1));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "options.max_weight");
    assert!(found[0].1.contains("at least 0.1667"), "{}", found[0].1);
    for max in [1.5, 0.0, f64::NAN] {
        assert_eq!(
            paths(
                assets
                    .optimise(Method::MinimumVariance)
                    .max_weight(max)
                    .build()
            ),
            ["options.max_weight"],
            "{max}"
        );
    }
    assert_eq!(
        paths(
            assets
                .optimise(Method::MinimumVariance)
                .bounds(-0.2, 0.5)
                .build()
        ),
        ["options.bounds"]
    );
    // Every problem at once, at the options' names.
    assert_eq!(
        paths(
            assets
                .optimise(Method::MinimumCvar)
                .confidence(1.5)
                .group("Equity", &["US Equity", "Gold"], 0.6, 0.4)
                .risk_aversion(2.0)
                .build()
        ),
        [
            "options.groups.Equity",
            "options.risk_aversion",
            "options.confidence"
        ]
    );
    // A parameter the method does not use, when one method is asked for.
    let found = issues(assets.optimise(Method::MaximumSharpe).confidence(0.9));
    assert_eq!(
        found,
        [(
            "options.confidence".to_owned(),
            "maximum Sharpe does not use confidence".to_owned()
        )]
    );
    assert_eq!(
        paths(assets.backtest(Method::EqualWeight).max_weight(0.3).build()),
        ["options.max_weight"]
    );
    // With the backtest's own settings.
    assert_eq!(
        paths(
            assets
                .compare([Method::MinimumVariance])
                .max_weight(0.1)
                .walk_forward(Years(50))
                .build()
        ),
        ["options.max_weight", "validation.years"]
    );
    // A comparison applies each to the methods that take it, and notes the rest.
    let compared = assets
        .compare([Method::EqualWeight, Method::MinimumCvar])
        .max_weight(0.3)
        .confidence(0.9)
        .build()
        .unwrap();
    assert!(!compared.report.strategies[0].notes.is_empty());
    // Settings of the assets too.
    let prices = datasets::asset_class_prices();
    assert_eq!(
        paths(
            Assets::builder()
                .prices(prices.clone())
                .periods_per_year(0.0)
                .risk_free_rate(f64::NAN)
                .build()
        ),
        ["risk_free_rate", "periods_per_year"]
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
    let result = assets.optimise(Method::MaximumSharpe).build().unwrap();
    let weights = result.results.weights.unwrap();
    assert!(!weights.contains_key("T-Bills"));
    assert_eq!(result.results.risk_free_rate, bills);
    assert_eq!(
        paths(
            assets
                .optimise(Method::MaximumSharpe)
                .asset_bounds("T-Bills", 0.0, 0.2)
                .build()
        ),
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
    let points = assets
        .optimise(Method::MinimumVariance)
        .max_weight(0.5)
        .frontier(8)
        .unwrap();
    assert_eq!(points.len(), 8);
    for pair in points.windows(2) {
        assert!(pair[1].expected_return >= pair[0].expected_return - 1e-9);
        assert!(pair[1].volatility >= pair[0].volatility - 1e-9);
    }
    // Weights by asset, each within the limit, adding up to one.
    for p in &points {
        assert_eq!(p.weights.len(), assets.labels().len());
        assert!(p.weights.values().all(|w| *w <= 0.5 + 1e-6));
        assert!((p.weights.values().sum::<f64>() - 1.0).abs() < 1e-6);
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
    assert_eq!(portfolio.data().transactions.len(), 203);
    assert_eq!(portfolio.data().accounts.len(), 2);
    assert_eq!(portfolio.as_of().unwrap().to_string(), "2026-10-30");
    let report = portfolio.report_with(&datasets::report_options()).unwrap();
    assert!(report.attribution.is_some());
    assert_eq!(report.scenarios.unwrap().scenarios.len(), 4);
    let weights = portfolio.weights().unwrap();
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-9);
    let performance = portfolio.performance().unwrap();
    assert_eq!(performance.dates().len(), portfolio.dates().len());
}

#[test]
fn a_portfolio_hands_its_holdings_to_the_optimiser() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().build().unwrap();
    // The instruments are the choices; the cash is there, riskless in dollars, moving with the
    // rate in euros.
    assert_eq!(
        assets.labels(),
        ["BND", "EXSA", "GLD", "VNQ", "VTI", "EUR cash", "USD cash"]
    );
    assert_eq!(assets.risky_assets(), ["BND", "EXSA", "GLD", "VNQ", "VTI"]);
    assert_eq!(assets.cash_assets(), ["USD cash", "EUR cash"]);
    assert_eq!(assets.risk_free_rate(), 0.0);
    let covariance = assets.covariance().unwrap().matrix;
    assert!(covariance.get(6, 6) == 0.0 && covariance.get(5, 5) > 0.0);
    assert_eq!(assets.periods_per_year(), 252.0);
    let result = assets
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.5)
        .build()
        .unwrap();
    let targets: BTreeMap<HoldingKey, f64> = result.results.holdings();
    let proposal = portfolio.rebalance(&result).build().unwrap();
    assert_eq!(proposal.results.accounts.len(), 2);
    let same = portfolio
        .rebalance_with(&targets, &RebalanceOptions::default())
        .unwrap();
    assert_eq!(same.results, proposal.results);
    let ira = portfolio
        .rebalance(&result)
        .accounts(["ira"])
        .whole_units()
        .build()
        .unwrap();
    assert_eq!(ira.results.accounts.len(), 1);
    assert_eq!(
        paths(
            portfolio
                .rebalance(&result)
                .accounts(["ira", "isa"])
                .build()
        ),
        ["accounts.isa"]
    );
    let unknown = paths(portfolio.rebalance([("VTI", 0.5), ("XYZ", 0.5)]).build());
    assert!(
        unknown.iter().all(|p| p.starts_with("targets")),
        "{unknown:?}"
    );
    let traded = portfolio.with_rebalance(&proposal).unwrap();
    let after = traded.weights().unwrap();
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
    let value = portfolio.valuation().unwrap().total_base;
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

#[test]
fn every_dataset_file_is_available_as_text() {
    assert!(datasets::FILES.iter().all(|f| datasets::file(f).is_some()));
    assert_eq!(datasets::file("prices.csv"), None);
}

#[test]
fn builders_read_files_and_build_reports() {
    let dir = std::env::temp_dir().join(format!("finalytics-builders-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["transactions", "instruments", "accounts", "prices", "fx"] {
        let text = datasets::file(&format!("portfolio/{name}.csv")).unwrap();
        std::fs::write(dir.join(format!("{name}.csv")), text).unwrap();
    }
    let portfolio = Portfolio::builder()
        .transactions(dir.join("transactions.csv"))
        .instruments(dir.join("instruments.csv"))
        .accounts(dir.join("accounts.csv"))
        .prices(dir.join("prices.csv"))
        .fx(dir.join("fx.csv"))
        .base_currency("USD")
        .build()
        .unwrap();
    assert_eq!(
        portfolio.valuation().unwrap(),
        datasets::portfolio().valuation().unwrap()
    );
    let report = portfolio
        .report()
        .options(datasets::report_options())
        .benchmark("BENCH")
        .name("Morgan family portfolio")
        .build()
        .unwrap();
    assert_eq!(report.name.as_deref(), Some("Morgan family portfolio"));
    assert!(report.attribution.is_some());
    let earlier = portfolio.report().as_of("2026-09-30").build().unwrap();
    assert_eq!(earlier.overview.provenance.as_of.to_string(), "2026-09-30");
    assert!(
        report
            .to_html(None)
            .contains("<title>Morgan family portfolio</title>")
    );
    let page = dir.join("report.html");
    finalytics::Show::save_html(&report, &page).unwrap();
    assert!(
        std::fs::read_to_string(&page)
            .unwrap()
            .starts_with("<!doctype html>")
    );
    // In a theme of its own, whatever the reader's setting.
    use finalytics::Show;
    report
        .with_theme(finalytics::Theme::Dark)
        .save_html(&page)
        .unwrap();
    assert!(
        std::fs::read_to_string(&page)
            .unwrap()
            .contains(r#"<html lang="en" data-theme="dark">"#)
    );
    assert_eq!(
        paths(
            portfolio
                .report()
                .benchmark("BENCH")
                .as_of("30/09/2026")
                .build()
        ),
        ["as_of"]
    );
    assert_eq!(
        paths(
            Portfolio::builder()
                .prices(dir.join("missing.csv"))
                .base_currency("dollars")
                .build()
        ),
        ["prices", "transactions", "base_currency"]
    );
    let assets = Assets::builder()
        .prices_file(dir.join("prices.csv"))
        .periods_per_year(252.0)
        .build()
        .unwrap();
    assert_eq!(assets.labels().len(), 6);
    assert_eq!(
        paths(
            Assets::builder()
                .risk_free_asset("T-Bills")
                .risk_free_rate(0.02)
                .build()
        ),
        ["prices", "risk_free_rate"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
