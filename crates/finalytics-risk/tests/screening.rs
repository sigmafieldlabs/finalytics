//! Screens on small universes whose answers are known: short histories, gaps, stale and missing
//! prices, a missing benchmark, fields, and every problem with the options reported together.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use finalytics_core::{Date, Error, Weekday};
use finalytics_risk::screening::{
    Columns, FieldValue, Measure, Order, Rule, ScreenOptions, ScreenResults, Securities, Series,
    Sort, Status, screen,
};

fn date(text: &str) -> Date {
    text.parse().unwrap()
}

/// `n` weekdays ending on `last`.
fn weekdays(last: &str, n: usize) -> Vec<Date> {
    let mut out = Vec::with_capacity(n);
    let mut d = date(last);
    while out.len() < n {
        if !matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday) {
            out.push(d);
        }
        d = d.add_days(-1).unwrap();
    }
    out.reverse();
    out
}

/// Closes that rise and fall in a fixed pattern from 100.
fn closes(n: usize, drift: f64) -> Vec<f64> {
    let mut p = 100.0;
    (0..n)
        .map(|k| {
            let wiggle = [0.012, -0.008, 0.004, -0.015, 0.009, 0.003, -0.006][k % 7];
            if k > 0 {
                p *= 1.0 + drift + wiggle;
            }
            p
        })
        .collect()
}

fn series(id: &str, last: &str, n: usize, drift: f64) -> Series {
    Series::new(id, weekdays(last, n), closes(n, drift))
}

fn issues(error: Error) -> Vec<(String, String)> {
    let Error::Validation(report) = error else {
        panic!("not a validation error: {error}")
    };
    report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect()
}

fn paths(error: Error) -> Vec<String> {
    issues(error).into_iter().map(|(p, _)| p).collect()
}

fn run(securities: &Securities, options: &ScreenOptions) -> ScreenResults {
    screen(securities, None, 0.0, options).unwrap()
}

fn with(f: impl FnOnce(&mut ScreenOptions)) -> ScreenOptions {
    let mut options = ScreenOptions::default();
    f(&mut options);
    options
}

#[test]
fn a_measure_whose_window_is_longer_than_the_history_has_no_value() {
    // About 14 months of closes: a year, but not three.
    let s = Securities::from_series(vec![series("NEW", "2026-10-30", 300, 0.001)]).unwrap();
    let r = run(&s, &ScreenOptions::default());
    let row = &r.securities[0];
    assert!(r.value("NEW", Measure::Return1y).is_some());
    assert!(r.value("NEW", Measure::Volatility).is_some());
    assert!(r.value("NEW", Measure::PriceToSma200).is_some());
    for m in [
        Measure::Return3y,
        Measure::Return5y,
        Measure::Volatility3y,
        Measure::CalmarRatio,
    ] {
        assert_eq!(r.value("NEW", m), None, "{m}");
    }
    // Without rules, nothing needs the longer windows: it is measured and kept.
    assert_eq!(row.status, Status::Measured);
    assert!(row.matched);
    let k = r
        .measures
        .iter()
        .position(|m| *m == Measure::Return3y)
        .unwrap();
    assert_eq!(r.coverage.lacking[k], 1);
    // A rule on a three-year measure makes it too short, and it fails the rule.
    let r = run(
        &s,
        &with(|o| o.rules = vec![Rule::above("return_3y", -1.0)]),
    );
    assert_eq!(r.securities[0].status, Status::TooShort);
    assert!(!r.securities[0].passed);
    assert_eq!(r.rules[0].lacking, 1);
    assert_eq!(r.coverage.too_short, 1);
    // One close is too short for anything but the data measures.
    let one = Securities::from_series(vec![Series::new(
        "ONE",
        vec![date("2026-10-30")],
        vec![10.0],
    )])
    .unwrap();
    let r = run(&one, &ScreenOptions::default());
    assert_eq!(r.securities[0].status, Status::TooShort);
    assert_eq!(r.value("ONE", Measure::Observations), Some(1.0));
    assert_eq!(r.value("ONE", Measure::LastPrice), Some(10.0));
    assert_eq!(r.value("ONE", Measure::Return1w), None);
}

