//! The reports on the shared portfolio: each figure agrees with the engine function it comes
//! from, the parts add up to their wholes, and every report turns into consistent tables.

#![allow(clippy::unwrap_used)]

use finalytics::formats::{CsvOptions, fx_from_csv, portfolio_from_csv, prices_from_csv};
use finalytics::reports::*;
use finalytics::risk::{GroupShock, HistoricalWindow, Shocks};
use finalytics::{
    Books, Currency, Date, Decimal, FlowTiming, InstrumentId, MarketData, Performance, Portfolio,
};

const ACCOUNTS: &str = include_str!("../../../tests/scenario/accounts.csv");
const INSTRUMENTS: &str = include_str!("../../../tests/scenario/instruments.csv");
const TRANSACTIONS: &str = include_str!("../../../tests/scenario/transactions.csv");
const PRICES: &str = include_str!("../../../tests/scenario/prices.csv");
const FX: &str = include_str!("../../../tests/scenario/fx.csv");

fn d(s: &str) -> Date {
    s.parse().unwrap()
}

fn setup() -> (Portfolio, Books, MarketData) {
    let options = CsvOptions::default();
    let portfolio = portfolio_from_csv(
        Currency::USD,
        INSTRUMENTS,
        TRANSACTIONS,
        Some(ACCOUNTS),
        options,
    )
    .unwrap();
    let mut market = MarketData::new();
    for (id, prices) in prices_from_csv(PRICES, options).unwrap() {
        market = market.with_prices(id, prices);
    }
    for fx in fx_from_csv(FX, options).unwrap() {
        market = market.with_fx(fx.base, fx.quote, fx.rates);
    }
    let books = Books::build(&portfolio, &market).unwrap();
    (portfolio, books, market)
}

fn options() -> ReportOptions {
    {
        let mut report_options = ReportOptions::default();
        report_options.group_by = vec!["asset_class".into(), "region".into()];
        report_options.benchmark = Some(InstrumentId::new("SPY").unwrap());
        report_options.rolling_window = 20;
        report_options
    }
}

#[track_caller]
fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-12 * b.abs().max(1.0),
        "{what}: {a} against {b}"
    );
}

fn every_table_is_rectangular(tables: &[ReportTable]) {
    assert!(!tables.is_empty());
    for t in tables {
        for row in &t.rows {
            assert_eq!(row.len(), t.columns.len(), "table {}", t.name);
        }
    }
}

#[test]
fn provenance_identifies_the_inputs() {
    let (portfolio, _, market) = setup();
    let source = Source::of(&portfolio, &market);
    assert_eq!(source.digest.len(), 64);
    assert_eq!(source, Source::of(&portfolio, &market));
    assert_eq!(source.transactions, 11);
    assert_eq!(source.accounts, 2);
    let mut changed = portfolio.clone();
    changed.transactions.pop();
    assert_ne!(Source::of(&changed, &market).digest, source.digest);
}

#[test]
fn holdings_add_up() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let r = holdings_report(&books, &market, &source, &options()).unwrap();
    assert_eq!(r.provenance.report, ReportKind::Holdings);
    assert_eq!(r.provenance.as_of, d("2026-10-30"));
    let valuation = books.value_on(d("2026-10-30"), &market).unwrap();
    assert_eq!(r.value, valuation.total_base);
    let total: Decimal = r.holdings.iter().map(|h| h.value_base).sum();
    assert_eq!(total, r.value);
    let accounts: Decimal = r.accounts.iter().map(|a| a.value_base).sum();
    assert_eq!(accounts, r.value);
    for b in &r.breakdowns {
        let w: f64 = b.slices.iter().map(|s| s.weight).sum();
        close(w, 1.0, &b.by);
    }
    let bys: Vec<&str> = r.breakdowns.iter().map(|b| b.by.as_str()).collect();
    assert_eq!(
        bys,
        [
            "holding",
            "currency",
            "kind",
            "account",
            "asset_class",
            "region"
        ]
    );
    // SPY's lots across both accounts add up to its holding.
    let spy = r
        .holdings
        .iter()
        .find(|h| h.name.as_deref() == Some("S&P 500 ETF"))
        .unwrap();
    let lots: Decimal = r
        .lots
        .iter()
        .filter(|l| l.instrument.as_str() == "SPY")
        .map(|l| l.quantity)
        .sum();
    assert_eq!(lots, spy.quantity);
    let realised: Decimal = books.realised_gains().iter().map(|g| g.gain_base).sum();
    assert_eq!(r.realised.total, realised);
    // One dividend of 180 USD with 27 withheld.
    assert_eq!(r.income.total, "153".parse::<Decimal>().unwrap());
    let c = &r.concentration;
    assert!(c.herfindahl.unwrap() > 1.0 / 3.0 && c.herfindahl.unwrap() < 1.0);
    close(
        c.effective_holdings.unwrap() * c.herfindahl.unwrap(),
        1.0,
        "effective",
    );
    every_table_is_rectangular(&r.tables());
}

