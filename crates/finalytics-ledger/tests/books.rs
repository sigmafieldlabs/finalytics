//! End-to-end tests: build books from transactions and value them.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Currency, Date, Decimal, Error, Money, TimeSeries};
use finalytics_ledger::*;

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn id<T>(make: fn(String) -> Option<T>, s: &str) -> T {
    make(s.to_owned()).unwrap()
}

fn iid(s: &str) -> InstrumentId {
    id(InstrumentId::new, s)
}

fn usd(s: &str) -> Money {
    Money::new(dec(s), Currency::USD)
}

fn tx(n: usize, date: &str, kind: TransactionKind) -> Transaction {
    Transaction::new(
        id(TransactionId::new, &format!("t{n}")),
        id(AccountId::new, "main"),
        d(date),
        kind,
    )
}

fn buy(instrument: &str, quantity: &str, price: &str, fees: &str) -> TransactionKind {
    TransactionKind::Buy {
        instrument: iid(instrument),
        quantity: dec(quantity),
        price: dec(price),
        fees: dec(fees),
    }
}

fn sell(instrument: &str, quantity: &str, price: &str, fees: &str) -> TransactionKind {
    TransactionKind::Sell {
        instrument: iid(instrument),
        quantity: dec(quantity),
        price: dec(price),
        fees: dec(fees),
        lot_method: None,
        lots: vec![],
    }
}

fn series(points: &[(&str, &str)]) -> TimeSeries<Decimal> {
    TimeSeries::new(
        points.iter().map(|p| d(p.0)).collect(),
        points.iter().map(|p| dec(p.1)).collect(),
    )
    .unwrap()
}

fn portfolio(instruments: Vec<Instrument>, transactions: Vec<Transaction>) -> PortfolioData {
    {
        let mut portfolio = PortfolioData::new(Currency::USD);
        portfolio.accounts = vec![Account::new(id(AccountId::new, "main"))];
        portfolio.instruments = instruments;
        portfolio.transactions = transactions;
        portfolio
    }
}

fn equity(name: &str, currency: Currency) -> Instrument {
    Instrument::new(iid(name), InstrumentKind::Equity, currency)
}

/// Deposits, two buys, a dividend, a split, a FIFO sale and a withdrawal, in US dollars.
fn single_currency() -> PortfolioData {
    portfolio(
        vec![equity("AAPL", Currency::USD)],
        vec![
            tx(
                1,
                "2026-01-02",
                TransactionKind::Deposit {
                    amount: usd("10000"),
                },
            ),
            tx(2, "2026-01-05", buy("AAPL", "10", "150", "5")),
            tx(3, "2026-03-02", buy("AAPL", "10", "200", "5")),
            tx(
                4,
                "2026-04-01",
                TransactionKind::Dividend {
                    instrument: iid("AAPL"),
                    amount: usd("20"),
                    withholding_tax: dec("3"),
                },
            ),
            tx(
                5,
                "2026-05-01",
                TransactionKind::Split {
                    instrument: iid("AAPL"),
                    from: dec("1"),
                    to: dec("2"),
                },
            ),
            tx(6, "2026-06-01", sell("AAPL", "25", "110", "5")),
            tx(
                7,
                "2026-06-15",
                TransactionKind::Withdrawal {
                    amount: usd("1000"),
                },
            ),
        ],
    )
}

#[test]
fn lots_cash_and_gains_follow_the_transactions() {
    let books = Books::build(&single_currency(), &MarketData::new()).unwrap();

    let gains: Vec<_> = books
        .realised_gains()
        .iter()
        .map(|g| {
            (
                g.lot.as_str(),
                g.quantity,
                g.cost_local,
                g.proceeds_local,
                g.gain_local,
            )
        })
        .collect();
    assert_eq!(
        gains,
        [
            ("t2", dec("20"), dec("1505"), dec("2196"), dec("691")),
            ("t3", dec("5"), dec("501.25"), dec("549"), dec("47.75")),
        ]
    );
    assert_eq!(books.realised_gains()[0].holding_days(), 147);

    let holdings = books.holdings();
    let positions = holdings.positions();
    assert_eq!(positions.len(), 1);
    assert_eq!(
        (positions[0].quantity, positions[0].cost_local),
        (dec("15"), dec("1503.75"))
    );
    let cash: Vec<_> = holdings.cash().map(|(_, c, a)| (c, a)).collect();
    assert_eq!(cash, [(Currency::USD, dec("8252"))]);

    let march = books.holdings_on(d("2026-03-31"));
    let lots: Vec<_> = march
        .lots()
        .map(|l| (l.id.as_str(), l.quantity, l.cost_local))
        .collect();
    assert_eq!(
        lots,
        [
            ("t2", dec("10"), dec("1505")),
            ("t3", dec("10"), dec("2005"))
        ]
    );
}

