//! Contributions add up to the portfolio's return.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Currency, Date, Decimal, Money, TimeSeries};
use finalytics_ledger::*;
use finalytics_perf::*;

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn id(s: &str) -> InstrumentId {
    InstrumentId::new(s).unwrap()
}

fn tx(n: usize, date: &str, kind: TransactionKind) -> Transaction {
    Transaction::new(
        TransactionId::new(format!("t{n}")).unwrap(),
        AccountId::new("main").unwrap(),
        d(date),
        kind,
    )
}

fn daily(start: &str, values: &[&str]) -> TimeSeries<Decimal> {
    let first = d(start);
    TimeSeries::new(
        (0..values.len())
            .map(|i| first.add_days(i as i32).unwrap())
            .collect(),
        values.iter().map(|v| dec(v)).collect(),
    )
    .unwrap()
}

fn setup() -> (Books, MarketData, Vec<Date>) {
    let mut portfolio = PortfolioData::new(Currency::USD);
    portfolio.accounts = vec![Account::new(AccountId::new("main").unwrap())];
    portfolio.instruments = vec![
        Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD)
            .classified("asset_class", "Equity"),
        Instrument::new(id("SAP"), InstrumentKind::Equity, Currency::EUR)
            .classified("asset_class", "Equity"),
        Instrument::new(id("BOND"), InstrumentKind::Bond, Currency::USD)
            .classified("asset_class", "Fixed income"),
        Instrument::new(id("NEWCO"), InstrumentKind::Equity, Currency::USD)
            .classified("asset_class", "Equity"),
    ];
    portfolio.transactions = vec![
        tx(
            1,
            "2026-03-02",
            TransactionKind::Deposit {
                amount: Money::new(dec("100000"), Currency::USD),
            },
        ),
        tx(
            2,
            "2026-03-02",
            TransactionKind::Buy {
                instrument: id("SPY"),
                quantity: dec("100"),
                price: dec("500"),
                fees: dec("10"),
            },
        ),
        tx(
            3,
            "2026-03-02",
            TransactionKind::FxConversion {
                sell: Money::new(dec("22000"), Currency::USD),
                buy: Money::new(dec("19900"), Currency::EUR),
            },
        ),
        tx(
            4,
            "2026-03-03",
            TransactionKind::Buy {
                instrument: id("SAP"),
                quantity: dec("100"),
                price: dec("190"),
                fees: dec("0"),
            },
        ),
        tx(
            5,
            "2026-03-03",
            TransactionKind::Buy {
                instrument: id("BOND"),
                quantity: dec("200"),
                price: dec("100"),
                fees: dec("0"),
            },
        ),
        tx(
            6,
            "2026-03-04",
            TransactionKind::Dividend {
                instrument: id("SPY"),
                amount: Money::new(dec("150"), Currency::USD),
                withholding_tax: dec("22.5"),
            },
        ),
        tx(
            7,
            "2026-03-05",
            TransactionKind::Sell {
                instrument: id("SPY"),
                quantity: dec("40"),
                price: dec("505"),
                fees: dec("5"),
                lot_method: None,
                lots: vec![],
            },
        ),
        tx(
            8,
            "2026-03-05",
            TransactionKind::SpinOff {
                instrument: id("SAP"),
                new_instrument: id("NEWCO"),
                ratio: dec("0.5"),
                cost_allocation: dec("0.1"),
            },
        ),
        tx(
            9,
            "2026-03-06",
            TransactionKind::Deposit {
                amount: Money::new(dec("5000"), Currency::USD),
            },
        ),
        tx(
            10,
            "2026-03-06",
            TransactionKind::Fee {
                amount: Money::new(dec("12"), Currency::USD),
            },
        ),
    ];
    let market = MarketData::new()
        .with_prices(
            id("SPY"),
            daily("2026-03-02", &["500", "502", "499", "505", "510"]),
        )
        .with_prices(
            id("SAP"),
            daily("2026-03-02", &["189", "190", "192", "180", "182"]),
        )
        .with_prices(
            id("BOND"),
            daily("2026-03-02", &["100", "100", "100.2", "100.1", "100.3"]),
        )
        .with_prices(id("NEWCO"), daily("2026-03-05", &["22", "23"]))
        .with_fx(
            Currency::EUR,
            Currency::USD,
            daily("2026-03-02", &["1.10", "1.11", "1.12", "1.10", "1.09"]),
        );
    let books = Books::build(&portfolio, &market).unwrap();
    let dates: Vec<Date> = (0..5)
        .map(|i| d("2026-03-02").add_days(i).unwrap())
        .collect();
    (books, market, dates)
}