#[test]
fn a_stale_security_is_listed_but_not_measured() {
    let s = Securities::from_series(vec![
        series("LIVE", "2026-10-30", 400, 0.0005),
        series("OLD", "2026-10-16", 400, 0.0005),
    ])
    .unwrap();
    let r = run(&s, &ScreenOptions::default());
    assert_eq!(r.as_of, date("2026-10-30"));
    let old = r.security("OLD").unwrap();
    assert_eq!(old.status, Status::Stale);
    assert!(!old.passed && !old.matched);
    assert_eq!(old.last_date, Some(date("2026-10-16")));
    assert_eq!(r.value("OLD", Measure::Stale), Some(1.0));
    assert_eq!(r.value("OLD", Measure::Observations), Some(400.0));
    assert_eq!(r.value("OLD", Measure::Return1y), None);
    assert_eq!(r.value("LIVE", Measure::Stale), Some(0.0));
    assert_eq!(r.matches, ["LIVE"]);
    assert_eq!((r.coverage.measured, r.coverage.stale), (1, 1));
    // Within the days allowed, it is measured on its last close.
    let r = run(&s, &with(|o| o.max_carry_days = 14));
    assert_eq!(r.security("OLD").unwrap().status, Status::Measured);
    // A date before the latest leaves later closes out.
    let r = run(&s, &with(|o| o.as_of = Some(date("2026-10-16"))));
    assert_eq!(r.coverage.stale, 0);
    assert_eq!(
        r.security("LIVE").unwrap().last_date,
        Some(date("2026-10-16"))
    );
}

#[test]
fn a_window_starts_at_the_last_close_on_or_before_its_first_day() {
    // Closes every weekday in October 2026 except a gap from the 5th to the 28th.
    let dates: Vec<Date> = weekdays("2026-10-30", 60)
        .into_iter()
        .filter(|d| *d < date("2026-10-05") || *d > date("2026-10-28"))
        .collect();
    let close: Vec<f64> = (0..dates.len()).map(|k| 50.0 + k as f64).collect();
    let n = close.len();
    let s =
        Securities::from_series(vec![Series::new("GAP", dates.clone(), close.clone())]).unwrap();
    let r = run(&s, &ScreenOptions::default());
    // A week before the 30th is the 23rd, in the gap: the base is the close of 2 October.
    let base = dates
        .iter()
        .rposition(|d| *d <= date("2026-10-23"))
        .unwrap();
    assert_eq!(dates[base], date("2026-10-02"));
    let expected = close[n - 1] / close[base] - 1.0;
    assert!((r.value("GAP", Measure::Return1w).unwrap() - expected).abs() < 1e-15);
    // A month back is 30 September.
    let base = dates
        .iter()
        .rposition(|d| *d <= date("2026-09-30"))
        .unwrap();
    let expected = close[n - 1] / close[base] - 1.0;
    assert!((r.value("GAP", Measure::Return1m).unwrap() - expected).abs() < 1e-15);
    // The history starts in August: three months back, 30 July, is before it.
    assert_eq!(r.value("GAP", Measure::Return3m), None);
    // The year to date starts from the last close of 2025, which the history lacks.
    assert_eq!(r.value("GAP", Measure::ReturnYtd), None);
}