#[test]
fn valuation_marks_to_market() {
    let books = Books::build(&single_currency(), &MarketData::new()).unwrap();
    let market = MarketData::new().with_prices(iid("AAPL"), series(&[("2026-06-30", "120")]));
    let valuation = books.value_on(d("2026-06-30"), &market).unwrap();
    assert_eq!(valuation.total_base, dec("10052"));
    assert_eq!(valuation.positions[0].unrealised_local, dec("296.25"));
    assert!(valuation.warnings.is_empty());

    let carried = books.value_on(d("2026-07-02"), &market).unwrap();
    assert_eq!(carried.positions[0].price_date, d("2026-06-30"));
    assert_eq!(carried.warnings.len(), 1);

    let Err(Error::Validation(report)) = books.value_on(d("2026-07-10"), &market) else {
        panic!("expected missing prices")
    };
    assert_eq!(report.issues()[0].path, "prices.AAPL");
}

#[test]
fn cash_below_zero_is_a_warning() {
    let mut pf = single_currency();
    // A purchase of 75,000 with 10,000 deposited.
    pf.transactions
        .push(tx(9, "2026-06-01", buy("AAPL", "500", "150", "0")));
    let books = Books::build(&pf, &MarketData::new()).unwrap();
    let market = MarketData::new().with_prices(iid("AAPL"), series(&[("2026-06-30", "120")]));
    let valuation = books.value_on(d("2026-06-30"), &market).unwrap();
    let issues = valuation.warnings.issues();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, "cash.main.USD");
    assert!(
        issues[0].message.contains("below zero"),
        "{}",
        issues[0].message
    );
    assert!(!valuation.warnings.has_errors());
}

#[test]
fn foreign_holdings_keep_cost_and_gains_in_both_currencies() {
    let mut pf = portfolio(vec![equity("SAP", Currency::EUR)], vec![]);
    pf.transactions = vec![
        tx(
            1,
            "2026-01-02",
            TransactionKind::Deposit {
                amount: Money::new(dec("10000"), Currency::EUR),
            },
        ),
        tx(2, "2026-01-05", buy("SAP", "10", "100", "0")),
        tx(3, "2026-02-02", sell("SAP", "10", "110", "0")),
    ];
    let market = MarketData::new()
        .with_fx(
            Currency::EUR,
            Currency::USD,
            series(&[
                ("2026-01-02", "1.10"),
                ("2026-01-05", "1.20"),
                ("2026-02-02", "1.10"),
            ]),
        )
        .with_prices(
            iid("SAP"),
            series(&[("2026-01-05", "100"), ("2026-02-02", "110")]),
        );
    let books = Books::build(&pf, &market).unwrap();
    let gain = &books.realised_gains()[0];
    assert_eq!(
        (gain.cost_base, gain.proceeds_base),
        (dec("1200.00"), dec("1210.00"))
    );
    assert_eq!(
        (gain.gain_local, gain.gain_base),
        (dec("100"), dec("10.00"))
    );

    let series = books
        .value_series(
            &[d("2026-01-02"), d("2026-01-05"), d("2026-02-02")],
            &market,
        )
        .unwrap();
    assert_eq!(
        series.values,
        [dec("11000.00"), dec("12000.00"), dec("11110.00")]
    );
    assert_eq!(series.flows, [dec("11000.00"), dec("0"), dec("0")]);
}