#[test]
fn long_term_means_held_for_more_than_the_period() {
    // As in rebalancing's tax rates: a lot held for exactly the period is still short-term.
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let report = holdings_report(&books, &market, &source, &options()).unwrap();
    let lot = &report.lots[0];
    for (period, long_term) in [(lot.holding_days, false), (lot.holding_days - 1, true)] {
        let mut o = options();
        o.long_term_after_days = period;
        let r = holdings_report(&books, &market, &source, &o).unwrap();
        assert_eq!(r.lots[0].long_term, long_term, "period {period}");
    }
}

#[test]
fn performance_agrees_with_the_engine() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let r = performance_report(&books, &market, &source, &options()).unwrap();
    let dates = r.value.dates.clone();
    let values = books.value_series(&dates, &market).unwrap();
    let p = Performance::from_values(&values, FlowTiming::StartOfDay).unwrap();
    let itd = r.periods.iter().find(|x| x.label == "ITD").unwrap();
    let twr = p.time_weighted(dates[0], *dates.last().unwrap()).unwrap();
    close(itd.cumulative.unwrap(), twr, "since inception");
    // The months compound to the whole period.
    let months: f64 = r.monthly.values[0][..12]
        .iter()
        .flatten()
        .map(|m| 1.0 + m)
        .product();
    close(months - 1.0, twr, "months");
    close(r.calendar_years[0].portfolio, twr, "year");
    let contributions: f64 = r.contributions.iter().map(|c| c.contribution).sum();
    close(contributions, twr, "contributions");
    for scheme in &r.groups {
        let total: f64 = scheme.groups.iter().map(|g| g.contribution).sum();
        close(total, twr, &scheme.scheme);
    }
    assert!(r.benchmark_growth.is_some() && r.benchmark_monthly.is_some());
    let september = r.flows.iter().find(|f| f.month == "2026-09").unwrap();
    close(september.inflows, 120_000.0, "deposits");
    close(september.income, 153.0, "income");
    let october = r.flows.iter().find(|f| f.month == "2026-10").unwrap();
    close(october.outflows, -5_000.0, "withdrawal");
    let household = r.household.as_ref().unwrap();
    close(household.household.time_weighted.unwrap(), twr, "household");
    every_table_is_rectangular(&r.tables());
}

#[test]
fn risk_agrees_with_the_engine() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let options = options();
    let r = risk_report(&books, &market, &source, &options).unwrap();
    let stats = finalytics::risk::statistics(
        &r.returns.values,
        &finalytics::risk::StatisticsOptions::default(),
    )
    .unwrap();
    assert_eq!(r.statistics, stats);
    assert!(r.relative.is_some());
    assert_eq!(r.tail.len(), 6);
    let counted: usize = r.histogram.bins.iter().map(|b| b.count).sum();
    assert_eq!(counted, r.returns.values.len());
    let h = r.holdings.as_ref().unwrap();
    let components: f64 = h
        .decomposition
        .contributions
        .iter()
        .map(|c| c.component)
        .sum();
    close(components, h.decomposition.volatility, "components");
    assert_eq!(h.groups.len(), 2);
    let names: Vec<&str> = r.rolling.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.len(), 4, "{names:?}");
    every_table_is_rectangular(&r.tables());
}

#[test]
fn overview_and_frontier() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let o = overview_report(&books, &market, &source, &options()).unwrap();
    let p = performance_report(&books, &market, &source, &options()).unwrap();
    let itd = p.periods.iter().find(|x| x.label == "ITD").unwrap();
    close(
        o.headline.since_inception.unwrap(),
        itd.cumulative.unwrap(),
        "headline",
    );
    let frontier = o.frontier.as_ref().unwrap();
    assert_eq!(frontier.len(), 25);
    let current = o.current.unwrap();
    // The current portfolio lies on or below the frontier at its volatility.
    let best = frontier
        .iter()
        .filter(|f| f.volatility <= current.volatility + 1e-9)
        .map(|f| f.expected_return)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(best >= current.expected_return - 1e-9 || best == f64::NEG_INFINITY);
    assert!(o.holdings.iter().all(|h| !h.expected_return_given));
    every_table_is_rectangular(&o.tables());
}