#[test]
fn closes_that_are_not_numbers_are_left_out_and_bad_prices_are_errors() {
    let mut close = closes(30, 0.0);
    close[3] = f64::NAN;
    close[10] = f64::NAN;
    let s = Securities::from_series(vec![Series::new(
        "HOLES",
        weekdays("2026-10-30", 30),
        close,
    )])
    .unwrap();
    assert_eq!(s.dates(0).len(), 28);
    assert!(
        s.warnings().issues()[0]
            .message
            .contains("2 closes are not numbers")
    );
    let r = run(&s, &ScreenOptions::default());
    assert_eq!(r.value("HOLES", Measure::Observations), Some(28.0));
    assert_eq!(r.warnings.issues().len(), 1);

    // Lengths that do not match are reported first, on their own.
    let err = Securities::from_series(vec![Series::new(
        "SHORT",
        weekdays("2026-10-30", 3),
        closes(2, 0.0),
    )])
    .unwrap_err();
    assert_eq!(paths(err), ["series[0].close"]);
    let err = Securities::from_series(vec![
        Series::new("BAD", weekdays("2026-10-30", 5), {
            let mut b = closes(5, 0.0);
            b[1] = 0.0;
            b[2] = f64::INFINITY;
            b
        }),
        Series::new(
            "BAD",
            {
                let mut d = weekdays("2026-10-30", 5);
                d.swap(3, 4);
                d
            },
            closes(5, 0.0),
        )
        .with_volume(vec![1.0, -2.0, 1.0, 1.0, 1.0]),
    ])
    .unwrap_err();
    assert_eq!(
        paths(err),
        [
            "series[1].id",
            "series[0].close[1]",
            "series[0].close[2]",
            "series[1].dates[4]",
            "series[1].volume[1]",
        ]
    );
}

#[test]
fn measures_against_a_benchmark_need_one() {
    let s = Securities::from_series(vec![
        series("SPY", "2026-10-30", 600, 0.0004),
        series("ABC", "2026-10-30", 600, 0.0007),
    ])
    .unwrap();
    // Without a benchmark, every measure is computed but those against one.
    let r = run(&s, &ScreenOptions::default());
    assert!(!r.measures.iter().any(|m| m.needs_benchmark()));
    assert!(!r.measures.contains(&Measure::AverageVolume20d));
    assert!(!r.measures.contains(&Measure::DividendYield));
    // Asking for one is a problem, reported with the others.
    let err = screen(
        &s,
        None,
        0.0,
        &with(|o| {
            o.measures = Some(vec!["beta".into(), "volume_ratio".into()]);
            o.rules = vec![Rule::above("relative_return_1y", 0.0)];
        }),
    )
    .unwrap_err();
    let found = issues(err);
    assert_eq!(
        found.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
        ["measures[0]", "measures[1]", "rules[0].measure"]
    );
    assert!(found[0].1.contains("give the universe one"));
    assert!(found[1].1.contains("needs volumes"));
    // The benchmark is not a candidate by default.
    let r = screen(&s, Some(0), 0.0, &ScreenOptions::default()).unwrap();
    assert!(r.securities.iter().all(|x| x.id != "SPY"));
    assert_eq!(r.coverage.securities, s.len() - 1);
    // Screened too, against itself it has a beta and a correlation of one.
    let r = screen(&s, Some(0), 0.0, &with(|o| o.include_benchmark = true)).unwrap();
    assert!((r.value("SPY", Measure::Beta).unwrap() - 1.0).abs() < 1e-12);
    assert!((r.value("SPY", Measure::Correlation).unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(r.value("SPY", Measure::RelativeReturn1y), Some(0.0));
    let relative =
        r.value("ABC", Measure::Return1y).unwrap() - r.value("SPY", Measure::Return1y).unwrap();
    assert!((r.value("ABC", Measure::RelativeReturn1y).unwrap() - relative).abs() < 1e-15);
    // A stale benchmark gives no measure against it, with a warning.
    let stale = Securities::from_series(vec![
        series("SPY", "2026-09-30", 600, 0.0004),
        series("ABC", "2026-10-30", 600, 0.0007),
    ])
    .unwrap();
    let r = screen(&stale, Some(0), 0.0, &ScreenOptions::default()).unwrap();
    assert_eq!(r.value("ABC", Measure::Beta), None);
    assert!(r.warnings.issues().iter().any(|i| i.path == "benchmark"));
}

#[test]
fn every_problem_with_the_options_is_reported_together() {
    let s = Securities::from_series(vec![
        series("A", "2026-10-30", 300, 0.0)
            .with_field("sector", "Energy")
            .with_field("pe_ratio", 12.0),
    ])
    .unwrap();
    let mut both = Rule::above("return_1y", 0.0);
    both.field = Some("pe_ratio".into());
    let mut neither = Rule::above("return_1y", 0.0);
    neither.measure = None;
    let mut no_value = Rule::above("rsi_14", 0.0);
    no_value.value = None;
    let options = with(|o| {
        o.measures = Some(vec![
            "return_2y".into(),
            "rsi_14".into(),
            "first_date".into(),
        ]);
        o.rules = vec![
            Rule::above("returns_1y", 0.1),
            Rule::one_of("sector", ["Energy"]),
            Rule::above("sector", 1.0),
            Rule::one_of("pe_ratio", ["12"]),
            Rule::between("rsi_14", 70.0, 30.0),
            no_value,
            both,
            neither,
            Rule::above("first_date", 1.0),
            Rule::above("rsi_14", f64::NAN),
        ];
        o.sort = Some(Sort::by("sector", Order::Descending));
        o.limit = Some(0);
    });
    let err = screen(&s, None, f64::INFINITY, &options).unwrap_err();
    let found = issues(err);
    let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "risk_free_rate",
            "measures[0]",
            "rules[0].field",
            "rules[2].op",
            "rules[3].op",
            "rules[4].value",
            "rules[5].value",
            "rules[6]",
            "rules[7]",
            "rules[8].measure",
            "rules[9].value",
            "sort.field",
            "limit",
        ]
    );
    assert!(found[2].1.contains("unknown measure or field returns_1y"));
    assert!(found[2].1.contains("sector, pe_ratio") || found[2].1.contains("pe_ratio, sector"));
    assert!(found[9].1.contains("is a date"));
}