#[test]
fn corporate_actions_carry_lots_and_cost_basis() {
    let pf = portfolio(
        ["PARENT", "CHILD", "NEWPARENT", "ACQ"]
            .iter()
            .map(|n| equity(n, Currency::USD))
            .collect(),
        vec![
            tx(
                1,
                "2026-01-02",
                TransactionKind::Deposit {
                    amount: usd("5000"),
                },
            ),
            tx(2, "2026-01-05", buy("PARENT", "100", "40", "0")),
            tx(
                3,
                "2026-02-02",
                TransactionKind::SpinOff {
                    instrument: iid("PARENT"),
                    new_instrument: iid("CHILD"),
                    ratio: dec("0.5"),
                    cost_allocation: dec("0.25"),
                },
            ),
            tx(
                4,
                "2026-03-02",
                TransactionKind::ReturnOfCapital {
                    instrument: iid("PARENT"),
                    amount: usd("3500"),
                },
            ),
            tx(
                5,
                "2026-04-01",
                TransactionKind::SymbolChange {
                    instrument: iid("PARENT"),
                    new_instrument: iid("NEWPARENT"),
                },
            ),
            tx(
                6,
                "2026-05-01",
                TransactionKind::Merger {
                    instrument: iid("CHILD"),
                    new_instrument: iid("ACQ"),
                    ratio: dec("2"),
                    cash_per_unit: dec("5"),
                },
            ),
            tx(
                7,
                "2026-06-01",
                TransactionKind::Merger {
                    instrument: iid("NEWPARENT"),
                    new_instrument: iid("NEWPARENT"),
                    ratio: dec("0"),
                    cash_per_unit: dec("50"),
                },
            ),
        ],
    );
    let books = Books::build(&pf, &MarketData::new()).unwrap();

    let after_spin = books.holdings_on(d("2026-02-02"));
    let lots: Vec<_> = after_spin
        .lots()
        .map(|l| {
            (
                l.id.as_str(),
                l.instrument.as_str(),
                l.quantity,
                l.cost_local,
                l.acquired,
            )
        })
        .collect();
    assert_eq!(
        lots,
        [
            (
                "t2/t3",
                "CHILD",
                dec("50.0"),
                dec("1000.00"),
                d("2026-01-05")
            ),
            ("t2", "PARENT", dec("100"), dec("3000.00"), d("2026-01-05")),
        ]
    );

    let gains: Vec<_> = books
        .realised_gains()
        .iter()
        .map(|g| (g.transaction.as_str(), g.quantity, g.gain_local))
        .collect();
    assert_eq!(
        gains,
        [
            ("t4", dec("0"), dec("500.00")),
            ("t7", dec("100"), dec("5000"))
        ]
    );

    let positions = books.holdings().positions();
    let held: Vec<_> = positions
        .iter()
        .map(|p| (p.instrument.as_str(), p.quantity, p.cost_local))
        .collect();
    assert_eq!(held, [("ACQ", dec("100.0"), dec("750.00"))]);
    let cash: Vec<_> = books.holdings().cash().map(|(_, _, a)| a).collect();
    assert_eq!(cash, [dec("9750.0")]);
}

#[test]
fn applying_stops_at_the_first_impossible_transaction() {
    let mut pf = single_currency();
    pf.transactions
        .push(tx(8, "2026-07-01", sell("AAPL", "16", "100", "0")));
    let Err(Error::Validation(report)) = Books::build(&pf, &MarketData::new()) else {
        panic!("expected an error")
    };
    assert_eq!(report.issues()[0].path, "transactions[7]");
    assert!(
        report.issues()[0]
            .message
            .contains("needs 16 units of AAPL, but account main holds 15"),
        "{}",
        report
    );
}

#[test]
fn structural_problems_are_all_reported() {
    let mut pf = single_currency();
    pf.transactions
        .push(tx(9, "2026-07-01", buy("MSFT", "-1", "100", "0")));
    pf.transactions.push(tx(
        1,
        "2026-07-01",
        TransactionKind::Deposit { amount: usd("0") },
    ));
    let report = pf.validate();
    let paths: Vec<_> = report.issues().iter().map(|i| i.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "transactions[7].instrument",
            "transactions[7].quantity",
            "transactions[8].id",
            "transactions[8].amount"
        ]
    );
}

