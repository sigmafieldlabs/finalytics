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
    Transaction::new(
        TransactionId::new(format!("t{n}")).unwrap(),
        AccountId::new(account).unwrap(),
        d(date),
        kind,
    )
}

fn series(points: &[(&str, &str)]) -> TimeSeries<Decimal> {
    TimeSeries::new(
        points.iter().map(|(date, _)| d(date)).collect(),
        points.iter().map(|(_, v)| dec(v)).collect(),
    )
    .unwrap()
}

const DATE: &str = "2026-03-10";

fn portfolio() -> PortfolioData {
    let usd = |a: &str| Money::new(dec(a), Currency::USD);
    let buy = |instrument: &str, quantity: &str, price: &str| TransactionKind::Buy {
        instrument: id(instrument),
        quantity: dec(quantity),
        price: dec(price),
        fees: Decimal::ZERO,
    };
    {
        let mut portfolio = PortfolioData::new(Currency::USD);
        portfolio.accounts = vec![
            Account::new(main_account()),
            Account::new(AccountId::new("isa").unwrap()),
        ];
        portfolio.instruments = vec![
            Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD),
            Instrument::new(id("SAP"), InstrumentKind::Equity, Currency::EUR),
            Instrument::new(id("BOND"), InstrumentKind::Bond, Currency::USD),
        ];
        portfolio.transactions = vec![
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
        ];
        portfolio
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

fn weight(weights: &std::collections::BTreeMap<HoldingKey, f64>, key: &HoldingKey) -> f64 {
    weights.get(key).copied().unwrap_or(0.0)
}

#[test]
fn trades_reach_the_targets_when_applied() {
    let portfolio = portfolio();
    let market = market();
    let books = Books::build(&portfolio, &market).unwrap();
    let mut options = RebalanceOptions::default();
    options.lot_method = LotMethod::HighestCost;
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
    let mut options = RebalanceOptions::default();
    options.whole_units = true;
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

    let mut options = RebalanceOptions::default();
    options.minimum_trade = dec("20000");
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

/// One account holding three lots of SPY, now at 500: bought at 300 more than a year ago (a
/// long-term gain), at 520 recently (a short-term loss) and at 450 recently (a short-term gain).
fn lots_portfolio() -> (PortfolioData, MarketData) {
    let buy = |n: usize, date: &str, price: &str| {
        tx(
            n,
            "main",
            date,
            TransactionKind::Buy {
                instrument: id("SPY"),
                quantity: dec("100"),
                price: dec(price),
                fees: Decimal::ZERO,
            },
        )
    };
    let mut portfolio = PortfolioData::new(Currency::USD);
    portfolio.accounts = vec![Account::new(main_account())];
    portfolio.instruments = vec![
        Instrument::new(id("SPY"), InstrumentKind::Fund, Currency::USD),
        Instrument::new(id("BOND"), InstrumentKind::Bond, Currency::USD),
    ];
    portfolio.transactions = vec![
        tx(
            1,
            "main",
            "2024-01-02",
            TransactionKind::Deposit {
                amount: Money::new(dec("150000"), Currency::USD),
            },
        ),
        buy(2, "2024-01-02", "300"),
        buy(3, "2026-01-05", "520"),
        buy(4, "2026-02-02", "450"),
    ];
    let market = MarketData::new()
        .with_prices(
            id("SPY"),
            series(&[
                ("2024-01-02", "300"),
                ("2026-01-05", "520"),
                ("2026-02-02", "450"),
                (DATE, "500"),
            ]),
        )
        .with_prices(id("BOND"), series(&[(DATE, "100")]));
    (portfolio, market)
}

#[test]
fn tax_aware_sales_take_losses_first_and_estimate_the_tax() {
    let (portfolio, market) = lots_portfolio();
    let books = Books::build(&portfolio, &market).unwrap();
    // Holdings: 300 SPY at 500 = 150,000, plus cash 150,000 - 127,000 = 23,000; total 173,000.
    // A target of 150 units' worth sells 150 units.
    let targets = vec![
        (HoldingKey::Instrument(id("SPY")), 75_000.0 / 173_000.0),
        (HoldingKey::Instrument(id("BOND")), 0.5),
    ];
    let mut options = RebalanceOptions::default();
    options.tax = Some(TaxRates::new(dec("0.4"), dec("0.2")));
    let plan = rebalance(
        &books,
        &market,
        d(DATE),
        &main_account(),
        &targets,
        &options,
    )
    .unwrap();
    let sale = plan.trades.iter().find(|t| t.side == Side::Sell).unwrap();
    assert_eq!(sale.quantity, dec("150"));
    // Tax per unit: the loss lot -8, the short-term gain 20, the long-term gain 40.
    let taken: Vec<(Date, Decimal)> = sale.lots.iter().map(|l| (l.acquired, l.quantity)).collect();
    assert_eq!(
        taken,
        [(d("2026-01-05"), dec("100")), (d("2026-02-02"), dec("50"))]
    );
    // Gains: 100 x (500 - 520) = -2,000 and 50 x (500 - 450) = 2,500.
    assert_eq!(sale.realised_gain_base, Some(dec("500")));
    assert_eq!(sale.tax_base, Some(dec("200")));
    assert_eq!(plan.tax_base, Some(dec("200")));

    // First in, first out would have taken the long-term lot.
    let fifo = rebalance(
        &books,
        &market,
        d(DATE),
        &main_account(),
        &targets,
        &RebalanceOptions::default(),
    )
    .unwrap();
    let fifo_sale = fifo.trades.iter().find(|t| t.side == Side::Sell).unwrap();
    // 100 x (500 - 300) + 50 x (500 - 520).
    assert_eq!(fifo_sale.realised_gain_base, Some(dec("19000")));
    assert_eq!(fifo.tax_base, None);
}

#[test]
fn fees_are_paid_and_recorded_as_the_books_record_them() {
    let portfolio = portfolio();
    let market = market();
    let books = Books::build(&portfolio, &market).unwrap();
    let mut options = RebalanceOptions::default();
    options.lot_method = LotMethod::HighestCost;
    options.fees = {
        let mut fees = Fees::default();
        fees.rate = dec("0.001");
        fees.per_trade = dec("1");
        fees.minimum = dec("5");
        fees
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
    for t in &plan.trades {
        let expected = (dec("1") + dec("0.001") * t.value_local)
            .max(dec("5"))
            .round_dp(2);
        assert_eq!(t.fees_local, expected, "{}", t.instrument);
    }
    let fees: Decimal = plan
        .trades
        .iter()
        .map(|t| {
            t.fees_local
                * if t.currency == Currency::EUR {
                    dec("1.10")
                } else {
                    Decimal::ONE
                }
        })
        .sum();
    assert_eq!(plan.fees_base, fees);

    // Applied to the books, the trades realise exactly the gains proposed, and the fees do not
    // overdraw the account.
    let mut after = portfolio.clone();
    after
        .transactions
        .extend(plan.transactions("rebalance-").unwrap());
    let rebuilt = Books::build(&after, &market).unwrap();
    let gains: Decimal = rebuilt
        .realised_gains()
        .iter()
        .filter(|g| g.transaction.as_str().starts_with("rebalance-"))
        .map(|g| g.gain_base)
        .sum();
    assert_eq!(gains, plan.realised_gain_base);
    let holdings = rebuilt.holdings_on(d(DATE));
    let usd = holdings
        .cash()
        .find(|(a, c, _)| *a == &main_account() && *c == Currency::USD)
        .map(|(_, _, amount)| amount)
        .unwrap();
    assert!(usd >= Decimal::ZERO, "cash {usd}");
}

#[test]
fn invalid_fees_and_tax_rates_are_reported() {
    let portfolio = portfolio();
    let market = market();
    let books = Books::build(&portfolio, &market).unwrap();
    let mut options = RebalanceOptions::default();
    options.fees = {
        let mut fees = Fees::default();
        fees.rate = dec("-0.001");
        fees
    };
    let mut tax = TaxRates::new(dec("1.5"), dec("0.2"));
    tax.long_term_after_days = -1;
    options.tax = Some(tax);
    let Err(finalytics_core::Error::Validation(report)) = rebalance(
        &books,
        &market,
        d(DATE),
        &main_account(),
        &targets(),
        &options,
    ) else {
        panic!("expected a validation error");
    };
    let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "options.fees.rate",
            "options.tax.short_term",
            "options.tax.long_term_after_days"
        ]
    );
}

#[test]
fn several_accounts_reach_combined_targets() {
    let portfolio = portfolio();
    let market = market();
    let books = Books::build(&portfolio, &market).unwrap();
    let isa = AccountId::new("isa").unwrap();
    let accounts = [main_account(), isa.clone()];
    let plan = rebalance_accounts(
        &books,
        &market,
        d(DATE),
        &accounts,
        &targets(),
        &RebalanceOptions::default(),
    )
    .unwrap();
    // main is worth 112,940 and isa 6,000.
    assert_eq!(plan.value_base, dec("118940.00"));
    assert_eq!(plan.accounts.len(), 2);
    for (key, target) in targets() {
        let after = weight(&plan.weights_after, &key);
        // Per-account weights pass through floating point, so they hold to about 1e-8.
        assert!(
            (after - target).abs() < 1e-7,
            "{key:?}: {after} against {target}"
        );
    }
    // SPY is above its target in both accounts, so each sells the same share of its holding.
    let sold_share = |p: &Rebalance, held: &str| {
        let sale = p
            .trades
            .iter()
            .find(|t| t.instrument == id("SPY") && t.side == Side::Sell)
            .unwrap();
        sale.quantity / dec(held)
    };
    let main_share = sold_share(&plan.accounts[0], "150");
    let isa_share = sold_share(&plan.accounts[1], "10");
    assert!(
        (main_share - isa_share).abs() < dec("0.0000001"),
        "{main_share} against {isa_share}"
    );

    // Applied together, the trades leave neither account overdrawn and reach the targets.
    let mut after = portfolio.clone();
    for (k, p) in plan.accounts.iter().enumerate() {
        assert!(p.warnings.issues().is_empty(), "{:?}", p.warnings);
        after
            .transactions
            .extend(p.transactions(&format!("rebalance-{k}-")).unwrap());
    }
    let rebuilt = Books::build(&after, &market).unwrap();
    let valuation = rebuilt.value_on(d(DATE), &market).unwrap();
    for c in &valuation.cash {
        assert!(c.amount >= dec("-0.01"), "{c:?}");
    }
    let total = valuation.total_base;
    for (key, target) in targets() {
        let HoldingKey::Instrument(instrument) = key else {
            continue;
        };
        let value: Decimal = valuation
            .positions
            .iter()
            .filter(|p| p.instrument == instrument)
            .map(|p| p.market_value_base)
            .sum();
        let actual = f64::try_from(value / total).unwrap();
        assert!((actual - target).abs() < 1e-6, "{instrument}: {actual}");
    }
}
