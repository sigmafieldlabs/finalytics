//! Risk decomposition of a portfolio from its books.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Currency, Date, Decimal, Money, TimeSeries};
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

fn dates() -> Vec<Date> {
    (0..6)
        .map(|i| d("2026-03-02").add_days(i).unwrap())
        .collect()
}

fn setup() -> (Books, MarketData) {
    let usd = |a: &str| Money::new(dec(a), Currency::USD);
    let mut portfolio = Portfolio::new(Currency::USD);
    portfolio.accounts = vec![
        Account::new(AccountId::new("main").unwrap()),
        Account::new(AccountId::new("isa").unwrap()),
    ];
    portfolio.instruments = vec![
        Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD)
            .classified("asset_class", "Equity"),
        Instrument::new(id("SAP"), InstrumentKind::Equity, Currency::EUR)
            .classified("asset_class", "Equity"),
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
        tx(
            3,
            TransactionKind::Buy {
                instrument: id("SPY"),
                quantity: dec("60"),
                price: dec("500"),
                fees: dec("0"),
            },
        ),
        tx(
            4,
            TransactionKind::Buy {
                instrument: id("SPY"),
                quantity: dec("20"),
                price: dec("500"),
                fees: dec("0"),
            },
        ),
        tx(
            5,
            TransactionKind::FxConversion {
                sell: usd("22000"),
                buy: Money::new(dec("20000"), Currency::EUR),
            },
        ),
        tx(
            7,
            TransactionKind::Buy {
                instrument: id("SAP"),
                quantity: dec("50"),
                price: dec("200"),
                fees: dec("0"),
            },
        ),
        tx(
            6,
            TransactionKind::Buy {
                instrument: id("BOND"),
                quantity: dec("200"),
                price: dec("100"),
                fees: dec("0"),
            },
        ),
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
        .with_fx(
            Currency::EUR,
            Currency::USD,
            daily(&["1.10", "1.11", "1.09", "1.10", "1.12", "1.11"]),
        );
    (Books::build(&portfolio, &market).unwrap(), market)
}

#[test]
fn weights_add_up_across_accounts() {
    let (books, market) = setup();
    let valuation = books.value_on(d("2026-03-02"), &market).unwrap();
    let weights = holding_weights(&valuation).unwrap();
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    assert!((total - 1.0).abs() < 1e-15);
    let spy = weights
        .iter()
        .find(|(k, _)| *k == HoldingKey::Instrument(id("SPY")))
        .unwrap();
    // 80 shares at 500, bought in two accounts, out of 100,000.
    assert!((spy.1 - 0.4).abs() < 1e-15);
    let euro = weights
        .iter()
        .find(|(k, _)| *k == HoldingKey::Cash(Currency::EUR))
        .unwrap();
    assert!((euro.1 - 10_000.0 * 1.10 / 100_000.0).abs() < 1e-15);
}

#[test]
fn returns_are_in_the_base_currency() {
    let (books, market) = setup();
    let holdings = [
        HoldingKey::Instrument(id("SAP")),
        HoldingKey::Cash(Currency::EUR),
        HoldingKey::Cash(Currency::USD),
    ];
    let returns = holding_returns(&books, &holdings, &dates(), &market).unwrap();
    assert_eq!(returns.dates.len(), 5);
    assert_eq!(returns.table.labels(), ["SAP", "EUR cash", "USD cash"]);
    let columns = returns.table.columns();
    assert!((columns[0][0] - (198.0 * 1.11 / (200.0 * 1.10) - 1.0)).abs() < 1e-15);
    assert!((columns[1][0] - (1.11 / 1.10 - 1.0)).abs() < 1e-15);
    assert!(columns[2].iter().all(|r| *r == 0.0));

    let unknown = [HoldingKey::Instrument(id("NONE"))];
    assert!(holding_returns(&books, &unknown, &dates(), &market).is_err());
    let late: Vec<Date> = (0..3)
        .map(|i| d("2026-03-20").add_days(i).unwrap())
        .collect();
    let Err(finalytics_core::Error::Validation(report)) =
        holding_returns(&books, &holdings[..1], &late, &market)
    else {
        panic!("expected missing prices");
    };
    assert!(report.issues().iter().any(|i| i.path == "prices.SAP"));
    assert!(report.issues().iter().any(|i| i.path == "fx.EURUSD"));
}

#[test]
fn risk_by_holding_and_asset_class() {
    let (books, market) = setup();
    let valuation = books.value_on(d("2026-03-07"), &market).unwrap();
    let weights = holding_weights(&valuation).unwrap();
    let (holdings, w): (Vec<HoldingKey>, Vec<f64>) = weights.into_iter().unzip();
    let returns = holding_returns(&books, &holdings, &dates(), &market).unwrap();
    let estimate = covariance(&returns.table, CovarianceMethod::Sample).unwrap();
    let annual = estimate.matrix.scaled(252.0);
    let risk = risk_decomposition(&w, &annual).unwrap();

    let total: f64 = risk.contributions.iter().map(|c| c.component).sum();
    assert!((total - risk.volatility).abs() < 1e-14);
    assert!((annual.quadratic_form(&w).unwrap().sqrt() - risk.volatility).abs() < 1e-15);

    let groups = group_risk(&risk, |k, _| match &holdings[k] {
        HoldingKey::Cash(_) => "Cash".to_owned(),
        HoldingKey::Instrument(i) => {
            books.instrument(i).unwrap().classifications["asset_class"].clone()
        }
    });
    let names: Vec<_> = groups.iter().map(|g| g.group.as_str()).collect();
    assert_eq!(names, ["Cash", "Equity", "Fixed income"]);
    let shares: f64 = groups.iter().map(|g| g.share.unwrap()).sum();
    assert!((shares - 1.0).abs() < 1e-14);
}