#[test]
fn portfolios_round_trip_through_json() {
    let pf = single_currency();
    let json = serde_json::to_string(&pf).unwrap();
    assert!(
        json.contains(
            r#""type":"buy","instrument":"AAPL","quantity":"10","price":"150","fees":"5""#
        ),
        "{json}"
    );
    let back: PortfolioData = serde_json::from_str(&json).unwrap();
    assert_eq!(back, pf);
    let numbers = r#"{"id":"x","account":"main","trade_date":"2026-01-05","type":"buy","instrument":"AAPL","quantity":10,"price":150.5}"#;
    let parsed: Transaction = serde_json::from_str(numbers).unwrap();
    assert_eq!(parsed.kind, buy("AAPL", "10", "150.5", "0"));
}

#[test]
fn a_sale_preview_leaves_the_lots_alone() {
    let books = Books::build(&single_currency(), &MarketData::new()).unwrap();
    // Before the sale: lots t2 (10 at 150.50) and t3 (10 at 200.50).
    let march = books.holdings_on(d("2026-03-31"));
    let main = id(AccountId::new, "main");
    let preview = |method| {
        march
            .lots_for_sale(&main, &iid("AAPL"), dec("12"), method)
            .unwrap()
            .into_iter()
            .map(|s| (s.lot.as_str().to_owned(), s.quantity, s.cost_local))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        preview(LotMethod::Fifo),
        [
            ("t2".into(), dec("10"), dec("1505")),
            ("t3".into(), dec("2"), dec("401"))
        ]
    );
    assert_eq!(
        preview(LotMethod::HighestCost),
        [
            ("t3".into(), dec("10"), dec("2005")),
            ("t2".into(), dec("2"), dec("301"))
        ]
    );
    assert_eq!(march.lots_of(&main, &iid("AAPL")).len(), 2);

    assert!(
        march
            .lots_for_sale(&main, &iid("AAPL"), dec("21"), LotMethod::Fifo)
            .is_err()
    );
    assert!(
        march
            .lots_for_sale(&main, &iid("AAPL"), dec("1"), LotMethod::SpecificId)
            .is_err()
    );
    assert!(
        march
            .lots_for_sale(&main, &iid("AAPL"), dec("0"), LotMethod::Fifo)
            .is_err()
    );
}

#[test]
fn holdings_are_written_as_text() {
    let spy = HoldingKey::Instrument(iid("SPY"));
    let usd = HoldingKey::Cash(Currency::USD);
    assert_eq!(spy.to_string(), "SPY");
    assert_eq!(usd.to_string(), "USD cash");
    for key in [&spy, &usd] {
        assert_eq!(&key.to_string().parse::<HoldingKey>().unwrap(), key);
        let text = serde_json::to_string(key).unwrap();
        assert_eq!(&serde_json::from_str::<HoldingKey>(&text).unwrap(), key);
    }
    // Keyed by holding, values are a plain map.
    let weights = std::collections::BTreeMap::from([(spy.clone(), 0.6), (usd.clone(), 0.4)]);
    assert_eq!(
        serde_json::to_string(&weights).unwrap(),
        r#"{"SPY":0.6,"USD cash":0.4}"#
    );
    // Text that is not a currency followed by " cash" names an instrument.
    assert_eq!(
        "usd cash".parse::<HoldingKey>().unwrap(),
        HoldingKey::Instrument(iid("usd cash"))
    );
    assert!("".parse::<HoldingKey>().is_err());
}

#[test]
fn an_instrument_cannot_be_named_like_cash() {
    let mut portfolio = single_currency();
    portfolio
        .instruments
        .push(equity("EUR cash", Currency::EUR));
    let issues = portfolio.validate();
    assert!(
        issues
            .issues()
            .iter()
            .any(|i| i.path.starts_with("instruments[") && i.message.contains("cash holding")),
        "{issues:?}"
    );
}