#[test]
fn rules_sort_and_limit_keep_the_matches_in_order() {
    let sectors = ["Energy", "Technology", "Energy", "Utilities", "Technology"];
    let pe = [Some(8.0), Some(30.0), None, Some(15.0), Some(22.0)];
    let drift = [0.0002, 0.0012, 0.0009, -0.0004, 0.0006];
    let universe: Vec<Series> = (0..5)
        .map(|k| {
            let mut s = series(&format!("S{k}"), "2026-10-30", 400, drift[k])
                .with_field("sector", sectors[k]);
            if let Some(x) = pe[k] {
                s = s.with_field("pe_ratio", x);
            }
            s
        })
        .collect();
    let s = Securities::from_series(universe).unwrap();
    assert_eq!(s.field_names(), ["pe_ratio", "sector"]);
    let r = run(
        &s,
        &with(|o| {
            o.measures = Some(vec!["return_1y".into()]);
            o.rules = vec![
                Rule::none_of("sector", ["Utilities"]),
                Rule::below("pe_ratio", 25.0),
            ];
            o.sort = Some(Sort::by("return_1y", Order::Descending));
        }),
    );
    // S2 has no P/E, so it fails that rule; S1's is too high; S3 is a utility.
    assert_eq!(r.matches, ["S4", "S0"]);
    assert_eq!(r.security("S4").unwrap().rank, Some(1));
    assert_eq!(r.rules[0].passed, 4);
    assert_eq!((r.rules[1].passed, r.rules[1].lacking), (3, 1));
    assert_eq!(r.rules[1].remaining, 2);
    assert_eq!(
        r.security("S0").unwrap().fields["sector"],
        FieldValue::from("Energy")
    );
    // Sorted by a field, smallest first, with the limit.
    let r = run(
        &s,
        &with(|o| {
            o.sort = Some(Sort::by("pe_ratio", Order::Ascending));
            o.limit = Some(3);
        }),
    );
    assert_eq!(r.matches, ["S0", "S3", "S4"]);
    assert!(r.security("S1").unwrap().passed && !r.security("S1").unwrap().matched);
    assert_eq!((r.coverage.passed, r.coverage.matched), (5, 3));
    // Without a value to sort by, a security comes last.
    let r = run(
        &s,
        &with(|o| o.sort = Some(Sort::by("pe_ratio", Order::Descending))),
    );
    assert_eq!(r.matches, ["S1", "S4", "S3", "S0", "S2"]);
    // The rules' measures are computed with those asked for.
    let r = run(
        &s,
        &with(|o| {
            o.measures = Some(vec![]);
            o.rules = vec![Rule::between("rsi_14", 0.0, 100.0)];
        }),
    );
    assert_eq!(r.measures, [Measure::Rsi14]);
    assert_eq!(r.coverage.passed, 5);
}