#[test]
fn contributions_add_up_to_the_time_weighted_return() {
    let (books, market, dates) = setup();
    for timing in [FlowTiming::StartOfDay, FlowTiming::EndOfDay] {
        let values = books.value_series(&dates, &market).unwrap();
        let performance = Performance::from_values(&values, timing).unwrap();
        let holdings = books.holding_series(&dates, &market).unwrap();
        let contributions = performance
            .contributions(&holdings, dates[0], dates[4])
            .unwrap();
        let total: f64 = contributions.iter().map(|c| c.contribution).sum();
        let twr = performance.time_weighted(dates[0], dates[4]).unwrap();
        assert!(
            (total - twr).abs() < 1e-12,
            "{timing:?}: contributions {total} vs return {twr}"
        );

        // Every line's values add up to the portfolio's.
        for (t, value) in values.values.iter().enumerate() {
            let sum: Decimal = holdings.lines.iter().map(|l| l.values_base[t]).sum();
            assert_eq!(sum, *value);
        }

        let groups = group_contributions(&contributions, |key| match key {
            HoldingKey::Cash(_) => "Cash".to_owned(),
            HoldingKey::Instrument(i) => {
                books.instrument(i).unwrap().classifications["asset_class"].clone()
            }
        });
        let names: Vec<_> = groups.iter().map(|g| g.group.as_str()).collect();
        assert_eq!(names, ["Cash", "Equity", "Fixed income"]);
        let group_total: f64 = groups.iter().map(|g| g.contribution).sum();
        assert!((group_total - twr).abs() < 1e-12);
    }
}

#[test]
fn holdings_report_their_own_and_currency_returns() {
    let (books, market, dates) = setup();
    let performance = Performance::from_values(
        &books.value_series(&dates, &market).unwrap(),
        FlowTiming::StartOfDay,
    )
    .unwrap();
    let holdings = books.holding_series(&dates, &market).unwrap();
    let contributions = performance
        .contributions(&holdings, dates[1], dates[2])
        .unwrap();
    let sap = contributions
        .iter()
        .find(|c| c.holding == HoldingKey::Instrument(id("SAP")))
        .unwrap();
    // One day, 3 to 4 March: SAP from 190 to 192 euros, the euro from 1.11 to 1.12 dollars.
    let local = 192.0 / 190.0 - 1.0;
    let base = (192.0 * 1.12) / (190.0 * 1.11) - 1.0;
    assert!((sap.return_local.unwrap() - local).abs() < 1e-12);
    assert!((sap.return_base.unwrap() - base).abs() < 1e-12);
    assert!((sap.currency_effect.unwrap() - (base - local)).abs() < 1e-12);

    // The dividend counts towards SPY: 502 to 499 a share plus 127.50 of net dividend on 100 shares.
    let spy = contributions
        .iter()
        .find(|c| c.holding == HoldingKey::Instrument(id("SPY")))
        .unwrap();
    let expected = (100.0 * 499.0 + 127.5) / (100.0 * 502.0) - 1.0;
    assert!((spy.return_base.unwrap() - expected).abs() < 1e-12);
}

#[test]
fn holdings_must_share_the_performance_dates() {
    let (books, market, dates) = setup();
    let performance = Performance::from_values(
        &books.value_series(&dates, &market).unwrap(),
        FlowTiming::StartOfDay,
    )
    .unwrap();
    let holdings = books.holding_series(&dates[1..], &market).unwrap();
    assert!(
        performance
            .contributions(&holdings, dates[1], dates[4])
            .is_err()
    );
}
