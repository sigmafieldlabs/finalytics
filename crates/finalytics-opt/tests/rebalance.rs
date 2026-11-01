//! Proposed trades applied to the books reach their targets.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Currency, Date, Decimal, Money, TimeSeries};
use finalytics_ledger::*;
use finalytics_opt::*;

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn id(s: &str) -> InstrumentId {
    InstrumentId::new(s).unwrap()
}

fn main_account() -> AccountId {
    AccountId::new("main").unwrap()
}

fn tx(n: usize, account: &str, date: &str, kind: TransactionKind) -> Transaction {
    Transaction {
        id: TransactionId::new(format!("t{n}")).unwrap(),
        account: AccountId::new(account).unwrap(),
        trade_date: d(date),
        settle_date: None,
        kind,
        note: None,
    }
}

fn series(points: &[(&str, &str)]) -> TimeSeries<Decimal> {
    TimeSeries::new(
        points.iter().map(|(date, _)| d(date)).collect(),
        points.iter().map(|(_, v)| dec(v)).collect(),
    )
    .unwrap()
}

const DATE: &str = "2026-03-10";

fn portfolio() -> Portfolio {
    let usd = |a: &str| Money::new(dec(a), Currency::USD);
    let buy = |instrument: &str, quantity: &str, price: &str| TransactionKind::Buy {
        instrument: id(instrument),
        quantity: dec(quantity),
        price: dec(price),
        fees: Decimal::ZERO,
    };
    Portfolio {
        base_currency: Currency::USD,
        accounts: vec![
            Account::new(main_account()),
            Account::new(AccountId::new("isa").unwrap()),
        ],
        instruments: vec![
            Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD),
            Instrument::new(id("SAP"), InstrumentKind::Equity, Currency::EUR),
            Instrument::new(id("BOND"), InstrumentKind::Bond, Currency::USD),
        ],
        transactions: vec![
            tx(
                1,
                "main",
                "2026-03-02",
                TransactionKind::Deposit {
                    amount: usd("100000"),
                },
            ),
            tx(2, "main", "2026-03-02", buy("SPY", "100", "400")),
            tx(3, "main", "2026-03-03", buy("SPY", "50", "450")),
            tx(
                4,
                "main",
                "2026-03-03",
                TransactionKind::FxConversion {
                    sell: usd("11000"),
                    buy: Money::new(dec("10000"), Currency::EUR),
                },
            ),
            tx(5, "main", "2026-03-03", buy("SAP", "40", "200")),
            tx(
                6,
                "isa",
                "2026-03-02",
                TransactionKind::Deposit {
                    amount: usd("5000"),
                },
            ),
            tx(7, "isa", "2026-03-02", buy("SPY", "10", "400")),
        ],
    }
}

fn market() -> MarketData {
    MarketData::new()
        .with_prices(id("SPY"), series(&[("2026-03-02", "400"), (DATE, "500")]))
        .with_prices(id("SAP"), series(&[("2026-03-03", "200"), (DATE, "210")]))
        .with_prices(id("BOND"), series(&[(DATE, "100")]))
        .with_fx(
            Currency::EUR,
            Currency::USD,
            series(&[
                ("2026-03-02", "1.10"),
                ("2026-03-03", "1.10"),
                (DATE, "1.10"),
            ]),
        )
}

fn targets() -> Vec<(HoldingKey, f64)> {
    vec![
        (HoldingKey::Instrument(id("SPY")), 0.3),
        (HoldingKey::Instrument(id("BOND")), 0.4),
        (HoldingKey::Instrument(id("SAP")), 0.2),
    ]
}

fn weight(weights: &[(HoldingKey, f64)], key: &HoldingKey) -> f64 {
    weights
        .iter()
        .find(|(k, _)| k == key)
        .map_or(0.0, |(_, w)| *w)
}

