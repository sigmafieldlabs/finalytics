//! Plans through the front door: the same plan on any number of threads, the report's forms, and
//! the inputs checked together.

#![allow(clippy::unwrap_used)]

use finalytics::planning::{Frequency, Status};
use finalytics::{Assets, Error, Plan, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()
        .unwrap()
}

fn retirement(assets: &Assets) -> Plan {
    assets
        .plan()
        .payments(4_000.0, "2046-11", "2066-10")
        .name("Retirement income")
        .start_value(250_000.0)
        .saving(1_500.0, Frequency::Month)
        .weights([("US Equity", 0.6), ("Bonds", 0.4)])
        .alternative("Cautious", [("US Equity", 0.3), ("Bonds", 0.7)])
        .inflation(0.025)
        .fee(0.0025)
        .simulations(1_000)
        .seed(3)
        .build()
        .unwrap()
}

#[test]
fn a_seed_gives_the_same_plan_on_any_number_of_threads() {
    let assets = assets();
    let many = retirement(&assets);
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| retirement(&assets));
    assert_eq!(one.results, many.results);
    assert_eq!(one.report, many.report);
}

#[test]
fn the_report_prints_shows_and_exports() {
    let plan = retirement(&assets());
    let text = plan.to_string();
    assert!(text.contains("FINALYTICS  ·  PLANNING"));
    assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
    let page = plan.report.to_html(None);
    assert!(page.starts_with("<!doctype html>"));
    assert!(page.contains("Retirement income"));
    // The report written as JSON reads back as the same report, in every form.
    let json = serde_json::to_string(&plan.report).unwrap();
    assert_eq!(
        finalytics::reports::report_text(&json, None).unwrap(),
        plan.report.to_text(None)
    );
    assert_eq!(finalytics::reports::report_html(&json, None).unwrap(), page);
    let tables = finalytics::reports::report_tables(&json).unwrap();
    assert_eq!(tables, plan.report.tables());
    let results = serde_json::to_string(&plan.results).unwrap();
    assert!(
        finalytics::results_text(&results)
            .unwrap()
            .starts_with("PlanResults")
    );
}

#[test]
fn a_status_follows_the_chance() {
    let plan = retirement(&assets());
    let r = &plan.results;
    let chance = r.outcome.chance.unwrap();
    assert_eq!(
        r.status,
        Some(Status::of(chance, r.chance_wanted, r.spare_from))
    );
    assert_eq!(r.alternatives.len(), 1);
}

#[test]
fn inputs_are_checked_together() {
    let err = assets()
        .plan()
        .target(-1.0, "2030-06")
        .weights([("Nope", 1.0)])
        .fee(2.0)
        .build()
        .unwrap_err();
    let Error::Validation(report) = err else {
        panic!("not a validation error")
    };
    let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
    for path in ["inflation", "fee", "goal.amount"] {
        assert!(paths.contains(&path), "{paths:?}");
    }
    assert!(paths.iter().any(|p| p.starts_with("weights")), "{paths:?}");
}
