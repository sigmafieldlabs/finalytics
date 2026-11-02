//! A household of three accounts, worked by hand.

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

fn account(s: &str) -> AccountId {
    AccountId::new(s).unwrap()
}

fn usd(a: &str) -> Money {
    Money::new(dec(a), Currency::USD)
}

fn tx(n: usize, account_id: &str, date: &str, kind: TransactionKind) -> Transaction {
    Transaction::new(
        TransactionId::new(format!("t{n}")).unwrap(),
        account(account_id),
        d(date),
        kind,
    )
}

fn dates() -> Vec<Date> {
    ["2026-03-02", "2026-03-03", "2026-03-04", "2026-03-05"]
        .iter()
        .map(|s| d(s))
        .collect()
}

/// Alice's brokerage account and ISA each hold 50 SPY; Bob's account receives 1,000 in cash on
/// the second day; on the third, 2,000 moves from the brokerage account to the ISA.
///
/// | date | SPY | main | isa | kid | household |
/// | --- | --- | --- | --- | --- | --- |
/// | 03-02 | 100 | 10,000 | 5,000 | 0 | 15,000 |
/// | 03-03 | 110 | 10,500 | 5,500 | 1,000 | 17,000 |
/// | 03-04 | 99 | 7,950 | 6,950 | 1,000 | 15,900 |
/// | 03-05 | 105 | 8,250 | 7,250 | 1,000 | 16,500 |
fn setup() -> (Books, MarketData) {
    let spy = InstrumentId::new("SPY").unwrap();
    let buy = |n, a, date| {
        tx(
            n,
            a,
            date,
            TransactionKind::Buy {
                instrument: spy.clone(),
                quantity: dec("50"),
                price: dec("100"),
                fees: dec("0"),
            },
        )
    };
    let deposit = |n, a, date, amount| {
        tx(
            n,
            a,
            date,
            TransactionKind::Deposit {
                amount: usd(amount),
            },
        )
    };
    let mut portfolio = PortfolioData::new(Currency::USD);
    portfolio.accounts = vec![
        Account::new(account("main"))
            .classified("owner", "Alice")
            .classified("wrapper", "taxable"),
        Account::new(account("isa"))
            .classified("owner", "Alice")
            .classified("wrapper", "ISA"),
        Account::new(account("kid")).classified("owner", "Bob"),
    ];
    portfolio.instruments = vec![Instrument::new(
        spy.clone(),
        InstrumentKind::Fund,
        Currency::USD,
    )];
    portfolio.transactions = vec![
        deposit(1, "main", "2026-03-02", "10000"),
        buy(2, "main", "2026-03-02"),
        deposit(3, "isa", "2026-03-02", "5000"),
        buy(4, "isa", "2026-03-02"),
        deposit(5, "kid", "2026-03-03", "1000"),
        tx(
            6,
            "main",
            "2026-03-04",
            TransactionKind::Withdrawal {
                amount: usd("2000"),
            },
        ),
        deposit(7, "isa", "2026-03-04", "2000"),
    ];
    let prices = TimeSeries::new(
        dates(),
        ["100", "110", "99", "105"].iter().map(|p| dec(p)).collect(),
    )
    .unwrap();
    let market = MarketData::new().with_prices(spy, prices);
    (Books::build(&portfolio, &market).unwrap(), market)
}

#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "{actual} against {expected}"
    );
}

#[test]
fn account_series_add_up_to_the_value_series() {
    let (books, market) = setup();
    let series = books.account_series(&dates(), &market).unwrap();
    let values: Vec<Vec<Decimal>> = series.lines.iter().map(|l| l.values.clone()).collect();
    assert_eq!(
        values,
        [
            ["10000", "10500", "7950", "8250"].map(dec),
            ["5000", "5500", "6950", "7250"].map(dec),
            ["0", "1000", "1000", "1000"].map(dec),
        ]
    );
    assert_eq!(series.lines[0].flows[2], dec("-2000"));
    assert_eq!(series.lines[1].flows[2], dec("2000"));
    let all = series.combined(|_| true);
    let whole = books.value_series(&dates(), &market).unwrap();
    assert_eq!(all.values, whole.values);
    assert_eq!(all.flows, whole.flows);
    // Between Alice's two accounts, the move from one to the other nets to zero.
    let alice = series.combined(|a| a.as_str() != "kid");
    assert_eq!(alice.flows[2], Decimal::ZERO);
}

#[test]
fn performance_by_account_and_by_group() {
    let (books, market) = setup();
    let mut options = HouseholdOptions::default();
    options.group_by = vec!["owner".into(), "wrapper".into()];
    options.timing = FlowTiming::StartOfDay;
    let h = household(&books, &market, &dates(), &options).unwrap();

    // The household: 17,000 / 16,000, 15,900 / 17,000 and 16,500 / 15,900.
    let (r1, r2, r3) = (
        17_000.0 / 16_000.0,
        15_900.0 / 17_000.0,
        16_500.0 / 15_900.0,
    );
    let twr = r1 * r2 * r3 - 1.0;
    close(h.household.time_weighted.unwrap(), twr);
    close(h.household.flows, 1_000.0);
    close(h.household.gain, 16_500.0 - 15_000.0 - 1_000.0);

    // The brokerage account: 10,500 / 10,000, 7,950 / 8,500 and 8,250 / 7,950.
    let main = &h.accounts[0];
    assert_eq!(main.name, "main");
    close(
        main.time_weighted.unwrap(),
        1.05 * (7_950.0 / 8_500.0) * (8_250.0 / 7_950.0) - 1.0,
    );
    close(main.flows, -2_000.0);
    close(main.weight_start.unwrap(), 10_000.0 / 15_000.0);

    // Gains of 500, -550 and 300 a day for each of Alice's accounts, over the household's
    // starting values of 16,000, 17,000 and 15,900, linked by its growth.
    let alice = 500.0 / 16_000.0 - 550.0 / 17_000.0 * r1 + 300.0 / 15_900.0 * r1 * r2;
    close(main.contribution, alice);
    close(h.accounts[1].contribution, alice);
    close(h.accounts[2].contribution, 0.0);
    let total: f64 = h.accounts.iter().map(|a| a.contribution).sum();
    close(total, twr);
    close(h.household.contribution, twr);

    let owners = &h.groups[0];
    assert_eq!(owners.scheme, "owner");
    let names: Vec<&str> = owners.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Alice", "Bob"]);
    close(owners.groups[0].contribution, 2.0 * alice);
    // Alice's accounts together have no net flow on the third day.
    close(owners.groups[0].flows, 0.0);
    let wrappers: Vec<&str> = h.groups[1].groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(wrappers, ["ISA", "taxable", "unclassified"]);
    assert_eq!(h.groups[1].groups[2].accounts, [account("kid")]);
}

#[test]
fn options_are_checked() {
    let (books, market) = setup();
    let mut options = HouseholdOptions::default();
    options.group_by = vec!["owner".into(), "owner".into()];
    let Err(finalytics_core::Error::Validation(report)) =
        household(&books, &market, &dates()[..1], &options)
    else {
        panic!("expected a validation error");
    };
    let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["dates", "options.group_by[1]"]);
}