#[test]
fn trades_reach_the_targets_when_applied() {
    let portfolio = portfolio();
    let market = market();
    let books = Books::build(&portfolio, &market).unwrap();
    let options = RebalanceOptions {
        lot_method: LotMethod::HighestCost,
        ..Default::default()
    };
    let plan = rebalance(
        &books,
        &market,
        d(DATE),
        &main_account(),
        &targets(),
        &options,
    )
    .unwrap();

    // SPY 75,000; SAP 40 at 210 euros; 2,000 euros; 26,500 dollars.
    assert_eq!(plan.value_base, dec("112940.00"));
    assert!(plan.warnings.issues().is_empty(), "{:?}", plan.warnings);
    let sides: Vec<_> = plan
        .trades
        .iter()
        .map(|t| (t.instrument.as_str(), t.side))
        .collect();
    assert_eq!(
        sides,
        [("SPY", Side::Sell), ("BOND", Side::Buy), ("SAP", Side::Buy)]
    );

    // The sale of 82.236 units takes the 450 lot first, then 32.236 units of the 400 lot.
    let sale = &plan.trades[0];
    assert_eq!(sale.quantity, dec("82.236"));
    let lots: Vec<_> = sale.lots.iter().map(|l| l.quantity).collect();
    assert_eq!(lots, [dec("50"), dec("32.236")]);
    assert_eq!(sale.realised_gain_base, Some(dec("5723.6")));
    assert_eq!(plan.conversions.len(), 1);
    assert_eq!(plan.conversions[0].buy.currency(), Currency::EUR);

    // Apply the proposal and value the account again.
    let mut after = portfolio.clone();
    after
        .transactions
        .extend(plan.transactions("rebalance-").unwrap());
    let rebuilt = Books::build(&after, &market).unwrap();
    let gains: Decimal = rebuilt
        .realised_gains()
        .iter()
        .filter(|g| g.transaction.as_str() == "rebalance-1")
        .map(|g| g.gain_base)
        .sum();
    assert!((gains - plan.realised_gain_base).abs() < dec("0.000001"));

    let valuation = rebuilt.value_on(d(DATE), &market).unwrap();
    let account_value: Decimal = valuation
        .positions
        .iter()
        .filter(|p| p.account == main_account())
        .map(|p| p.market_value_base)
        .chain(
            valuation
                .cash
                .iter()
                .filter(|c| c.account == main_account())
                .map(|c| c.value_base),
        )
        .sum();
    assert!((account_value - plan.value_base).abs() < dec("0.01"));
    for (key, target) in targets() {
        let HoldingKey::Instrument(instrument) = &key else {
            continue;
        };
        let value: Decimal = valuation
            .positions
            .iter()
            .filter(|p| p.account == main_account() && &p.instrument == instrument)
            .map(|p| p.market_value_base)
            .sum();
        // Conversions are rounded to cents, which moves the account's value by a fraction of a
        // cent.
        let actual = f64::try_from(value / account_value).unwrap();
        assert!(
            (actual - target).abs() < 1e-7,
            "{instrument}: {actual} against {target}"
        );
        // The proposal's own account of the weights after matches the rebuilt books.
        assert!((weight(&plan.weights_after, &key) - actual).abs() < 1e-12);
    }
    // The euro balance is spent down to rounding; the rest of the cash is in dollars.
    let euros: Decimal = valuation
        .cash
        .iter()
        .filter(|c| c.account == main_account() && c.currency == Currency::EUR)
        .map(|c| c.amount)
        .sum();
    assert!(euros.abs() <= dec("0.005"));
    // The other account is untouched.
    assert_eq!(
        rebuilt
            .holdings_on(d(DATE))
            .lots_of(&AccountId::new("isa").unwrap(), &id("SPY"))
            .len(),
        1
    );
}

#[test]
fn whole_units_and_minimum_trades() {
    let books = Books::build(&portfolio(), &market()).unwrap();
    let options = RebalanceOptions {
        whole_units: true,
        ..Default::default()
    };
    let plan = rebalance(
        &books,
        &market(),
        d(DATE),
        &main_account(),
        &targets(),
        &options,
    )
    .unwrap();
    assert!(plan.trades.iter().all(|t| t.quantity == t.quantity.trunc()));
    // First in, first out: the 400 lot goes first.
    assert_eq!(plan.trades[0].lots[0].quantity, dec("82"));

    let options = RebalanceOptions {
        minimum_trade: dec("20000"),
        ..Default::default()
    };
    let plan = rebalance(
        &books,
        &market(),
        d(DATE),
        &main_account(),
        &targets(),
        &options,
    )
    .unwrap();
    let kept: Vec<_> = plan.trades.iter().map(|t| t.instrument.as_str()).collect();
    assert_eq!(kept, ["SPY", "BOND"]);
    assert!(
        plan.warnings
            .issues()
            .iter()
            .any(|i| i.path == "trades.SAP")
    );
}

#[test]
fn selling_everything_not_in_the_targets() {
    let books = Books::build(&portfolio(), &market()).unwrap();
    let targets = vec![(HoldingKey::Instrument(id("BOND")), 1.0)];
    let plan = rebalance(
        &books,
        &market(),
        d(DATE),
        &main_account(),
        &targets,
        &Default::default(),
    )
    .unwrap();
    let sold: Vec<_> = plan
        .trades
        .iter()
        .filter(|t| t.side == Side::Sell)
        .map(|t| (t.instrument.as_str(), t.quantity))
        .collect();
    assert_eq!(sold, [("SAP", dec("40")), ("SPY", dec("150"))]);
    // Sales of 75,000 and 9,240 dollars, and a purchase of the whole 112,940.
    let traded = 75_000.0 + 9_240.0 + 112_940.0;
    assert!(
        (plan.turnover - traded / 112_940.0).abs() < 1e-12,
        "{}",
        plan.turnover
    );
}

#[test]
fn invalid_targets_are_reported() {
    let books = Books::build(&portfolio(), &market()).unwrap();
    let targets = vec![
        (HoldingKey::Instrument(id("SPY")), 0.7),
        (HoldingKey::Instrument(id("GOLD")), 0.2),
        (HoldingKey::Instrument(id("SPY")), 0.3),
    ];
    let Err(finalytics_core::Error::Validation(report)) = rebalance(
        &books,
        &market(),
        d(DATE),
        &main_account(),
        &targets,
        &Default::default(),
    ) else {
        panic!("expected a validation error");
    };
    let paths: Vec<_> = report.issues().iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["targets[1]", "targets[2]", "targets"]);
}
