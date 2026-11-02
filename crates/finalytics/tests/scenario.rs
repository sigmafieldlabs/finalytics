//! The shared scenario's results, which the Python and JavaScript tests must reproduce.
//!
//! `tests/scenario/expected.json` holds the results. Regenerate it after a deliberate change with
//! `UPDATE_SCENARIO=1 cargo test -p finalytics --test scenario`, and review the difference.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics::opt::{Linkage, Objective, Problem, RebalanceOptions, rebalance};
use finalytics::risk::{self, CovarianceMethod, StatisticsOptions, TailMethod};
use finalytics::{AccountId, Books, Currency, Date, Decimal, HoldingKey, InstrumentId, LotMethod};
use serde_json::{Value, json};

const INSTRUMENTS: &str = include_str!("../../../tests/scenario/instruments.csv");
const TRANSACTIONS: &str = include_str!("../../../tests/scenario/transactions.csv");
const PRICES: &str = include_str!("../../../tests/scenario/prices.csv");
const FX: &str = include_str!("../../../tests/scenario/fx.csv");
const EXPECTED: &str = "../../tests/scenario/expected.json";

fn label(key: &HoldingKey) -> String {
    match key {
        HoldingKey::Instrument(id) => id.to_string(),
        HoldingKey::Cash(c) => format!("{c} cash"),
    }
}

fn results() -> Value {
    use finalytics::formats::*;
    let options = CsvOptions::default();
    let portfolio =
        portfolio_from_csv(Currency::USD, INSTRUMENTS, TRANSACTIONS, None, options).unwrap();
    let mut market = finalytics::MarketData::new();
    for (id, prices) in prices_from_csv(PRICES, options).unwrap() {
        market = market.with_prices(id, prices);
    }
    for fx in fx_from_csv(FX, options).unwrap() {
        market = market.with_fx(fx.base, fx.quote, fx.rates);
    }
    let books = Books::build(&portfolio, &market).unwrap();
    let mut dates: Vec<Date> = market
        .all_prices()
        .flat_map(|(_, s)| s.dates().to_vec())
        .collect();
    dates.sort();
    dates.dedup();
    let last = *dates.last().unwrap();

    let valuation = books.value_on(last, &market).unwrap();
    let realised: Vec<Decimal> = books.realised_gains().iter().map(|g| g.gain_base).collect();

    let values = books.value_series(&dates, &market).unwrap();
    let performance =
        finalytics::Performance::from_values(&values, finalytics::FlowTiming::StartOfDay).unwrap();
    let holdings = books.holding_series(&dates, &market).unwrap();
    let (first, end) = (dates[0], last);
    let contributions: BTreeMap<String, f64> = performance
        .contributions(&holdings, first, end)
        .unwrap()
        .iter()
        .map(|c| (label(&c.holding), c.contribution))
        .collect();
    let returns = performance.returns().values().to_vec();
    let statistics = risk::statistics(&returns, &StatisticsOptions::default()).unwrap();
    let tail = risk::tail_risk(&returns, 0.95, TailMethod::CornishFisher).unwrap();

    let weights = risk::holding_weights(&valuation).unwrap();
    let keys: Vec<HoldingKey> = weights.iter().map(|(k, _)| k.clone()).collect();
    let held = risk::holding_returns(&books, &keys, &dates, &market).unwrap();
    let estimate = risk::covariance(&held.table, CovarianceMethod::LedoitWolf).unwrap();
    let annual = estimate.matrix.scaled(252.0);
    let w: Vec<f64> = weights.iter().map(|(_, w)| *w).collect();
    let decomposition = risk::risk_decomposition(&w, &annual).unwrap();

    let instruments: Vec<HoldingKey> = ["BOND", "SAP", "SPY"]
        .iter()
        .map(|id| HoldingKey::Instrument(InstrumentId::new(*id).unwrap()))
        .collect();
    let assets = risk::holding_returns(&books, &instruments, &dates, &market).unwrap();
    let covariance = risk::covariance(&assets.table, CovarianceMethod::Sample)
        .unwrap()
        .matrix
        .scaled(252.0);
    let problem = Problem::new(covariance);
    let minimum = problem.solve(&Objective::MinimumVariance).unwrap();
    let hrp = problem
        .solve(&Objective::HierarchicalRiskParity {
            linkage: Linkage::Single,
        })
        .unwrap();

    let targets = vec![
        (instruments[2].clone(), 0.4),
        (instruments[0].clone(), 0.4),
        (instruments[1].clone(), 0.15),
    ];
    let mut options = RebalanceOptions::default();
    options.lot_method = LotMethod::HighestCost;
    let plan = rebalance(
        &books,
        &market,
        last,
        &AccountId::new("main").unwrap(),
        &targets,
        &options,
    )
    .unwrap();
    let trades: Vec<Value> = plan
        .trades
        .iter()
        .map(|t| json!([t.instrument, t.side, t.quantity]))
        .collect();

    json!({
        "valuation_total_base": valuation.total_base,
        "realised_gains_base": realised,
        "time_weighted": performance.time_weighted(first, end),
        "money_weighted": performance.money_weighted(first, end),
        "modified_dietz": performance.modified_dietz(first, end),
        "returns": returns,
        "contributions": contributions,
        "statistics": statistics,
        "tail_risk": [tail.value_at_risk, tail.expected_shortfall],
        "weights": weights.iter().map(|(k, w)| (label(k), *w)).collect::<BTreeMap<_, _>>(),
        "shrinkage": estimate.shrinkage,
        "volatility": decomposition.volatility,
        "risk_shares": decomposition.contributions.iter().map(|c| c.share).collect::<Vec<_>>(),
        "minimum_variance": minimum.weights,
        "hierarchical_risk_parity": hrp.weights,
        "rebalance_trades": trades,
        "rebalance_realised_gain_base": plan.realised_gain_base,
        "source_digest": finalytics::reports::Source::of(&portfolio, &market).digest,
    })
}

/// Compares `actual` with `expected`: text exactly, numbers to `tolerance` relative.
fn compare(actual: &Value, expected: &Value, path: &str, tolerance: f64) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!(
                (a - e).abs() <= tolerance * e.abs().max(1e-300) || a == e,
                "{path}: {a} against {e}"
            );
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(a.len(), e.len(), "{path}: lengths differ");
            for (k, (x, y)) in a.iter().zip(e).enumerate() {
                compare(x, y, &format!("{path}[{k}]"), tolerance);
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            let keys = |m: &serde_json::Map<String, Value>| {
                let mut k: Vec<_> = m.keys().cloned().collect();
                k.sort();
                k
            };
            assert_eq!(keys(a), keys(e), "{path}: fields differ");
            for (k, v) in a {
                compare(v, &e[k], &format!("{path}.{k}"), tolerance);
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

#[test]
fn results_match_the_shared_expectations() {
    let found = results();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(EXPECTED);
    if std::env::var_os("UPDATE_SCENARIO").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&found).unwrap() + "\n").unwrap();
        return;
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    compare(&found, &expected, "results", 1e-12);
}
