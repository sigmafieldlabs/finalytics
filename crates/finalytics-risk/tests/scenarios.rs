//! Scenarios on a portfolio's books, worked by hand.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics_core::{Currency, Date, Decimal, Error, Money, TimeSeries};
use finalytics_ledger::*;
use finalytics_risk::*;

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn id(s: &str) -> InstrumentId {
    InstrumentId::new(s).unwrap()
}

fn tx(n: usize, kind: TransactionKind) -> Transaction {
    Transaction::new(
        TransactionId::new(format!("t{n}")).unwrap(),
        AccountId::new(if n.is_multiple_of(2) { "isa" } else { "main" }).unwrap(),
        d("2026-03-02"),
        kind,
    )
}

fn daily(values: &[&str]) -> TimeSeries<Decimal> {
    let first = d("2026-03-02");
    TimeSeries::new(
        (0..values.len())
            .map(|i| first.add_days(i as i32).unwrap())
            .collect(),
        values.iter().map(|v| dec(v)).collect(),
    )
    .unwrap()
}

/// SPY 80 at 507, SAP 50 at 204 EUR (1.11), BOND 200 at 100.1, 18,000 USD and 10,000 EUR on
/// 2026-03-07: 40,560 + 11,322 + 20,020 + 18,000 + 11,100 = 101,002 USD.
fn setup() -> (Books, MarketData) {
    let usd = |a: &str| Money::new(dec(a), Currency::USD);
    let buy = |n, instrument: &str, quantity: &str, price: &str| {
        tx(
            n,
            TransactionKind::Buy {
                instrument: id(instrument),
                quantity: dec(quantity),
                price: dec(price),
                fees: dec("0"),
            },
        )
    };
    let mut portfolio = PortfolioData::new(Currency::USD);
    portfolio.accounts = vec![
        Account::new(AccountId::new("main").unwrap()),
        Account::new(AccountId::new("isa").unwrap()),
    ];
    portfolio.instruments = vec![
        Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD)
            .classified("asset_class", "Equity"),
        Instrument::new(id("SAP"), InstrumentKind::Equity, Currency::EUR)
            .classified("asset_class", "Equity")
            .classified("region", "Europe"),
        Instrument::new(id("BOND"), InstrumentKind::Bond, Currency::USD)
            .classified("asset_class", "Fixed income"),
    ];
    portfolio.transactions = vec![
        tx(
            1,
            TransactionKind::Deposit {
                amount: usd("60000"),
            },
        ),
        tx(
            2,
            TransactionKind::Deposit {
                amount: usd("40000"),
            },
        ),
        buy(3, "SPY", "60", "500"),
        buy(4, "SPY", "20", "500"),
        tx(
            5,
            TransactionKind::FxConversion {
                sell: usd("22000"),
                buy: Money::new(dec("20000"), Currency::EUR),
            },
        ),
        buy(7, "SAP", "50", "200"),
        buy(6, "BOND", "200", "100"),
    ];
    let market = MarketData::new()
        .with_prices(
            id("SPY"),
            daily(&["500", "505", "498", "502", "510", "507"]),
        )
        .with_prices(
            id("SAP"),
            daily(&["200", "198", "203", "201", "199", "204"]),
        )
        .with_prices(
            id("BOND"),
            daily(&["100", "100.1", "100.3", "100.2", "100.0", "100.1"]),
        )
        .with_prices(id("INDEX"), daily(&["50", "49", "48", "51", "52", "53"]))
        .with_fx(
            Currency::EUR,
            Currency::USD,
            daily(&["1.10", "1.11", "1.09", "1.10", "1.12", "1.11"]),
        );
    (Books::build(&portfolio, &market).unwrap(), market)
}

#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "{actual} against {expected}"
    );
}

fn impact<'a>(result: &'a ScenarioResult, key: &HoldingKey) -> &'a HoldingImpact {
    result.holdings.iter().find(|h| &h.holding == key).unwrap()
}

fn instrument(s: &str) -> HoldingKey {
    HoldingKey::Instrument(id(s))
}

fn group(scheme: &str, value: &str, shock: f64) -> GroupShock {
    GroupShock {
        scheme: scheme.into(),
        value: value.into(),
        shock,
    }
}

