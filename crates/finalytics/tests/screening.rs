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
    assert!(text.contains("The top 50 of 56 matches; the page lists them all."));
    let html = screen.report.to_html(None);
    assert!(html.starts_with("<!doctype html>"));
    assert_eq!(html.matches(r#"<td class="rk">"#).count(), 56);
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
fn the_measures_leave_out_those_no_security_has() {
    // Two and a half years into the prices, no company has three years of closes.
    let screen = universe().screen().as_of("2007-06-29").build().unwrap();
    let r = &screen.results;
    let k = r
        .measures
        .iter()
        .position(|m| *m == Measure::Return3y)
        .unwrap();
    assert_eq!(r.coverage.lacking[k], 56);
    assert_eq!(r.coverage.lacking_history[k], 56);
    let spread = screen.report.spread.iter().find(|s| s.name == "return_3y");
    assert_eq!(spread.map(|s| (s.count, s.lacking_history)), Some((0, 56)));
    let text = screen.to_string();
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    assert!(
        text.contains("With no rules, all 56 securities pass."),
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
    assert_eq!(screen.report.exceptions.stale, ["T-Bills"]);
    let text = words(&screen.to_string());
    assert!(
        text.contains(
            "55 of 56 securities have at least two prices. 1 has no price in the 5 days to 30 \
             Oct 2026 and is left out: T-Bills."
        ),
        "{text}"
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