#[test]
fn fields_are_numbers_or_text_with_names_of_their_own() {
    let s = Securities::from_series(vec![
        series("A", "2026-10-30", 30, 0.0),
        series("B", "2026-10-30", 30, 0.0),
    ])
    .unwrap();
    let fields: BTreeMap<String, BTreeMap<String, FieldValue>> = [
        (
            "A".to_owned(),
            BTreeMap::from([
                ("sector".to_owned(), FieldValue::from("Energy")),
                ("rsi_14".to_owned(), FieldValue::from(3.0)),
                ("pe_ratio".to_owned(), FieldValue::from(f64::NAN)),
            ]),
        ),
        (
            "B".to_owned(),
            BTreeMap::from([("sector".to_owned(), FieldValue::from(4.0))]),
        ),
    ]
    .into();
    let err = s.clone().with_fields(fields).unwrap_err();
    assert_eq!(paths(err), ["fields.A.rsi_14", "fields.B.sector"]);
    let fields = [(
        "Z".to_owned(),
        BTreeMap::from([("sector".to_owned(), FieldValue::from("Energy"))]),
    )]
    .into();
    let s = s.with_fields(fields).unwrap();
    assert!(s.warnings().issues()[0].message.contains("no prices for Z"));
}

#[test]
fn columns_are_checked_as_series_are() {
    let days = |dates: &[&str]| dates.iter().map(|d| date(d).days_since_epoch()).collect();
    let columns = Columns::new(
        vec!["A".into(), "B".into()],
        vec![0, 2],
        days(&["2026-10-29", "2026-10-30", "2026-10-30", "2026-10-29"]),
        vec![1.0, 2.0, 3.0, 4.0],
    );
    assert_eq!(
        paths(Securities::from_columns(columns).unwrap_err()),
        ["series[1].dates[1]"]
    );
    let columns = Columns::new(
        vec!["A".into(), "B".into()],
        vec![0],
        days(&["2026-10-29"]),
        vec![1.0, 2.0],
    );
    assert_eq!(
        paths(Securities::from_columns(columns).unwrap_err()),
        ["starts", "close"]
    );
    let mut columns = Columns::new(
        vec!["A".into(), "B".into()],
        vec![0, 2],
        days(&["2026-10-29", "2026-10-30", "2026-10-29", "2026-10-30"]),
        vec![1.0, 2.0, f64::NAN, 4.0],
    );
    columns.dividends = Some(vec![0.0, f64::NAN, 0.5, 0.0]);
    let s = Securities::from_columns(columns).unwrap();
    assert_eq!(s.close(1), [4.0]);
    assert_eq!(s.dates(1), [date("2026-10-30")]);
    assert!(s.has_dividends());
}

