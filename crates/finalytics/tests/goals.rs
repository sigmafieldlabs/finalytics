//! Goals kept with a portfolio: read from a goals file, saved in the portfolio's document, and
//! planned as they are.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::planning::{Flow, Frequency, Goal, Inflation, Mix, Model, Month};
use finalytics::{Assets, Error, PlanOptions, Portfolio, PortfolioCsv, datasets};

const TRANSACTIONS: &str = include_str!("../data/portfolio/transactions.csv");
const INSTRUMENTS: &str = include_str!("../data/portfolio/instruments.csv");
const ACCOUNTS: &str = include_str!("../data/portfolio/accounts.csv");
const PRICES: &str = include_str!("../data/portfolio/prices.csv");
const FX: &str = include_str!("../data/portfolio/fx.csv");

const GOALS: &str = "\
goal,type,amount,by,every,start,end,on,growth,name,weights,month,chance,spare,start_value,plan_start,inflation,fee,model,block,simulations,seed
House,target,120000,2030-06,,,,,,,,,0.85,0.95,25000,2026-11,0.025,0.002,bootstrap,6,2000,7
House,saving,1000,,month,2026-11,2030-05,,,Monthly saving,,,,,,,,,,,,
House,spending,5000,,,,,2028-03,0,Car,,,,,,,,,,,,
House,mix,,,,,,,,Balanced,US Equity=0.6;Bonds=0.4,,,,,,,,,,,
House,alternative,,,,,,,,Cautious,US Equity=0.3;Bonds=0.7,,,,,,,,,,,
Retirement,payments,3000,,month,2040-01,2060-12,,,,,,,,400000,,0.02,,historical,,,
Retirement,glide,,,,,,,,Glide path,US Equity=0.8;Bonds=0.2,2026-11,,,,,,,,,,
Retirement,glide,,,,,,,,,US Equity=0.4;Bonds=0.6,2040-01,,,,,,,,,,
";

fn month(text: &str) -> Month {
    text.parse().unwrap()
}

fn weights(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(a, w)| ((*a).to_owned(), *w)).collect()
}

fn portfolio() -> Portfolio {
    Portfolio::from_csv(
        &PortfolioCsv::new(TRANSACTIONS)
            .instruments(INSTRUMENTS)
            .accounts(ACCOUNTS)
            .prices(PRICES)
            .fx(FX)
            .goals(GOALS),
    )
    .unwrap()
}

fn paths(result: Result<impl std::fmt::Debug, Error>) -> Vec<String> {
    match result {
        Err(Error::Validation(report)) => report.issues().iter().map(|i| i.path.clone()).collect(),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn a_goals_file_gives_each_goal_as_a_plan() {
    let portfolio = portfolio();
    let goals = portfolio.goals();
    let names: Vec<&str> = goals.iter().map(|g| g.name.as_deref().unwrap()).collect();
    assert_eq!(names, ["House", "Retirement"]);
    let house = &goals[0];
    let mut expected = PlanOptions::default();
    expected.goal = Some(Goal::Target {
        name: Some("House".to_owned()),
        amount: 120_000.0,
        by: month("2030-06"),
    });
    expected.chance = 0.85;
    expected.spare = Some(0.95);
    expected.start = Some(month("2026-11"));
    expected.start_value = 25_000.0;
    expected.flows = vec![
        Flow::Saving {
            name: Some("Monthly saving".to_owned()),
            amount: 1000.0,
            on: None,
            every: Some(Frequency::Month),
            start: Some(month("2026-11")),
            end: Some(month("2030-05")),
            growth: None,
        },
        Flow::Spending {
            name: Some("Car".to_owned()),
            amount: 5000.0,
            on: Some(month("2028-03")),
            every: None,
            start: None,
            end: None,
            growth: Some(0.0),
        },
    ];
    expected.weights = Some(Mix::Fixed {
        name: Some("Balanced".to_owned()),
        weights: weights(&[("US Equity", 0.6), ("Bonds", 0.4)]),
    });
    expected.alternatives = vec![Mix::Fixed {
        name: Some("Cautious".to_owned()),
        weights: weights(&[("US Equity", 0.3), ("Bonds", 0.7)]),
    }];
    expected.fee = 0.002;
    expected.inflation = Some(Inflation::Rate { rate: 0.025 });
    expected.model = Model::Bootstrap { block: 6.0 };
    expected.simulations = 2000;
    expected.seed = 7;
    // A goal takes the portfolio's base currency.
    expected.currency = Some(portfolio.base_currency());
    expected.name = Some("House".to_owned());
    assert_eq!(house, &expected);
    let retirement = &goals[1];
    assert_eq!(retirement.model, Model::Historical);
    assert_eq!(retirement.start_value, 400_000.0);
    let Some(Mix::GlidePath { name, points }) = &retirement.weights else {
        panic!("a glide path")
    };
    assert_eq!(name.as_deref(), Some("Glide path"));
    assert_eq!(
        points.keys().copied().collect::<Vec<_>>(),
        [month("2026-11"), month("2040-01")]
    );
}

#[test]
fn goals_are_saved_with_the_portfolio_and_planned_as_they_are() {
    let portfolio = portfolio();
    let read = Portfolio::from_json(&portfolio.to_json()).unwrap();
    assert_eq!(read.goals(), portfolio.goals());
    // They stay with the portfolio as it changes.
    let moved = read.with_market(read.market().clone()).unwrap();
    assert_eq!(moved.goals(), portfolio.goals());
    // A portfolio without goals writes the same document as before.
    let plain = datasets::portfolio();
    assert!(plain.goals().is_empty());
    assert!(!plain.to_json().contains("\"goals\""));
    // A stored goal plans as the same goal built step by step.
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()
        .unwrap();
    let stored = assets.plan_with(&portfolio.goals()[0]).unwrap();
    assert_eq!(stored.results.simulations, 2000);
    assert_eq!(stored.report.name.as_deref(), Some("House"));
    // The goal read back from the document plans the same: a saved portfolio and the same
    // history reproduce the plan.
    let again = assets.plan_with(&read.goals()[0]).unwrap();
    assert_eq!(again.results, stored.results);
}

#[test]
fn every_problem_in_a_goals_file_is_reported_by_line() {
    let bad = "\
goal,type,amount,by,weights,month
House,target,1000,2030-06,,
House,mix,,,US Equity=lots,
Car,saving,500,,,
Trip,holiday,1000,,,
House,target,1000,2031-01,,
Boat,target,abc,2030-01,,
";
    let found = paths(Portfolio::from_csv(
        &PortfolioCsv::new(TRANSACTIONS)
            .instruments(INSTRUMENTS)
            .accounts(ACCOUNTS)
            .goals(bad),
    ));
    assert_eq!(
        found,
        [
            "goals.line 3 (weights)",
            "goals.line 5 (type)",
            "goals.line 6 (type)",
            "goals.line 7 (amount)",
            "goals.goal",
        ],
        "{found:?}"
    );
    // Goals kept with a portfolio need names, no two the same.
    let mut unnamed = portfolio().goals()[0].clone();
    unnamed.name = None;
    let found = paths(datasets::portfolio().with_goals([unnamed]));
    assert_eq!(found, ["goals[0].name"]);
    let twice = portfolio().goals()[0].clone();
    let found = paths(datasets::portfolio().with_goals([twice.clone(), twice]));
    assert_eq!(found, ["goals[1].name"]);
}
