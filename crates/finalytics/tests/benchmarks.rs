//! A benchmark in its own currency, converted to the base currency, and blended benchmarks, from
//! which the attribution report derives the benchmark's segments.

#![allow(clippy::unwrap_used)]

use finalytics::{Portfolio, PortfolioCsv, datasets};

fn file(name: &str) -> &'static str {
    datasets::file(name).unwrap()
}

/// The sample portfolio in `base`, from its files.
fn sample(base: &str, instruments: &str) -> Portfolio {
    Portfolio::from_csv(
        &PortfolioCsv::new(file("portfolio/transactions.csv"))
            .instruments(instruments)
            .accounts(file("portfolio/accounts.csv"))
            .prices(file("portfolio/prices.csv"))
            .fx(file("portfolio/fx.csv"))
            .base_currency(base),
    )
    .unwrap()
}

fn column(name: &str) -> Vec<(String, f64)> {
    let text = file(name);
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let k = header.len() - 1;
    lines
        .map(|l| {
            let cells: Vec<&str> = l.split(',').collect();
            (cells[0].to_owned(), cells[k].parse().unwrap())
        })
        .collect()
}

#[test]
fn a_benchmark_is_converted_to_the_base_currency() {
    let instruments = file("portfolio/instruments.csv");
    let eur = sample("EUR", instruments);
    let report = eur.report().benchmark("BENCH").build().unwrap();
    let growth = report.overview.benchmark_growth.as_ref().unwrap();
    // BENCH in dollars, at the EUR/USD rate of each date.
    let bench: Vec<(String, f64)> = column("portfolio/prices.csv");
    let rates: Vec<(String, f64)> = column("portfolio/fx.csv");
    let euros = |k: usize| bench[k].1 / rates[k].1;
    for k in [1, 20, 200] {
        let want = euros(k) / euros(0) * 100.0;
        assert!(
            (growth.values[k] - want).abs() < 1e-9,
            "{k}: {} {want}",
            growth.values[k]
        );
    }
    let p = &report.overview.provenance;
    assert_eq!(
        p.benchmark.as_deref(),
        Some("BENCH, in USD, converted to EUR at the market data's rates")
    );
    let text = report.to_text(None);
    assert!(
        text.contains("Benchmark: BENCH, in USD, converted to EUR"),
        "{text}"
    );
    assert!(
        report
            .to_html(None)
            .contains("Benchmark: BENCH, in USD, converted to EUR")
    );
    // In dollars, nothing is converted.
    let usd = sample("USD", instruments)
        .report()
        .benchmark("BENCH")
        .build()
        .unwrap();
    let levels = usd.overview.benchmark_growth.unwrap();
    assert!((levels.values[20] - bench[20].1 / bench[0].1 * 100.0).abs() < 1e-9);
    assert_eq!(
        usd.overview.provenance.benchmark.as_deref(),
        Some("BENCH, in USD")
    );
}

#[test]
fn the_reports_say_when_a_benchmark_cannot_be_converted_or_its_currency_is_not_listed() {
    let instruments = file("portfolio/instruments.csv");
    // BENCH listed in pounds, for which the data has no rate: compared as it is, and said so.
    let pounds = instruments.replace("BENCH,other,USD", "BENCH,other,GBP");
    let report = sample("USD", &pounds)
        .report()
        .benchmark("BENCH")
        .build()
        .unwrap();
    assert_eq!(
        report.overview.provenance.benchmark.as_deref(),
        Some(
            "BENCH, in GBP: the market data has no GBP to USD rate, so its levels are compared \
             unconverted"
        )
    );
    assert!(report.overview.benchmark_growth.is_some());
    // Not listed: taken to be in the base currency, and said so.
    let unlisted: String = instruments
        .lines()
        .filter(|l| !l.starts_with("BENCH"))
        .map(|l| format!("{l}\n"))
        .collect();
    let report = sample("EUR", &unlisted)
        .report()
        .benchmark("BENCH")
        .build()
        .unwrap();
    assert_eq!(
        report.overview.provenance.benchmark.as_deref(),
        Some("BENCH, taken to be in EUR, the base currency, as the instruments do not list it")
    );
}

