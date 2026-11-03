//! The reports printed as text and laid out as an HTML page.

#![allow(clippy::unwrap_used)]

use finalytics::datasets;

fn report() -> finalytics::PortfolioReport {
    datasets::portfolio()
        .report_with(&datasets::report_options())
        .unwrap()
}

#[test]
fn the_text_fits_eighty_columns() {
    let report = report();
    let text = report.to_text(Some("Morgan family portfolio"));
    for line in text.lines() {
        assert!(line.chars().count() <= 80, "{line}");
        assert_eq!(line, line.trim_end());
    }
    for title in [
        "PORTFOLIO OVERVIEW",
        "HOLDINGS",
        "PERFORMANCE",
        "RISK",
        "ATTRIBUTION",
        "SCENARIOS",
    ] {
        assert!(text.contains(&format!("FINALYTICS  ·  {title}")), "{title}");
    }
    assert_eq!(text.matches("Morgan family portfolio").count(), 6);
    assert!(text.contains("Fees and taxes"));
    assert!(text.contains("Brokerage account"));
    // Each report prints on its own, and the whole prints without a name.
    assert!(report.risk.to_string().starts_with('━'));
    assert_eq!(report.to_string(), report.to_text(None));
}

#[test]
fn the_figures_in_the_text_are_the_reports() {
    let report = report();
    let text = report.overview.to_string();
    let value = report.overview.headline.value.round() as i64;
    let grouped = value
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        text.contains(&format!("Value (USD)             {grouped}")),
        "{text}"
    );
}

#[test]
fn the_html_page_is_self_contained() {
    let report = report();
    let html = report.to_html(Some("Morgan & family <portfolio>"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Morgan &amp; family &lt;portfolio&gt;</title>"));
    for id in [
        "overview",
        "holdings",
        "performance",
        "risk",
        "attribution",
        "scenarios",
    ] {
        assert!(html.contains(&format!(r#"<section id="{id}">"#)), "{id}");
    }
    assert!(html.contains("data:image/png;base64,iVBORw0KGgo"));
    assert!(html.contains("prefers-color-scheme:dark"));
    assert!(html.contains("@media print"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("http://") && !html.contains("https://"));
    // No number failed to compute: Rust writes those as NaN and inf.
    assert!(!html.contains("NaN"));
    let infinite = html.match_indices("inf").any(|(i, _)| {
        !html[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic())
            && !html[..i].ends_with(|c: char| c.is_ascii_alphabetic())
    });
    assert!(!infinite);
    let untitled = report.to_html(None);
    assert!(untitled.contains("<title>Portfolio report</title>"));
}

#[test]
fn small_portfolios_print_too() {
    use finalytics::reports::ReportOptions;
    use finalytics::{Portfolio, PortfolioCsv};
    let cases = [
        // Cash only.
        (
            "date,type,amount,currency\n2026-01-02,deposit,1000,USD\n",
            "date,X\n2026-01-02,1\n2026-01-05,1\n2026-01-06,1\n",
        ),
        // One holding over three dates, without a benchmark.
        (
            "date,type,instrument,quantity,price,amount,currency\n\
             2026-01-02,deposit,,,,1000,USD\n2026-01-02,buy,X,5,100,,\n",
            "date,X\n2026-01-02,100\n2026-01-05,101\n2026-01-06,99\n",
        ),
    ];
    for (transactions, prices) in cases {
        let portfolio =
            Portfolio::from_csv(&PortfolioCsv::new(transactions).prices(prices)).unwrap();
        let report = portfolio.report_with(&ReportOptions::default()).unwrap();
        let text = report.to_text(None);
        assert!(text.lines().all(|l| l.chars().count() <= 80), "{text}");
        let html = report.to_html(None);
        assert!(html.ends_with("</html>\n"));
        assert!(!html.contains("NaN"));
    }
}
