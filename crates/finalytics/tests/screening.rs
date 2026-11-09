//! A screen of the S&P 500 prices in `tests/sp500`: the report prints in 80 columns and lays out
//! as a page, reads back from JSON as every report does, and the raw results print.

#![allow(clippy::unwrap_used)]

use finalytics::prelude::*;
use finalytics::reports::{report_html, report_tables, report_text};
use finalytics::screening::{Columns, Measure, Status};

fn universe() -> Universe {
    Universe::builder()
        .prices_csv(include_str!("../../../tests/sp500/prices.csv"))
        .fields_csv(include_str!("../../../tests/sp500/companies.csv"))
        .benchmark("T-Bills")
        .risk_free_rate(0.03)
        .build()
        .unwrap()
}

#[test]
fn the_report_prints_in_80_columns_and_shows_every_match() {
    let u = universe();
    assert_eq!(u.len(), 56);
    assert_eq!(u.fields(), ["name", "sector", "industry"]);
    let screen = u.screen().name("Every company").build().unwrap();
    let text = screen.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(text.contains("Every company"));
    // T-Bills, the benchmark, serves beta and is not a candidate unless asked for.
    assert!(text.contains("The top 50 of 55 matches; the page lists them all."));
    let html = screen.report.to_html(None);
    assert!(html.starts_with("<!doctype html>"));
    assert_eq!(html.matches(r#"<td class="rk">"#).count(), 55);
    assert!(screen.results.securities.iter().all(|s| s.id != "T-Bills"));
    let with = u.screen().include_benchmark().build().unwrap();
    assert_eq!(with.results.securities.len(), 56);
    assert!(with.results.securities.iter().any(|s| s.id == "T-Bills"));
    // A screen with rules on fields and measures, sorted and limited.
    let screen = u
        .screen()
        .as_of("2026-06-30")
        .above("return_1y", 0.0)
        .one_of("sector", ["Information Technology", "Energy"])
        .between("rsi_14", 30.0, 70.0)
        .sort("sharpe_ratio", Order::Descending)
        .limit(5)
        .build()
        .unwrap();
    let r = &screen.results;
    assert!(r.matches.len() <= 5);
    for id in &r.matches {
        let s = r.security(id).unwrap();
        assert_eq!(s.status, Status::Measured);
        assert!(r.value(id, Measure::Return1y).unwrap() > 0.0);
    }
    let text = screen.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(words(&text).contains("Sector is Information Technology or Energy"));
    assert!(words(&text).contains("RSI (14 days) between 30 and 70"));
    // The report reads back from JSON, as the bindings pass it.
    let json = serde_json::to_string(&screen.report).unwrap();
    assert_eq!(
        report_text(&json, None).unwrap(),
        screen.report.to_text(None)
    );
    assert!(
        report_html(&json, None)
            .unwrap()
            .contains("Screening report")
    );
    let names: Vec<String> = report_tables(&json)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(
        names,
        ["coverage", "rules", "matches", "near_misses", "spread"]
    );
    let results = serde_json::to_string(&screen.results).unwrap();
    let printed = finalytics::results_text(&results).unwrap();
    assert!(printed.starts_with("ScreenResults · as of 2026-06-30"));
    assert_eq!(printed, screen.results.to_string());
}

/// The words of a report, whatever the line breaks.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn a_benchmark_among_the_securities_leaves_the_others_in_order_with_their_own_fields() {
    let all = universe();
    let u = Universe::builder()
        .prices_csv(include_str!("../../../tests/sp500/prices.csv"))
        .fields_csv(include_str!("../../../tests/sp500/companies.csv"))
        .benchmark("AAPL")
        .build()
        .unwrap();
    let screen = u.screen().build().unwrap();
    let ids: Vec<&str> = screen
        .results
        .securities
        .iter()
        .map(|s| s.id.as_str())
        .collect();
    let others: Vec<&str> = u
        .ids()
        .iter()
        .map(String::as_str)
        .filter(|id| *id != "AAPL")
        .collect();
    assert_eq!(ids, others);
    assert_eq!(screen.results.matches, others);
    let names = |screen: &finalytics::screening::Screen| -> Vec<(String, String)> {
        screen
            .results
            .securities
            .iter()
            .map(|s| (s.id.clone(), format!("{:?}", s.fields.get("name"))))
            .collect()
    };
    let everyone = names(&all.screen().include_benchmark().build().unwrap());
    for pair in names(&screen) {
        assert!(everyone.contains(&pair), "{pair:?}");
    }
}

#[test]
fn the_measures_leave_out_those_no_security_has() {
    // Two and a half years into the prices, no company has three years of closes.
    let screen = universe().screen().as_of("2007-06-29").build().unwrap();
    let r = &screen.results;
    let k = r
        .measures
        .iter()
        .position(|m| *m == Measure::Return3y)
        .unwrap();
    assert_eq!(r.coverage.lacking[k], 55);
    assert_eq!(r.coverage.lacking_history[k], 55);
    let spread = screen.report.spread.iter().find(|s| s.name == "return_3y");
    assert_eq!(spread.map(|s| (s.count, s.lacking_history)), Some((0, 55)));
    let text = screen.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(
        text.contains("With no rules, all 55 securities pass."),
        "{text}"
    );
    let html = words(&screen.report.to_html(None));
    assert!(
        html.contains(
            "Not computed, as no history is long enough: 3-year return, per year; 5-year \
             return, per year; 3-year volatility; 3-year max drawdown; 3-year Sharpe ratio; \
             Calmar ratio."
        ),
        "{html}"
    );
    assert!(!html.contains("<tr><td>3-year return, per year</td>"));
    assert!(html.contains("<tr><td>1-year return</td>"));

    // A benchmark with no close in the last four months is stale, and leaves the measures
    // against it with no value, for lack of the benchmark rather than of history.
    let prices: String = include_str!("../../../tests/sp500/prices.csv")
        .lines()
        .map(|line| match line.split_once(',') {
            Some((date, _)) if date > "2026-06-30" && date.starts_with("20") => {
                format!("{},\n", line.rsplit_once(',').unwrap().0)
            }
            _ => format!("{line}\n"),
        })
        .collect();
    let u = Universe::builder()
        .prices_csv(&prices)
        .benchmark("T-Bills")
        .build()
        .unwrap();
    let screen = u.screen().build().unwrap();
    let html = words(&screen.report.to_html(None));
    assert!(
        html.contains(
            "No security has a value of beta, correlation, 6-month return vs benchmark or \
             1-year return vs benchmark."
        ),
        "{html}"
    );
    assert!(!html.contains("no history is long enough"), "{html}");
    // The stale benchmark is not a candidate, so it is not listed, unless asked for.
    assert!(screen.report.exceptions.stale.is_empty());
    let with = u.screen().include_benchmark().build().unwrap();
    assert_eq!(with.report.exceptions.stale, ["T-Bills"]);
    let text = words(&with.to_string());
    assert!(
        text.contains(
            "55 of 56 securities have at least two prices. 1 has no price in the 5 days to 30 \
             Oct 2026 and is left out: T-Bills."
        ),
        "{text}"
    );
}

#[test]
fn the_data_names_the_benchmark_when_given_and_always_the_risk_free_rate() {
    let prices = include_str!("../../../tests/sp500/prices.csv");
    let data = |u: &Universe| {
        let screen = u.screen().measures(["return_1y"]).build().unwrap();
        let text = screen.to_string();
        assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
        let html = screen.report.to_html(None);
        (
            text[text.find("\nData\n").unwrap()..].to_owned(),
            html[html.find(r#"<section id="data">"#).unwrap()..].to_owned(),
        )
    };
    // Both, in one line under the line on the data.
    let (text, html) = data(&universe());
    let line = "Benchmark: T-Bills · risk-free rate: 3% a year.";
    assert!(text.contains(&format!("\n  {line}\n")), "{text}");
    assert!(
        html.contains(&format!(r#"<p class="dline">{line}</p>"#)),
        "{html}"
    );
    // The rate alone.
    let u = Universe::builder()
        .prices_csv(prices)
        .risk_free_rate(0.0425)
        .build()
        .unwrap();
    let (text, html) = data(&u);
    assert!(
        text.contains("\n  Risk-free rate: 4.25% a year.\n"),
        "{text}"
    );
    assert!(!text.contains("Benchmark") && !html.contains("Benchmark"));
    // Neither: the rate the ratios used, the default 0%, and no benchmark.
    let u = Universe::builder().prices_csv(prices).build().unwrap();
    let (text, html) = data(&u);
    assert!(text.contains("\n  Risk-free rate: 0% a year.\n"), "{text}");
    assert!(!text.contains("Benchmark") && !html.contains("Benchmark"));
    assert!(
        html.contains(r#"<p class="dline">Risk-free rate: 0% a year.</p>"#),
        "{html}"
    );
}

#[test]
fn columns_give_the_same_universe_as_series() {
    let u = universe();
    let mut columns = Columns::default();
    for id in u.ids() {
        let s = u.series(id).unwrap();
        columns.ids.push(id.clone());
        columns.starts.push(columns.days.len());
        columns
            .days
            .extend(s.dates.iter().map(|d| d.days_since_epoch()));
        columns.close.extend(&s.close);
    }
    let from_columns = Universe::from_columns(columns)
        .unwrap()
        .with_benchmark("T-Bills")
        .unwrap()
        .with_risk_free_rate(0.03);
    let a = u.screen().measures(["return_1y", "beta"]).build().unwrap();
    let b = from_columns
        .screen()
        .measures(["return_1y", "beta"])
        .build()
        .unwrap();
    for (x, y) in a.results.securities.iter().zip(&b.results.securities) {
        assert_eq!(x.values, y.values);
    }
}

#[test]
fn formulas_are_numeric_fields_that_rules_and_the_sort_take() {
    use std::collections::BTreeMap;

    use finalytics::screening::FieldValue;

    let u = universe();
    // A price-to-earnings ratio for each company, and none for one.
    let fields: BTreeMap<String, BTreeMap<String, FieldValue>> = u
        .ids()
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != 3)
        .map(|(k, id)| {
            let pe = BTreeMap::from([("pe".to_owned(), FieldValue::Number(8.0 + k as f64))]);
            (id.clone(), pe)
        })
        .collect();
    let u = u.with_fields(fields).unwrap();
    let screen = u
        .screen()
        .as_of("2007-06-29")
        .formula("risk_adjusted", "return_1y / volatility")
        .formula("per_pe", "max(risk_adjusted, 0) / pe")
        .above("risk_adjusted", 0.5)
        .sort("per_pe", Order::Descending)
        .build()
        .unwrap();
    let r = &screen.results;
    assert!(
        r.fields
            .ends_with(&["risk_adjusted".to_owned(), "per_pe".to_owned()])
    );
    let at = |m: Measure| r.measures.iter().position(|x| *x == m).unwrap();
    let number = |s: &finalytics::screening::SecurityScreen, name: &str| match s.fields.get(name) {
        Some(FieldValue::Number(x)) => Some(*x),
        _ => None,
    };
    for s in &r.securities {
        let expected = match (
            s.values[at(Measure::Return1y)],
            s.values[at(Measure::Volatility)],
        ) {
            (Some(a), Some(b)) if b != 0.0 => Some(a / b),
            _ => None,
        };
        assert_eq!(number(s, "risk_adjusted"), expected, "{}", s.id);
        if s.passed {
            assert!(number(s, "risk_adjusted").unwrap() > 0.5);
        }
    }
    // The company without a ratio has no value of the formula on it, and is counted.
    let without = &r.securities[3];
    assert!(number(without, "per_pe").is_none());
    assert!(r.formulas[1].lacking >= 1);
    let keys: Vec<f64> = r
        .matches
        .iter()
        .filter_map(|id| number(r.security(id).unwrap(), "per_pe"))
        .collect();
    assert!(keys.windows(2).all(|w| w[0] >= w[1]), "{keys:?}");
    // The report lists each formula with the data, in both forms, and as a table.
    let text = screen.to_string();
    assert!(
        words(&text).contains("risk_adjusted = return_1y / volatility"),
        "{text}"
    );
    assert!(words(&screen.report.to_html(None)).contains("per_pe = max(risk_adjusted, 0) / pe"));
    assert!(screen.report.tables().iter().any(|t| t.name == "formulas"));
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
}

#[test]
fn every_problem_with_the_formulas_is_reported_together() {
    let u = universe();
    let Err(finalytics::Error::Validation(report)) = u
        .screen()
        .formula("a", "retrun_1y / 2")
        .formula("b", "c + 1")
        .formula("c", "sector * 2")
        .formula("volatility", "1")
        .formula("d", "return_1y > 1")
        .build()
    else {
        panic!("screened with bad formulas")
    };
    let found: Vec<(String, String)> = report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect();
    let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "formulas.a",
            "formulas.b",
            "formulas.c",
            "formulas.volatility",
            "formulas.d"
        ],
        "{found:?}"
    );
    assert!(
        found[0]
            .1
            .contains("`retrun_1y` at character 1: unknown name; did you mean return_1y?")
    );
    assert!(found[1].1.contains("given after this one"));
    assert!(found[2].1.contains("sector is text"));
    assert!(found[4].1.contains("`>` at character 11"));
}
