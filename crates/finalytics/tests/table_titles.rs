//! Every report's tables have titles that are valid Excel sheet names, so that a report exported
//! as a workbook opens with one sheet per table under its own title.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use finalytics::backtest::{Rebalance, Years};
use finalytics::planning::Frequency;
use finalytics::prelude::*;
use finalytics::reports::ReportTable;

/// The problems with `tables`' titles as Excel sheet names: at most 31 characters, none of
/// `[ ] : * ? / \`, not empty, not starting or ending with an apostrophe, and unique within the
/// workbook regardless of case.
fn problems(report: &str, tables: &[ReportTable]) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for table in tables {
        let title = &table.title;
        let problem = if title.trim().is_empty() {
            Some("is empty".to_owned())
        } else if title.chars().count() > 31 {
            Some(format!(
                "has {} characters, more than 31",
                title.chars().count()
            ))
        } else if let Some(c) = title.chars().find(|c| "[]:*?/\\".contains(*c)) {
            Some(format!("contains {c:?}"))
        } else if title.starts_with('\'') || title.ends_with('\'') {
            Some("starts or ends with an apostrophe".to_owned())
        } else if !seen.insert(title.to_lowercase()) {
            Some("is the title of another table".to_owned())
        } else {
            None
        };
        if let Some(problem) = problem {
            found.push(format!("{report}: {:?} ({}) {problem}", title, table.name));
        }
    }
    found
}

#[test]
fn every_table_title_is_a_valid_excel_sheet_name() {
    let mut found = Vec::new();

    let portfolio = datasets::portfolio();
    let report = portfolio.report_with(&datasets::report_options()).unwrap();
    found.extend(problems("portfolio", &report.tables()));
    for (name, tables) in [
        ("overview", report.overview.tables()),
        ("holdings", report.holdings.tables()),
        ("performance", report.performance.tables()),
        ("risk", report.risk.tables()),
        ("attribution", report.attribution.as_ref().unwrap().tables()),
        ("scenarios", report.scenarios.as_ref().unwrap().tables()),
    ] {
        found.extend(problems(name, &tables));
    }

    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap();
    let optimisation = assets
        .optimise(Method::MaximumSharpe)
        .max_weight(0.4)
        .group("Equity", &["US Equity", "Intl Equity"], 0.2, 0.6)
        .with_frontier()
        .build()
        .unwrap();
    found.extend(problems("optimisation", &optimisation.report.tables()));

    let comparison = assets
        .compare([Method::EqualWeight, Method::MinimumVariance])
        .weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])
        .rebalance(Rebalance::Quarterly)
        .walk_forward(Years(5))
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .start_value(100_000.0)
        .flow("2015-01-30", 10_000.0)
        .build()
        .unwrap();
    found.extend(problems("comparison", &comparison.report.tables()));

    let backtest = assets
        .backtest(Method::MinimumVariance)
        .walk_forward(Years(5))
        .benchmark([("US Equity", 0.6), ("Bonds", 0.4)])
        .start_value(100_000.0)
        .build()
        .unwrap();
    found.extend(problems("backtest", &backtest.report.tables()));

    let target = portfolio
        .assets()
        .build()
        .unwrap()
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.4)
        .build()
        .unwrap();
    let rebalancing = portfolio
        .rebalance(&target)
        .whole_units()
        .tax(0.37, 0.2)
        .build()
        .unwrap();
    found.extend(problems("rebalance", &rebalancing.report.tables()));

    let plan = assets
        .plan()
        .payments(4_000.0, "2046-11", "2066-10")
        .start_value(250_000.0)
        .saving(1_500.0, Frequency::Month)
        .weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .alternative("Cautious", [("US Equity", 0.3), ("Bonds", 0.7)])
        .inflation(0.025)
        .simulations(500)
        .build()
        .unwrap();
    found.extend(problems("plan", &plan.report.tables()));

    let universe = Universe::builder()
        .prices_csv(include_str!("../../../tests/sp500/prices.csv"))
        .fields_csv(include_str!("../../../tests/sp500/companies.csv"))
        .benchmark("T-Bills")
        .build()
        .unwrap();
    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .sort("sharpe_ratio", Order::Descending)
        .limit(5)
        .build()
        .unwrap();
    found.extend(problems("screen", &screen.report.tables()));

    assert!(found.is_empty(), "{}", found.join("\n"));
}