fn validation(result: Result<ScenarioResult, Error>) -> Vec<(String, String)> {
    let Err(Error::Validation(report)) = result else {
        panic!("expected a validation error");
    };
    report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect()
}

#[test]
fn shocks_to_groups_currencies_and_instruments() {
    let (books, market) = setup();
    let mut shocks = Shocks::default();
    shocks.instruments = BTreeMap::from([(id("BOND"), -0.05)]);
    shocks.groups = vec![group("asset_class", "Equity", -0.2)];
    shocks.currencies = BTreeMap::from([(Currency::EUR, -0.1)]);
    shocks.factors = None;
    let mut options = ScenarioOptions::default();
    options.group_by = vec!["asset_class".into()];
    options.risk = None;
    let r = shock_scenario(&books, &market, d("2026-03-07"), &shocks, &options).unwrap();
    close(r.value_before, 101_002.0);
    // SPY 40,560 x 0.8; SAP 11,322 x 0.8 x 0.9; BOND 20,020 x 0.95; EUR cash 11,100 x 0.9.
    close(impact(&r, &instrument("SPY")).value_after, 32_448.0);
    let sap = impact(&r, &instrument("SAP"));
    close(sap.return_base, -0.28);
    close(sap.value_after, 8_151.84);
    assert_eq!(
        sap.source,
        ShockSource::Group {
            scheme: "asset_class".into(),
            value: "Equity".into()
        }
    );
    assert_eq!(
        impact(&r, &instrument("BOND")).source,
        ShockSource::Instrument
    );
    close(
        impact(&r, &HoldingKey::Cash(Currency::EUR)).value_after,
        9_990.0,
    );
    close(
        impact(&r, &HoldingKey::Cash(Currency::USD)).value_after,
        18_000.0,
    );
    close(r.value_after, 87_608.84);
    close(r.change, -13_393.16);
    close(r.portfolio_return, -13_393.16 / 101_002.0);
    assert!(r.not_shocked.is_empty());

    let by_class = &r.groups[0];
    let names: Vec<&str> = by_class.groups.iter().map(|g| g.group.as_str()).collect();
    assert_eq!(names, ["Equity", "Fixed income", "cash"]);
    close(by_class.groups[0].change, -8_112.0 - 3_170.16);
    let total: f64 = by_class.groups.iter().map(|g| g.contribution).sum();
    close(total, r.portfolio_return);
}

#[test]
fn an_instrument_in_two_shocked_groups_needs_its_own_shock() {
    let (books, market) = setup();
    let mut shocks = Shocks::default();
    shocks.groups = vec![
        group("asset_class", "Equity", -0.2),
        group("region", "Europe", -0.3),
        group("region", "Asia", -0.3),
    ];
    let date = d("2026-03-07");
    let options = ScenarioOptions::default();
    let issues = validation(shock_scenario(&books, &market, date, &shocks, &options));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].0, "shocks.groups");
    assert!(
        issues[0]
            .1
            .contains("SAP is in more than one shocked group")
    );

    // With its own shock, SAP takes it, and the other groups apply as usual.
    shocks.instruments.insert(id("SAP"), -0.25);
    let r = shock_scenario(&books, &market, date, &shocks, &options).unwrap();
    close(impact(&r, &instrument("SAP")).return_local, -0.25);
    close(impact(&r, &instrument("SPY")).return_local, -0.2);
    // BOND is not shocked; Asia matches nothing.
    assert_eq!(r.not_shocked, [id("BOND")]);
    assert_eq!(impact(&r, &instrument("BOND")).source, ShockSource::None);
    let warnings: Vec<&str> = r
        .warnings
        .issues()
        .iter()
        .map(|i| i.path.as_str())
        .collect();
    assert_eq!(warnings, ["shocks.groups[2]"]);
}

#[test]
fn shocks_are_validated() {
    let (books, market) = setup();
    let mut shocks = Shocks::default();
    shocks.instruments = BTreeMap::from([(id("NOPE"), -0.1), (id("SPY"), -1.5)]);
    shocks.currencies = BTreeMap::from([(Currency::USD, 0.1)]);
    let issues = validation(shock_scenario(
        &books,
        &market,
        d("2026-03-07"),
        &shocks,
        &ScenarioOptions::default(),
    ));
    let paths: Vec<&str> = issues.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "shocks.instruments.NOPE",
            "shocks.instruments.SPY",
            "shocks.currencies.USD"
        ]
    );
}

