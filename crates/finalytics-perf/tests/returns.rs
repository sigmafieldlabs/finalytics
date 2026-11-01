//! Returns against closed-form results.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Date, TimeSeries};
use finalytics_perf::*;

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

/// 10,000 deposited on day 0, 5,000 more on day 2.
fn sample(timing: FlowTiming) -> Performance {
    let dates = vec![
        d("2026-01-01"),
        d("2026-01-02"),
        d("2026-01-03"),
        d("2026-01-04"),
    ];
    Performance::from_parts(
        dates,
        vec![10_000.0, 10_100.0, 15_200.0, 15_000.0],
        vec![10_000.0, 0.0, 5_000.0, 0.0],
        timing,
    )
    .unwrap()
}

#[test]
fn daily_returns_remove_flows_at_the_start_of_the_day() {
    let p = sample(FlowTiming::StartOfDay);
    let r = p.returns();
    assert!(close(r.values()[0], 10_100.0 / 10_000.0 - 1.0));
    assert!(close(r.values()[1], 15_200.0 / (10_100.0 + 5_000.0) - 1.0));
    assert!(close(r.values()[2], 15_000.0 / 15_200.0 - 1.0));
    let total = p.time_weighted(d("2026-01-01"), d("2026-01-04")).unwrap();
    assert!(close(
        total,
        1.01 * (15_200.0 / 15_100.0) * (15_000.0 / 15_200.0) - 1.0
    ));
    assert!(close(p.growth_of(100.0).values()[3], 100.0 * (1.0 + total)));
}

#[test]
fn or_at_the_end_of_the_day() {
    let p = sample(FlowTiming::EndOfDay);
    assert!(close(
        p.returns().values()[1],
        (15_200.0 - 5_000.0) / 10_100.0 - 1.0
    ));
}

#[test]
fn an_empty_start_has_no_return_but_a_negative_one_is_an_error() {
    let dates = vec![d("2026-01-01"), d("2026-01-02"), d("2026-01-03")];
    let p = Performance::from_parts(
        dates.clone(),
        vec![0.0, 0.0, 1000.0],
        vec![0.0, 0.0, 1000.0],
        FlowTiming::StartOfDay,
    )
    .unwrap();
    assert_eq!(p.returns().values(), [0.0, 0.0]);
    let err = Performance::from_parts(
        dates,
        vec![0.0, -50.0, 10.0],
        vec![0.0, 0.0, 0.0],
        FlowTiming::StartOfDay,
    );
    assert!(err.is_err());
}

#[test]
fn modified_dietz_weights_flows_by_time_invested() {
    let p = sample(FlowTiming::StartOfDay);
    // Over 3 days from 2026-01-01: the 5,000 arrives at the start of day 2 and is invested 2 of 3 days.
    let md = p.modified_dietz(d("2026-01-01"), d("2026-01-04")).unwrap();
    let expected = (15_000.0 - 10_000.0 - 5_000.0) / (10_000.0 + 5_000.0 * 2.0 / 3.0);
    assert!(close(md, expected), "{md} vs {expected}");
}

#[test]
fn money_weighted_return_solves_the_cash_flows() {
    // 1,000 invested, worth 1,100 a year (365 days) later: 10%.
    let dates = vec![d("2025-01-01"), d("2026-01-01")];
    let p = Performance::from_parts(
        dates,
        vec![1000.0, 1100.0],
        vec![0.0, 0.0],
        FlowTiming::StartOfDay,
    )
    .unwrap();
    assert!(close(
        p.money_weighted(d("2025-01-01"), d("2026-01-01")).unwrap(),
        0.10
    ));

    let irr = internal_rate_of_return(&[(0.0, -1000.0), (0.5, -500.0), (1.0, 1650.0)]).unwrap();
    let npv = -1000.0 - 500.0 / (1.0 + irr).powf(0.5) + 1650.0 / (1.0 + irr);
    assert!(npv.abs() < 1e-8, "npv {npv} at {irr}");
    assert!(internal_rate_of_return(&[(0.0, 100.0), (1.0, 100.0)]).is_none());
}