#[test]
fn small_measures_are_exact() {
    // 280 weekdays to 30 October 2026, closes 100, 101, ... with a dividend each quarter and a
    // volume of 1,000 plus the day's number.
    let dates = weekdays("2026-10-30", 280);
    let n = dates.len();
    let close: Vec<f64> = (0..n).map(|k| 100.0 + k as f64).collect();
    let volume: Vec<f64> = (0..n).map(|k| 1_000.0 + k as f64).collect();
    let dividends: Vec<f64> = (0..n)
        .map(|k| if k % 63 == 62 { 0.5 } else { 0.0 })
        .collect();
    let s = Securities::from_series(vec![
        Series::new("UP", dates.clone(), close.clone())
            .with_volume(volume.clone())
            .with_dividends(dividends.clone()),
    ])
    .unwrap();
    let r = screen(&s, None, 0.04, &ScreenOptions::default()).unwrap();
    let v = |m: Measure| r.value("UP", m).unwrap();
    let last = close[n - 1];
    let sma = |k: usize| close[n - k..].iter().sum::<f64>() / k as f64;
    assert!((v(Measure::PriceToSma20) - last / sma(20)).abs() < 1e-14);
    assert!((v(Measure::PriceToSma200) - last / sma(200)).abs() < 1e-14);
    assert_eq!(v(Measure::Sma50Above200), 1.0);
    assert_eq!(v(Measure::Drawdown), 0.0);
    assert_eq!(v(Measure::MaxDrawdown), 0.0);
    assert_eq!(v(Measure::Rsi14), 100.0);
    assert_eq!(v(Measure::From52wHigh), 0.0);
    let year = dates
        .iter()
        .rposition(|d| *d <= date("2025-10-30"))
        .unwrap();
    assert!((v(Measure::From52wLow) - (last / close[year] - 1.0)).abs() < 1e-14);
    let paid: f64 = (0..n)
        .filter(|&k| dates[k] > date("2025-10-30"))
        .map(|k| dividends[k])
        .sum();
    assert_eq!(paid, 2.0);
    assert!((v(Measure::DividendYield) - paid / last).abs() < 1e-15);
    let recent = &volume[n - 20..];
    let average = recent.iter().sum::<f64>() / 20.0;
    assert_eq!(v(Measure::AverageVolume20d), average);
    assert!((v(Measure::VolumeRatio) - volume[n - 1] / average).abs() < 1e-15);
    let dollars: f64 = recent
        .iter()
        .zip(&close[n - 20..])
        .map(|(a, b)| a * b)
        .sum::<f64>()
        / 20.0;
    assert!((v(Measure::DollarVolume20d) - dollars).abs() < 1e-6);
    assert_eq!(v(Measure::Observations), n as f64);
    assert!(r.value("UP", Measure::Return3y).is_none());
    assert!(r.value("UP", Measure::CalmarRatio).is_none());
    // Every day gains more than the risk-free rate: no downside, and so no Sortino ratio.
    assert_eq!(v(Measure::DownsideDeviation), 0.0);
    assert_eq!(r.value("UP", Measure::SortinoRatio), None);
}

#[test]
fn dividends_that_are_all_unknown_are_none_given() {
    let n = 300;
    let days: Vec<i32> = weekdays("2026-10-30", n)
        .iter()
        .map(|d| d.days_since_epoch())
        .collect();
    let mut columns = Columns::new(
        vec!["UNKNOWN".into(), "NONE".into(), "PAID".into()],
        vec![0, n, 2 * n],
        [days.clone(), days.clone(), days].concat(),
        [closes(n, 0.0), closes(n, 0.0), closes(n, 0.0)].concat(),
    );
    let mut paid = vec![f64::NAN; n];
    paid[n - 10] = 1.5;
    columns.dividends = Some([vec![f64::NAN; n], vec![0.0; n], paid].concat());
    let s = Securities::from_columns(columns).unwrap();
    let r = run(&s, &ScreenOptions::default());
    assert_eq!(r.value("UNKNOWN", Measure::DividendYield), None);
    assert_eq!(r.value("NONE", Measure::DividendYield), Some(0.0));
    let last = s.close(2)[n - 1];
    assert_eq!(r.value("PAID", Measure::DividendYield), Some(1.5 / last));
}