fn factor_model() -> FactorModel {
    FactorModel::new(
        vec!["SPY".into(), "SAP".into(), "BOND".into()],
        vec![vec![1.0, 0.0], vec![1.2, 0.0], vec![0.0, -5.0]],
        Matrix::new(
            vec!["equity".into(), "rates".into()],
            vec![0.04, -0.002, -0.002, 0.0001],
        )
        .unwrap(),
        vec![0.01, 0.02, 0.0001],
    )
    .unwrap()
}

#[test]
fn factor_shocks_with_and_without_propagation() {
    let (books, market) = setup();
    let mut factors = FactorShocks::new(factor_model(), BTreeMap::from([("equity".into(), -0.1)]));
    let date = d("2026-03-07");
    let options = ScenarioOptions::default();
    let shocks = |f: &FactorShocks| {
        let mut shocks = Shocks::default();
        shocks.factors = Some(f.clone());
        shocks
    };
    let r = shock_scenario(&books, &market, date, &shocks(&factors), &options).unwrap();
    close(impact(&r, &instrument("SPY")).return_local, -0.1);
    close(impact(&r, &instrument("SAP")).return_local, -0.12);
    close(impact(&r, &instrument("BOND")).return_local, 0.0);
    assert_eq!(
        r.factor_returns,
        BTreeMap::from([("equity".to_owned(), -0.1), ("rates".to_owned(), 0.0)])
    );

    // Rates are expected to move by -0.002 / 0.04 x -0.1 = 0.005 when equities fall 10%, so
    // the bond, with an exposure of -5, falls 2.5%.
    factors.propagate = true;
    let r = shock_scenario(&books, &market, date, &shocks(&factors), &options).unwrap();
    close(r.factor_returns["rates"], 0.005);
    close(impact(&r, &instrument("BOND")).return_local, -0.025);
    assert_eq!(impact(&r, &instrument("BOND")).source, ShockSource::Factor);
}

#[test]
fn risk_before_and_after_with_a_stress() {
    let (books, market) = setup();
    let labels = ["SPY", "SAP", "BOND", "EUR cash"];
    let covariance = Matrix::new(
        labels.iter().map(|s| s.to_string()).collect(),
        vec![
            0.04, 0.02, 0.0, 0.0, //
            0.02, 0.05, 0.0, 0.004, //
            0.0, 0.0, 0.0025, 0.0, //
            0.0, 0.004, 0.0, 0.01,
        ],
    )
    .unwrap();
    let stress = CorrelationStress {
        volatility_scale: 1.5,
        correlation_shift: 0.5,
    };
    let mut options = ScenarioOptions::default();
    options.group_by = vec!["asset_class".into()];
    let mut model = ScenarioRiskModel::from_covariance(covariance.clone());
    model.confidence = 0.99;
    model.stress = Some(stress);
    options.risk = Some(model);
    let mut shocks = Shocks::default();
    shocks.groups = vec![group("asset_class", "Equity", -0.2)];
    let r = shock_scenario(&books, &market, d("2026-03-07"), &shocks, &options).unwrap();

    // Holdings are in order BOND, SAP, SPY, then cash EUR, USD; USD cash is riskless.
    let order = ["BOND", "SAP", "SPY", "EUR cash"];
    let position = |l: &str| labels.iter().position(|x| *x == l).unwrap();
    let variance = |w: &[f64], c: &Matrix| -> f64 {
        let mut v = 0.0;
        for (i, a) in order.iter().enumerate() {
            for (j, b) in order.iter().enumerate() {
                v += w[i] * w[j] * c.get(position(a), position(b));
            }
        }
        v
    };
    let weights = |after: bool| -> Vec<f64> {
        r.holdings[..4]
            .iter()
            .map(|h| {
                if after {
                    h.weight_after.unwrap()
                } else {
                    h.weight_before
                }
            })
            .collect()
    };
    let before = r.risk_before.as_ref().unwrap();
    close(
        before.volatility,
        variance(&weights(false), &covariance).sqrt(),
    );
    let z = normal_quantile(0.99);
    close(before.value_at_risk_base, z * before.volatility * 101_002.0);
    let stressed = stressed_covariance(&covariance, &stress).unwrap();
    // 2.25 x (0.5 x 0.02 + 0.5 x 0.2 x sqrt(0.05)) for SPY and SAP.
    close(
        stressed.get(0, 1),
        2.25 * (0.5 * 0.02 + 0.5 * 0.2 * 0.05f64.sqrt()),
    );
    let after = r.risk_after.as_ref().unwrap();
    close(after.volatility, variance(&weights(true), &stressed).sqrt());
    close(
        after.value_at_risk_base,
        z * after.volatility * r.value_after,
    );
    let components: f64 = after.contributions.iter().map(|c| c.component).sum();
    close(components, after.volatility);
    let groups: Vec<&str> = after.groups[0]
        .groups
        .iter()
        .map(|g| g.group.as_str())
        .collect();
    assert_eq!(groups, ["Equity", "Fixed income", "cash"]);

    // Every holding but base-currency cash must be in the model.
    let mut missing = options.clone();
    let small = Matrix::new(
        vec!["SPY".into(), "SAP".into()],
        vec![0.04, 0.02, 0.02, 0.05],
    )
    .unwrap();
    missing.risk.as_mut().unwrap().covariance = Some(small);
    let issues = validation(shock_scenario(
        &books,
        &market,
        d("2026-03-07"),
        &shocks,
        &missing,
    ));
    let messages: Vec<&str> = issues.iter().map(|(_, m)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "BOND is not in the risk model",
            "EUR cash is not in the risk model"
        ]
    );
}