#[test]
fn periods_use_the_valuation_before_their_start() {
    // A business-day series from 2023-12-29 to 2026-11-02 growing 0.01% a day.
    let mut dates = Vec::new();
    let mut day = d("2023-12-29");
    while day <= d("2026-11-02") {
        if day.weekday().index() < 5 {
            dates.push(day);
        }
        day = day.add_days(1).unwrap();
    }
    let values: Vec<f64> = (0..dates.len())
        .map(|i| 100.0 * 1.0001f64.powi(i as i32))
        .collect();
    let p = Performance::from_parts(
        dates.clone(),
        values.clone(),
        vec![0.0; dates.len()],
        FlowTiming::StartOfDay,
    )
    .unwrap();
    let value_on = |date: &str| values[dates.partition_point(|&x| x <= d(date)) - 1];

    let ytd = p.period(Period::YearToDate, None).unwrap();
    assert_eq!((ytd.start, ytd.end), (d("2025-12-31"), d("2026-11-02")));
    assert!(close(
        ytd.cumulative,
        value_on("2026-11-02") / value_on("2025-12-31") - 1.0
    ));
    assert_eq!(ytd.annualised, None);

    let mtd = p.period(Period::MonthToDate, None).unwrap();
    assert_eq!(mtd.start, d("2026-10-30")); // the last valuation on or before 31 October, a Saturday

    let two_years = p.period(Period::Years { years: 2 }, None).unwrap();
    assert_eq!(two_years.start, d("2024-11-01"));
    let annual = two_years.annualised.unwrap();
    assert!(close(
        annual,
        (1.0 + two_years.cumulative).powf(365.25 / 731.0) - 1.0
    ));

    assert!(p.period(Period::Years { years: 3 }, None).is_none());
    let itd = p
        .period(Period::SinceInception, Some(d("2024-06-30")))
        .unwrap();
    assert_eq!((itd.start, itd.end), (d("2023-12-29"), d("2024-06-28")));
}

#[test]
fn excess_return_against_a_benchmark() {
    let p = sample(FlowTiming::StartOfDay);
    let benchmark =
        TimeSeries::new(vec![d("2026-01-01"), d("2026-01-03")], vec![200.0, 201.0]).unwrap();
    let excess = p
        .excess_return(Period::SinceInception, None, &benchmark, 3)
        .unwrap();
    assert!(close(excess.benchmark, 201.0 / 200.0 - 1.0));
    assert!(close(
        excess.arithmetic,
        excess.portfolio - excess.benchmark
    ));
    assert!(close(
        excess.geometric,
        (1.0 + excess.portfolio) / (1.0 + excess.benchmark) - 1.0
    ));
    assert!(
        p.excess_return(Period::SinceInception, None, &benchmark, 0)
            .is_none()
    );
}

#[test]
fn performance_from_the_books() {
    use finalytics_core::{Currency, Decimal, Money};
    use finalytics_ledger::*;
    let id = |s: &str| InstrumentId::new(s).unwrap();
    let tx = |n: &str, date: &str, kind| Transaction {
        id: TransactionId::new(n).unwrap(),
        account: AccountId::new("main").unwrap(),
        trade_date: d(date),
        settle_date: None,
        kind,
        note: None,
    };
    let portfolio = Portfolio {
        base_currency: Currency::USD,
        accounts: vec![Account::new(AccountId::new("main").unwrap())],
        instruments: vec![Instrument::new(
            id("SPY"),
            InstrumentKind::Fund,
            Currency::USD,
        )],
        transactions: vec![
            tx(
                "t1",
                "2026-01-02",
                TransactionKind::Deposit {
                    amount: Money::new(Decimal::from(1000), Currency::USD),
                },
            ),
            tx(
                "t2",
                "2026-01-02",
                TransactionKind::Buy {
                    instrument: id("SPY"),
                    quantity: Decimal::from(2),
                    price: Decimal::from(500),
                    fees: Decimal::ZERO,
                },
            ),
        ],
    };
    let prices = TimeSeries::new(
        vec![d("2026-01-02"), d("2026-01-05")],
        vec![Decimal::from(500), Decimal::from(510)],
    )
    .unwrap();
    let market = MarketData::new().with_prices(id("SPY"), prices);
    let books = Books::build(&portfolio, &market).unwrap();
    let series = books
        .value_series(&[d("2026-01-02"), d("2026-01-05")], &market)
        .unwrap();
    let p = Performance::from_values(&series, FlowTiming::StartOfDay).unwrap();
    assert!(close(
        p.time_weighted(d("2026-01-02"), d("2026-01-05")).unwrap(),
        0.02
    ));
}