#[test]
fn attribution_from_the_books() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let segment = |name: &str, w: f64, r: f64| BenchmarkSegment {
        name: name.into(),
        benchmark_weight: w,
        benchmark_return: r,
    };
    let mut options = options();
    options.attribution = Some(AttributionInputs {
        scheme: "asset_class".into(),
        periods: vec![
            BenchmarkPeriod {
                start: d("2026-09-01"),
                end: d("2026-09-30"),
                segments: vec![
                    segment("Equity", 0.6, -0.04),
                    segment("Fixed income", 0.4, 0.01),
                ],
            },
            BenchmarkPeriod {
                start: d("2026-09-30"),
                end: d("2026-10-30"),
                segments: vec![
                    segment("Equity", 0.6, -0.02),
                    segment("Fixed income", 0.4, 0.005),
                ],
            },
        ],
        linking: Default::default(),
    });
    let r = attribution_report(&books, &market, &source, &options).unwrap();
    for p in &r.periods {
        let w: f64 = p.segments.iter().map(|s| s.portfolio_weight).sum();
        close(w, 1.0, "weights");
        // Cash is not in the benchmark, so it has only an allocation effect.
        assert!(
            p.segments
                .iter()
                .any(|s| s.name == "cash" && s.benchmark_weight == 0.0)
        );
    }
    let a = &r.attribution;
    close(
        a.allocation + a.selection + a.interaction,
        a.excess_return,
        "effects",
    );
    assert!(r.time_weighted.is_some());
    every_table_is_rectangular(&r.tables());

    options.attribution = None;
    assert!(attribution_report(&books, &market, &source, &options).is_err());
}

#[test]
fn scenarios_side_by_side() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let mut options = options();
    options.scenarios = vec![
        NamedScenario {
            name: "Equities -20%".into(),
            scenario: ScenarioDefinition::Shocks {
                shocks: {
                    let mut shocks = Shocks::default();
                    shocks.groups = vec![GroupShock {
                        scheme: "asset_class".into(),
                        value: "Equity".into(),
                        shock: -0.2,
                    }];
                    shocks
                },
            },
            stress: Some(finalytics::risk::CorrelationStress {
                volatility_scale: 1.5,
                correlation_shift: 0.5,
            }),
        },
        NamedScenario {
            name: "September".into(),
            scenario: ScenarioDefinition::Historical {
                window: HistoricalWindow {
                    start: d("2026-09-01"),
                    end: d("2026-09-30"),
                    proxies: vec![],
                },
            },
            stress: None,
        },
        NamedScenario {
            name: "Broken".into(),
            scenario: ScenarioDefinition::Shocks {
                shocks: {
                    let mut shocks = Shocks::default();
                    shocks.currencies = vec![(Currency::USD, -0.1)];
                    shocks
                },
            },
            stress: None,
        },
    ];
    let r = scenarios_report(&books, &market, &source, &options).unwrap();
    assert_eq!(r.scenarios.len(), 3);
    assert!(r.scenarios[2].result.is_none() && !r.scenarios[2].problems.is_empty());
    assert_eq!(r.summary.len(), 2);
    assert_eq!(r.changes.columns, ["Equities -20%", "September"]);
    let first = r.scenarios[0].result.as_ref().unwrap();
    assert!(
        first.risk_after.as_ref().unwrap().volatility
            > first.risk_before.as_ref().unwrap().volatility
    );
    let total: f64 = r.changes.values.iter().map(|row| row[0].unwrap()).sum();
    close(total, r.summary[0].change, "changes");
    every_table_is_rectangular(&r.tables());
}

#[test]
fn every_report_reads_back_and_turns_into_tables() {
    let (portfolio, books, market) = setup();
    let source = Source::of(&portfolio, &market);
    let options = options();
    let texts = [
        serde_json::to_string(&holdings_report(&books, &market, &source, &options).unwrap()),
        serde_json::to_string(&performance_report(&books, &market, &source, &options).unwrap()),
        serde_json::to_string(&risk_report(&books, &market, &source, &options).unwrap()),
        serde_json::to_string(&overview_report(&books, &market, &source, &options).unwrap()),
    ];
    for text in texts {
        let text = text.unwrap();
        let tables = report_tables(&text).unwrap();
        every_table_is_rectangular(&tables);
    }
    assert!(report_tables("{}").is_err());
}
