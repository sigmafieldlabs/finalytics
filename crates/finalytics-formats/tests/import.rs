//! Reading portfolios from CSV and JSON.

#![allow(clippy::unwrap_used)]

use finalytics_core::{Currency, Date, Decimal, Error, ValidationReport};
use finalytics_formats::*;
use finalytics_ledger::{Books, InstrumentId, TransactionKind};

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn report(result: Result<impl std::fmt::Debug, Error>) -> ValidationReport {
    match result {
        Err(Error::Validation(report)) => report,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn paths(report: &ValidationReport) -> Vec<String> {
    report
        .issues()
        .iter()
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect()
}

const INSTRUMENTS: &str = "\
id,kind,currency,name,class.asset_class,id.isin
AAPL,equity,USD,Apple Inc.,Equity,US0378331005
SAP,equity,EUR,SAP SE,Equity,
ESZ6,future,USD,E-mini S&P Dec 2026,Equity,
";

const TRANSACTIONS: &str = "\
id,account,trade_date,type,instrument,quantity,price,fees,amount,currency,lot_method,lots
t1,main,2026-01-02,deposit,,,,,10000,USD,,
t2,main,2026-01-05,buy,AAPL,10,150,5,,,,
t3,main,2026-03-02,buy,AAPL,10,200,5,,,,
t4,main,2026-06-01,sell,AAPL,4,190,0,,,specific_id,t3:3;t2:1
t5,main,2026-06-02,dividend,AAPL,,,,12.5,USD,,
";

#[test]
fn reads_instruments_with_classifications_and_identifiers() {
    let instruments = instruments_from_csv(INSTRUMENTS, CsvOptions::default()).unwrap();
    assert_eq!(instruments.len(), 3);
    assert_eq!(instruments[0].classifications["asset_class"], "Equity");
    assert_eq!(instruments[0].identifiers["isin"], "US0378331005");
    assert!(instruments[1].identifiers.is_empty());
    assert_eq!(instruments[1].currency, Currency::EUR);
}

#[test]
fn reads_transactions_of_each_shape() {
    let transactions = transactions_from_csv(TRANSACTIONS, CsvOptions::default()).unwrap();
    assert_eq!(transactions.len(), 5);
    let TransactionKind::Sell {
        lots, lot_method, ..
    } = &transactions[3].kind
    else {
        panic!()
    };
    assert_eq!(
        lot_method.map(|m| format!("{m:?}")),
        Some("SpecificId".into())
    );
    assert_eq!(
        lots.iter()
            .map(|l| (l.lot.as_str(), l.quantity))
            .collect::<Vec<_>>(),
        [("t3", dec("3")), ("t2", dec("1"))]
    );
}

#[test]
fn a_portfolio_from_csv_builds_books() {
    let portfolio = portfolio_from_csv(
        Some(Currency::USD),
        Some(INSTRUMENTS),
        TRANSACTIONS,
        None,
        CsvOptions::default(),
    )
    .unwrap();
    assert_eq!(portfolio.accounts.len(), 1);
    let books = Books::build(&portfolio, &Default::default()).unwrap();
    let gains: Vec<_> = books
        .realised_gains()
        .iter()
        .map(|g| (g.lot.as_str(), g.gain_local))
        .collect();
    // 3 units of t3 at 200.5 each, then 1 unit of t2 at 150.5, all sold at 190.
    assert_eq!(gains, [("t3", dec("-31.5")), ("t2", dec("39.5"))]);
}

#[test]
fn every_problem_is_reported_with_its_line_and_column() {
    let csv = "\
id,account,trade_date,type,instrument,quantity,price,amount,currency
t1,main,2026-01-02,deposit,,,,1000,usd
t2,main,2026-02-30,buy,AAPL,ten,150,,
t3,main,2026-03-02,swap,AAPL,1,1,,
t4,main,2026-03-03,sell,AAPL,1,,,
";
    let report = report(transactions_from_csv(csv, CsvOptions::default()));
    assert_eq!(
        paths(&report),
        [
            r#"line 2 (currency): invalid currency "usd": expected three upper-case letters, such as "USD""#,
            "line 3 (trade_date): invalid date: 2026-02 has 28 days, so day 30 does not exist",
            r#"line 3 (quantity): "ten" is not a number (use a dot for decimals and no thousands separators)"#,
            r#"line 4 (type): "swap" is not a transaction type"#,
            "line 5 (price): is required",
        ]
    );
}

#[test]
fn missing_columns_are_reported_once() {
    let report = report(instruments_from_csv(
        "id,currency\nAAPL,USD\n",
        CsvOptions::default(),
    ));
    assert_eq!(paths(&report), [r#"line 1: missing column "kind""#]);
}

#[test]
fn a_portfolio_needs_only_dated_transactions() {
    let csv = "date,type,instrument,quantity,price,amount,currency
2026-01-02,deposit,,,,10000,EUR
2026-01-02,buy,ASML,5,640,,
2026-03-02,dividend,ASML,,,8.10,EUR
2026-01-05,buy,VOO,10,500,,
2026-01-05,dividend,VOO,,,1.50,USD
";
    let portfolio = portfolio_from_csv(None, None, csv, None, CsvOptions::default()).unwrap();
    assert_eq!(portfolio.base_currency, Currency::EUR);
    let ids: Vec<&str> = portfolio
        .transactions
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    assert_eq!(ids, ["t1", "t2", "t3", "t4", "t5"]);
    assert!(
        portfolio
            .transactions
            .iter()
            .all(|t| t.account.as_str() == "main")
    );
    let currencies: Vec<(&str, Currency)> = portfolio
        .instruments
        .iter()
        .map(|i| (i.id.as_str(), i.currency))
        .collect();
    assert_eq!(
        currencies,
        [("ASML", Currency::EUR), ("VOO", Currency::USD)]
    );
    let without = "date,type,instrument,quantity,price\n2026-01-02,buy,VOO,1,500\n";
    let report = report(portfolio_from_csv(
        None,
        None,
        without,
        None,
        CsvOptions::default(),
    ));
    assert_eq!(paths(&report).len(), 1);
    assert!(paths(&report)[0].starts_with("base_currency:"));
}

#[test]
fn structural_problems_come_back_under_their_file() {
    let transactions =
        "id,account,trade_date,type,instrument,quantity,price\nt1,main,2026-01-02,buy,MSFT,1,10\n";
    let report = report(portfolio_from_csv(
        Some(Currency::USD),
        Some(INSTRUMENTS),
        transactions,
        None,
        CsvOptions::default(),
    ));
    assert_eq!(
        paths(&report),
        ["portfolio.transactions[0].instrument: unknown instrument MSFT"]
    );
}

#[test]
fn accounts_with_classifications() {
    let accounts = "id,name,lot_method,class.owner,class.wrapper\n\
                    main,Brokerage,highest_cost,Alice,taxable\n\
                    isa,Stocks and shares ISA,,Alice,ISA\n\
                    spare,,,Bob,\n";
    let portfolio = portfolio_from_csv(
        Some(Currency::USD),
        Some(INSTRUMENTS),
        TRANSACTIONS,
        Some(accounts),
        CsvOptions::default(),
    )
    .unwrap();
    let ids: Vec<&str> = portfolio.accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["main", "isa", "spare"]);
    let main = &portfolio.accounts[0];
    assert_eq!(main.name.as_deref(), Some("Brokerage"));
    assert_eq!(main.lot_method, finalytics_ledger::LotMethod::HighestCost);
    assert_eq!(main.classifications["wrapper"], "taxable");
    assert_eq!(
        portfolio.accounts[1].lot_method,
        finalytics_ledger::LotMethod::Fifo
    );
    assert!(
        !portfolio.accounts[2]
            .classifications
            .contains_key("wrapper")
    );

    // With an accounts file, a transaction in an account it does not list is an error.
    let unlisted = report(portfolio_from_csv(
        Some(Currency::USD),
        Some(INSTRUMENTS),
        TRANSACTIONS,
        Some("id\nisa\n"),
        CsvOptions::default(),
    ));
    assert!(
        !paths(&unlisted).is_empty()
            && paths(&unlisted)
                .iter()
                .all(|p| p.starts_with("portfolio.transactions[")
                    && p.ends_with("unknown account main")),
        "{:?}",
        paths(&unlisted)
    );

    let malformed = report(accounts_from_csv(
        "id,lot_method\nmain,newest\n,fifo\n",
        CsvOptions::default(),
    ));
    assert_eq!(
        paths(&malformed),
        [
            r#"line 2 (lot_method): "newest" is not a lot method; use fifo, lifo, highest_cost, lowest_cost, average_cost or specific_id"#,
            "line 3 (id): is required"
        ]
    );
}

#[test]
fn prices_in_long_and_wide_layouts() {
    let long =
        "date,instrument,price\n2026-11-03,AAPL,201\n2026-11-02,AAPL,200\n2026-11-02,SAP,150.5\n";
    let prices = prices_from_csv(long, CsvOptions::default()).unwrap();
    let aapl = &prices[&InstrumentId::new("AAPL").unwrap()];
    assert_eq!(aapl.dates(), [d("2026-11-02"), d("2026-11-03")]);

    let wide = "date;AAPL;SAP\n2026-11-02;200;150.5\n2026-11-03;201;\n";
    let wide = prices_from_csv(wide, {
        let mut csv_options = CsvOptions::default();
        csv_options.delimiter = b';';
        csv_options
    })
    .unwrap();
    assert_eq!(wide, prices);

    let report = report(prices_from_csv(
        "date,instrument,price\n2026-11-02,AAPL,200\n2026-11-02,AAPL,201\n2026-11-03,AAPL,0\n",
        CsvOptions::default(),
    ));
    assert_eq!(
        paths(&report),
        [
            "line 3: a second price for AAPL on 2026-11-02",
            "line 4: the price for AAPL on 2026-11-03 must be greater than zero"
        ]
    );
}

#[test]
fn fx_rates_in_long_and_wide_layouts() {
    let long = fx_from_csv(
        "date,base,quote,rate\n2026-11-02,EUR,USD,1.08\n2026-11-02,USD,JPY,150\n",
        CsvOptions::default(),
    )
    .unwrap();
    let wide = fx_from_csv(
        "date,EUR/USD,usdjpy\n2026-11-02,1.08,150\n",
        CsvOptions::default(),
    )
    .unwrap();
    assert_eq!(long, wide);
    assert_eq!(
        (long[0].base, long[0].quote),
        (Currency::EUR, Currency::USD)
    );
    assert!(fx_from_csv("date,EURO\n2026-11-02,1\n", CsvOptions::default()).is_err());
}

#[test]
fn documents_round_trip_with_their_market_data() {
    let portfolio = portfolio_from_csv(
        Some(Currency::USD),
        Some(INSTRUMENTS),
        TRANSACTIONS,
        None,
        CsvOptions::default(),
    )
    .unwrap();
    let prices = prices_from_csv(
        "date,instrument,price\n2026-11-02,AAPL,200\n",
        CsvOptions::default(),
    )
    .unwrap();
    let mut market = finalytics_ledger::MarketData::new().with_max_carry_days(3);
    for (id, series) in prices {
        market = market.with_prices(id, series);
    }
    let mut document = PortfolioDocument::new(portfolio);
    document.market = Some(market);
    let json = document.to_json();
    assert!(json.contains(r#""format": "finalytics.portfolio""#));
    assert_eq!(PortfolioDocument::from_json(&json).unwrap(), document);
}

#[test]
fn malformed_documents_say_where() {
    let newer = r#"{"format":"finalytics.portfolio","format_version":2,"portfolio":{}}"#;
    assert!(
        paths(&report(PortfolioDocument::from_json(newer)))[0]
            .starts_with("format_version: this document uses format version 2")
    );

    let bad_quantity = r#"{"format":"finalytics.portfolio","format_version":1,"portfolio":{"base_currency":"USD","accounts":[{"id":"main"}],
        "instruments":[{"id":"AAPL","kind":"equity","currency":"USD"}],
        "transactions":[{"id":"t1","account":"main","trade_date":"2026-01-02","type":"buy","instrument":"AAPL","quantity":"ten","price":"1"}]}}"#;
    let report = report(PortfolioDocument::from_json(bad_quantity));
    assert_eq!(report.issues()[0].path, "portfolio.transactions[0]");

    let unknown = r#"{"format":"finalytics.portfolio","format_version":1,"portfolio":{"base_currency":"USD","accounts":[],
        "instruments":[],"transactions":[{"id":"t1","account":"main","trade_date":"2026-01-02","type":"deposit","amount":{"amount":"5","currency":"USD"}}]}}"#;
    assert_eq!(
        paths(&self::report(PortfolioDocument::from_json(unknown))),
        ["portfolio.transactions[0].account: unknown account main"]
    );
}