#[test]
fn a_historical_period_replayed_on_todays_holdings() {
    let (books, market) = setup();
    let window = HistoricalWindow::new(d("2026-03-02"), d("2026-03-05"));
    let r = historical_scenario(
        &books,
        &market,
        d("2026-03-07"),
        &window,
        &ScenarioOptions::default(),
    )
    .unwrap();
    // SPY 500 to 502, SAP 200 to 201 with EUR 1.10 to 1.10, BOND 100 to 100.2.
    close(impact(&r, &instrument("SPY")).return_local, 0.004);
    close(impact(&r, &instrument("SAP")).return_local, 0.005);
    close(impact(&r, &instrument("SAP")).currency_return, 0.0);
    close(impact(&r, &instrument("BOND")).return_local, 0.002);
    assert_eq!(impact(&r, &instrument("SPY")).source, ShockSource::History);
    let path = r.path.as_ref().unwrap();
    assert_eq!(path.dates.len(), 4);
    close(path.values[0], 101_002.0);
    close(*path.values.last().unwrap(), r.value_after);
    // On 2026-03-04: SPY 498/500, SAP 203/200 x 1.09/1.10, BOND 100.3/100, EUR 1.09/1.10.
    let expected = 40_560.0 * 498.0 / 500.0
        + 11_322.0 * 203.0 / 200.0 * 1.09 / 1.10
        + 20_020.0 * 100.3 / 100.0
        + 18_000.0
        + 11_100.0 * 1.09 / 1.10;
    close(path.values[2], expected);
    let drawdown = path.worst_drawdown.as_ref().unwrap();
    assert_eq!(drawdown.peak, d("2026-03-03"));
    assert_eq!(drawdown.trough, d("2026-03-04"));

    // A proxy replaces an instrument's prices; its currency's rate still applies.
    let mut proxied = window.clone();
    proxied.proxies = BTreeMap::from([(id("SAP"), id("INDEX"))]);
    let r = historical_scenario(
        &books,
        &market,
        d("2026-03-07"),
        &proxied,
        &ScenarioOptions::default(),
    )
    .unwrap();
    let sap = impact(&r, &instrument("SAP"));
    close(sap.return_local, 51.0 / 50.0 - 1.0);
    assert_eq!(
        sap.source,
        ShockSource::Proxy {
            instrument: id("INDEX")
        }
    );
}

#[test]
fn a_period_without_prices_is_reported() {
    let (books, market) = setup();
    let window = HistoricalWindow::new(d("2026-01-02"), d("2026-03-05"));
    let issues = validation(historical_scenario(
        &books,
        &market,
        d("2026-03-07"),
        &window,
        &ScenarioOptions::default(),
    ));
    let messages: Vec<&str> = issues.iter().map(|(_, m)| m.as_str()).collect();
    assert!(messages.contains(&"BOND has no price for the period; name a proxy for it"));
    assert!(messages.contains(&"no value on or shortly before 2026-01-02"));
}