#[test]
fn a_blended_benchmark_is_rebalanced_monthly_and_gives_the_attribution_segments() {
    let portfolio = datasets::portfolio();
    let weights = [("VTI", 0.55), ("BND", 0.35), ("GLD", 0.1)];
    let report = portfolio
        .report()
        .benchmark_blend(weights)
        .attribution_by("asset_class")
        .group_by(["asset_class"])
        .build()
        .unwrap();
    let growth = report.overview.benchmark_growth.as_ref().unwrap();
    // Within the first month, the blend holds the units bought at its start.
    let prices = finalytics::formats::price_columns_from_csv(
        file("portfolio/prices.csv"),
        finalytics::formats::CsvOptions::default(),
    )
    .unwrap();
    let price = |id: &str, k: usize| -> f64 {
        let s = &prices.iter().find(|(i, _)| i.as_str() == id).unwrap().1;
        f64::try_from(s.values()[k]).unwrap()
    };
    for k in [1, 5, 15] {
        let want: f64 = weights
            .iter()
            .map(|(id, w)| 100.0 * w * price(id, k) / price(id, 0))
            .sum();
        assert!((growth.values[k] - want).abs() < 1e-9, "{k}");
    }
    assert_eq!(growth.name, "Benchmark");
    assert_eq!(
        report.overview.provenance.benchmark.as_deref(),
        Some("a blend of 55% VTI, 35% BND and 10% GLD, rebalanced monthly")
    );
    // The attribution report derives a period for each month, by asset class.
    let attribution = report.attribution.as_ref().unwrap();
    assert_eq!(attribution.periods.len(), 36);
    let first = &attribution.periods[0];
    let weight = |name: &str| {
        first
            .segments
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.benchmark_weight)
            .unwrap()
    };
    assert!((weight("Equity") - 0.55).abs() < 1e-12);
    assert!((weight("Fixed income") - 0.35).abs() < 1e-12);
    assert!((weight("Real assets") - 0.1).abs() < 1e-12);
    // The segments' returns add up to the blend's return over each month.
    for (k, period) in attribution.periods.iter().enumerate().take(6) {
        let blend: f64 = period
            .segments
            .iter()
            .map(|s| s.benchmark_weight * s.benchmark_return)
            .sum();
        let start = growth
            .dates
            .iter()
            .position(|d| *d == period.start)
            .unwrap();
        let end = growth.dates.iter().position(|d| *d == period.end).unwrap();
        let levels = growth.values[end] / growth.values[start] - 1.0;
        assert!((blend - levels).abs() < 1e-12, "{k}: {blend} {levels}");
    }
}

#[test]
fn mistakes_with_a_blended_benchmark_are_reported() {
    let portfolio = datasets::portfolio();
    let Err(finalytics::Error::Validation(found)) = portfolio
        .report()
        .benchmark_blend([("VTI", 0.5), ("NOPE", 0.2)])
        .build()
    else {
        panic!("a bad blend must be refused")
    };
    let messages: Vec<(String, String)> = found
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect();
    assert_eq!(
        messages,
        [
            (
                "options.benchmark.NOPE".to_owned(),
                "NOPE has no prices in the market data".to_owned()
            ),
            (
                "options.benchmark".to_owned(),
                "the blend's weights add up to 0.7, not one".to_owned()
            ),
        ]
    );
    let Err(finalytics::Error::Validation(found)) = portfolio.attribution_report(&{
        let mut o = finalytics::reports::ReportOptions::default();
        o.benchmark = Some("BENCH".parse::<finalytics::InstrumentId>().unwrap().into());
        o.attribution = Some(finalytics::reports::AttributionInputs {
            scheme: "asset_class".to_owned(),
            periods: Vec::new(),
            linking: Default::default(),
        });
        o
    }) else {
        panic!("attribution without segments needs a blend")
    };
    assert_eq!(found.issues()[0].path, "options.attribution.periods");
}
