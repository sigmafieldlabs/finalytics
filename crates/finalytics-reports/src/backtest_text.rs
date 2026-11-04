//! The optimisation and backtest reports as text for printing: 80 columns, in sections.

use std::collections::BTreeMap;
use std::fmt;

use finalytics_backtest::Validation;

use crate::backtest_html::{
    allocation_heading, and_list, capitalise, costs, failure_notes, frequency, grouped_notes,
    month, rebalancing, shown_assets, span, straddles, table_name, tested_label, validation,
    weights_heading,
};
use crate::backtests::{
    BacktestContext, BacktestReport, ComparisonReport, RunReport, StrategyReport,
};
use crate::text::{
    L, Page, R, WIDTH, bar, fit, num, pad_left, pad_right, pct, period_word, plural,
    shown_difference, spark, width, wrap,
};

fn percent(v: f64) -> String {
    pct(Some(v), 1, false)
}

fn weight(v: f64) -> String {
    crate::text::weight_pct(v, 0)
}

/// The heading of a report: its kind and the periods scored, its title, and how it was run.
fn header(page: &mut Page, kind: &str, title: Option<&str>, lines: &[String], c: &BacktestContext) {
    page.rule('━');
    let left = format!("FINALYTICS  ·  {kind}");
    let right = span(c.test_start, c.test_end);
    page.line(format!(
        "{left}{}",
        pad_left(&right, WIDTH.saturating_sub(width(&left)))
    ));
    if let Some(title) = title {
        page.line(title);
    }
    for line in lines {
        page.parts(line);
    }
    page.rule('━');
}

fn footer(page: &mut Page, c: &BacktestContext) {
    page.blank();
    page.line(format!(
        "  Finalytics {} · inputs {}",
        c.version,
        c.digest.chars().take(12).collect::<String>()
    ));
}

/// Label and text rows, the text wrapped beside its label.
fn notes(page: &mut Page, rows: &[(&str, String)]) {
    let lw = rows.iter().map(|(l, _)| width(l)).max().unwrap_or(0);
    for (label, text) in rows {
        for (k, line) in wrap(text, WIDTH - 4 - lw).into_iter().enumerate() {
            let label = if k == 0 { *label } else { "" };
            page.line(format!("  {}  {line}", pad_right(label, lw)));
        }
    }
}

/// The method notes, as in the HTML page, shortened: for methods fitted to returns (`fitted`),
/// fixed weights (`fixed`), or both.
fn method(page: &mut Page, c: &BacktestContext, fitted: bool, fixed: bool) {
    let period = period_word(c.periods_per_year);
    let assets = if c.assets.len() <= 8 {
        and_list(&c.assets)
    } else {
        plural(c.assets.len(), "asset")
    };
    let mut rows = vec![(
        "Assets",
        format!(
            "{assets}{}. {} returns, {}.",
            c.risk_free_asset
                .as_ref()
                .map_or_else(String::new, |a| format!("; {a} as cash")),
            capitalise(frequency(c.periods_per_year)),
            span(c.start, c.end)
        ),
    )];
    let mut validation = match &c.settings.validation {
        _ if !fitted => format!(
            "Fixed weights, not fitted to these returns, scored on {}.",
            span(c.test_start, c.test_end)
        ),
        Validation::InSample => {
            "In sample: fitted to the whole history and scored on it.".to_owned()
        }
        Validation::Holdout { .. } => format!(
            "Holdout: fitted to the history to {}, scored on {}.",
            month(c.fit_end.unwrap_or(c.start)),
            span(c.test_start, c.test_end)
        ),
        Validation::WalkForward { expanding, .. } => format!(
            "Walk-forward: refitted at each rebalance on {}, scored on {}.",
            match (expanding, c.window) {
                (true, Some(w)) => format!("all the history so far (first {})", plural(w, period)),
                (false, Some(w)) => format!("the previous {}", plural(w, period)),
                _ => "the history before it".to_owned(),
            },
            span(c.test_start, c.test_end)
        ),
        _ => "As set.".to_owned(),
    };
    if fitted && fixed {
        validation.push_str(" Fixed weights, not fitted, are scored on the same periods.");
    }
    rows.push(("Validation", validation));
    if fitted {
        rows.push((
            "Limits",
            if c.limits.is_empty() {
                "Fully invested and long only.".to_owned()
            } else {
                format!(
                    "{}; fully invested.",
                    capitalise(&crate::backtest_html::limits_sentence(c))
                )
            },
        ));
    }
    rows.push((
        "Trading",
        format!(
            "{}, {}.",
            capitalise(&rebalancing(&c.settings.rebalance)),
            costs(c.settings.trading_cost_bps)
        ),
    ));
    rows.push((
        "Ratios",
        format!(
            "Against the {} return, {} a year over the test; value at risk and expected shortfall \
             from history, at 95% over one {period}.",
            c.risk_free_asset.as_deref().unwrap_or("risk-free"),
            percent(c.risk_free_rate)
        ),
    ));
    notes(page, &rows);
}

/// One sparkline per run, labelled, with its last value.
fn sparks(page: &mut Page, runs: &[(&str, &crate::Series)], format: &dyn Fn(f64) -> String) {
    let lw = runs
        .iter()
        .map(|(n, _)| width(n).min(24))
        .max()
        .unwrap_or(0);
    let values: Vec<String> = runs
        .iter()
        .map(|(_, s)| s.values.last().map_or_else(String::new, |v| format(*v)))
        .collect();
    let vw = values.iter().map(|v| width(v)).max().unwrap_or(0);
    let cells = WIDTH.saturating_sub(2 + lw + 2 + 2 + vw).clamp(10, 50);
    for ((name, series), value) in runs.iter().zip(&values) {
        page.line(format!(
            "  {}  {}  {}",
            pad_right(&fit(name, 24), lw),
            spark(&series.values, cells),
            pad_left(value, vw)
        ));
    }
}

/// A table of weights: strategies down the side and assets across when there are few assets,
/// and the other way round otherwise.
fn weights_table(page: &mut Page, c: &BacktestContext, latest: &[(&str, &BTreeMap<String, f64>)]) {
    let (assets, rest) = shown_assets(c, &latest.iter().map(|(_, w)| *w).collect::<Vec<_>>());
    let mut columns: Vec<String> = assets.clone();
    if !rest.is_empty() {
        columns.push(format!("{} others", rest.len()));
    }
    let value = |w: &BTreeMap<String, f64>, k: usize| -> f64 {
        match assets.get(k) {
            Some(a) => w.get(a).copied().unwrap_or(0.0),
            None => rest.iter().map(|a| w.get(a).copied().unwrap_or(0.0)).sum(),
        }
    };
    if columns.len() <= 6 {
        let head: Vec<&str> = std::iter::once("")
            .chain(columns.iter().map(String::as_str))
            .collect();
        let rows: Vec<Vec<String>> = latest
            .iter()
            .map(|(name, w)| {
                std::iter::once((*name).to_owned())
                    .chain((0..columns.len()).map(|k| weight(value(w, k))))
                    .collect()
            })
            .collect();
        page.table(&head, &rows, None);
    } else {
        let head: Vec<&str> = std::iter::once("")
            .chain(latest.iter().map(|(n, _)| *n))
            .collect();
        let rows: Vec<Vec<String>> = columns
            .iter()
            .enumerate()
            .map(|(k, a)| {
                std::iter::once(a.clone())
                    .chain(latest.iter().map(|(_, w)| weight(value(w, k))))
                    .collect()
            })
            .collect();
        page.table(&head, &rows, None);
    }
}

fn statistics_row(
    label: &str,
    runs: &[&RunReport],
    f: &dyn Fn(&RunReport) -> String,
) -> Vec<String> {
    std::iter::once(label.to_owned())
        .chain(runs.iter().map(|r| f(r)))
        .collect()
}

impl ComparisonReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own) if given.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let name = name.or(self.name.as_deref());
        let c = &self.context;
        let mut page = Page::new(None);
        let tested = tested_label(c);
        let n = self.strategies.len();
        header(
            &mut page,
            "COMPARISON",
            name,
            &[
                format!(
                    "{} · {} · {}",
                    validation(c),
                    rebalancing(&c.settings.rebalance),
                    costs(c.settings.trading_cost_bps)
                ),
                format!(
                    "{} · {} · {}{}",
                    plural(n, "method"),
                    plural(c.assets.len(), "asset"),
                    frequency(c.periods_per_year),
                    c.risk_free_asset
                        .as_ref()
                        .map_or_else(String::new, |a| format!(" · {a} as cash"))
                ),
            ],
            c,
        );
        let mut entries: Vec<&StrategyReport> = self.strategies.iter().collect();
        if let Some(b) = &self.benchmark {
            entries.push(b);
        }
        let strategies: Vec<&StrategyReport> = self.strategies.iter().collect();
        page.section(&format!("{tested}, {}", span(c.test_start, c.test_end)));
        let rows: Vec<Vec<String>> = entries
            .iter()
            .map(|s| {
                let r = &s.tested;
                vec![
                    table_name(s),
                    percent(r.statistics.annualised_return),
                    percent(r.statistics.volatility),
                    num(r.statistics.sharpe_ratio, 2),
                    percent(r.statistics.max_drawdown),
                    percent(r.tail_risk.expected_shortfall),
                    pct(Some(r.turnover_per_year), 0, false),
                ]
            })
            .collect();
        page.table(
            &[
                "Method",
                "Return",
                "Volatility",
                "Sharpe",
                "Drawdown",
                "CVaR 95%",
                "Turnover",
            ],
            &rows,
            None,
        );
        page.blank();
        let mut note = String::from(
            "Return, volatility and turnover a year, turnover one-way (the share of value bought, \
             or sold); drawdown the largest fall; CVaR the average loss in the worst 5% of \
             periods.",
        );
        for line in grouped_notes(&strategies)
            .into_iter()
            .chain(failure_notes(c, &strategies))
        {
            note.push(' ');
            note.push_str(&line);
        }
        page.note(&note);
        // In sample against out of sample, for the methods fitted to returns.
        if strategies.iter().any(|s| s.fitted)
            && strategies
                .iter()
                .filter(|s| s.fitted)
                .all(|s| s.in_sample.is_some())
        {
            page.section("In sample and out of sample");
            let rows: Vec<Vec<String>> = strategies
                .iter()
                .filter_map(|s| {
                    let i = s.in_sample.as_ref()?;
                    let o = &s.tested;
                    Some(vec![
                        table_name(s),
                        num(i.statistics.sharpe_ratio, 2),
                        num(o.statistics.sharpe_ratio, 2),
                        match (i.statistics.sharpe_ratio, o.statistics.sharpe_ratio) {
                            (Some(a), Some(b)) => num(Some(shown_difference(a, b, 2)), 2),
                            _ => "–".to_owned(),
                        },
                        percent(i.statistics.annualised_return),
                        percent(o.statistics.annualised_return),
                    ])
                })
                .collect();
            page.table(
                &[
                    "Method",
                    "Sharpe in",
                    "Sharpe out",
                    "Change",
                    "Return in",
                    "Return out",
                ],
                &rows,
                None,
            );
            page.note(&crate::backtest_html::in_and_out(c));
        }
        page.section(&format!("Growth of 100, {}", tested.to_lowercase()));
        let growth: Vec<(&str, &crate::Series)> = entries
            .iter()
            .map(|s| (s.name.as_str(), &s.tested.growth))
            .collect();
        sparks(&mut page, &growth, &|v| num(Some(v), 1));
        if let Some(first) = entries.first() {
            page.section("Calendar-year returns");
            let head: Vec<&str> = std::iter::once("Year")
                .chain(entries.iter().map(|s| s.name.as_str()))
                .collect();
            let rows: Vec<Vec<String>> = first
                .tested
                .years
                .iter()
                .map(|y| {
                    let label = if y.complete {
                        y.year.to_string()
                    } else {
                        format!("{}*", y.year)
                    };
                    std::iter::once(label)
                        .chain(entries.iter().map(|s| {
                            pct(
                                s.tested
                                    .years
                                    .iter()
                                    .find(|z| z.year == y.year)
                                    .map(|z| z.value),
                                1,
                                false,
                            )
                        }))
                        .collect()
                })
                .collect();
            page.table(&head, &rows, None);
            if first.tested.years.iter().any(|y| !y.complete) {
                page.blank();
                page.note("* Part of the year.");
            }
        }
        let latest: Vec<(&str, &BTreeMap<String, f64>)> = strategies
            .iter()
            .filter_map(|s| s.latest().map(|w| (s.name.as_str(), &w.weights)))
            .collect();
        if !latest.is_empty() {
            page.section(&weights_heading(c, &strategies));
            weights_table(&mut page, c, &latest);
        }
        page.section("How the methods were tested");
        method(
            &mut page,
            c,
            strategies.iter().any(|s| s.fitted),
            strategies.iter().any(|s| !s.fitted),
        );
        footer(&mut page, c);
        page.finish()
    }
}

impl fmt::Display for ComparisonReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}

impl BacktestReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own, or the strategy's).
    pub fn to_text(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let s = &self.strategy;
        let title = name.or(self.name.as_deref()).unwrap_or(&s.name);
        let tested = tested_label(c);
        let period = period_word(c.periods_per_year);
        let mut page = Page::new(None);
        let mut meta = vec![format!(
            "{} · {} · {}",
            validation(c),
            rebalancing(&c.settings.rebalance),
            costs(c.settings.trading_cost_bps)
        )];
        let mut second = vec![frequency(c.periods_per_year).to_owned()];
        if let Some(a) = &c.risk_free_asset {
            second.push(format!("{a} as cash"));
        }
        if let Some(b) = &self.benchmark {
            second.push(format!("against {}", b.name));
        }
        meta.push(capitalise(&second.join(" · ")));
        header(&mut page, "BACKTEST", Some(title), &meta, c);
        if let Some(w) = s.latest() {
            page.section(&allocation_heading(s));
            let mut parts: Vec<(String, f64)> = w
                .weights
                .iter()
                .filter(|(_, v)| **v >= 0.005)
                .map(|(a, v)| (a.clone(), *v))
                .collect();
            parts.sort_by(|a, b| b.1.total_cmp(&a.1));
            let cash = 1.0 - w.weights.values().sum::<f64>();
            if cash >= 0.005 {
                parts.push((
                    c.risk_free_asset
                        .clone()
                        .unwrap_or_else(|| "Cash".to_owned()),
                    cash,
                ));
            }
            let lw = parts
                .iter()
                .map(|(a, _)| width(a).min(18))
                .max()
                .unwrap_or(0);
            for (asset, v) in parts.iter().take(12) {
                page.line(format!(
                    "  {}  {}  {}",
                    pad_right(&fit(asset, 18), lw),
                    bar(*v, 30),
                    pad_left(&percent(*v), 6)
                ));
            }
            if parts.len() > 12 {
                let others: f64 = parts.iter().skip(12).map(|(_, v)| v).sum();
                page.line(format!(
                    "  {}  {}  {}",
                    pad_right(&format!("{} others", parts.len() - 12), lw),
                    bar(others, 30),
                    pad_left(&percent(others), 6)
                ));
            }
        }
        page.section("Statistics");
        let mut heads: Vec<String> = vec![String::new()];
        let mut runs: Vec<&RunReport> = Vec::new();
        if let Some(i) = &s.in_sample {
            heads.push("In sample".to_owned());
            runs.push(i);
            heads.push(tested.to_owned());
        } else {
            heads.push(fit(&s.name, 20));
        }
        runs.push(&s.tested);
        if let Some(b) = &self.benchmark {
            // The benchmark in sample only beside the strategy's.
            if let Some(i) = b.in_sample.as_ref().filter(|_| s.in_sample.is_some()) {
                heads.push(format!("{} in", fit(&b.name, 14)));
                runs.push(i);
                heads.push(format!("{} out", fit(&b.name, 14)));
            } else {
                heads.push(fit(&b.name, 20));
            }
            runs.push(&b.tested);
        }
        let mut rows = vec![
            statistics_row("Return a year", &runs, &|r| {
                percent(r.statistics.annualised_return)
            }),
            statistics_row("Volatility", &runs, &|r| percent(r.statistics.volatility)),
            statistics_row("Sharpe ratio", &runs, &|r| {
                num(r.statistics.sharpe_ratio, 2)
            }),
            statistics_row("Sortino ratio", &runs, &|r| {
                num(r.statistics.sortino_ratio, 2)
            }),
            statistics_row("Maximum drawdown", &runs, &|r| {
                percent(r.statistics.max_drawdown)
            }),
            statistics_row("CDaR 95%", &runs, &|r| {
                percent(r.statistics.conditional_drawdown_at_risk)
            }),
            statistics_row("Ulcer index", &runs, &|r| percent(r.statistics.ulcer_index)),
            statistics_row(&format!("Worst {period}"), &runs, &|r| {
                pct(Some(r.statistics.worst), 1, true)
            }),
            statistics_row(&format!("VaR 95%, 1 {period}"), &runs, &|r| {
                percent(r.tail_risk.value_at_risk)
            }),
        ];
        if s.tested.relative.is_some() {
            rows.push(statistics_row("Tracking error", &runs, &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| percent(x.tracking_error))
            }));
            rows.push(statistics_row("Information ratio", &runs, &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| num(x.information_ratio, 2))
            }));
            rows.push(statistics_row("Downside capture", &runs, &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| num(x.downside_capture, 2))
            }));
            rows.push(statistics_row("Upside capture", &runs, &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| num(x.upside_capture, 2))
            }));
        }
        let head: Vec<&str> = heads.iter().map(String::as_str).collect();
        page.table(&head, &rows, None);
        let o = &s.tested;
        page.section(&format!("Trading, {}", tested.to_lowercase()));
        if o.rebalances == 0 {
            page.note(
                "Bought at the start and held: no trades after it, so no turnover or trading \
                 costs.",
            );
        } else {
            page.pairs(&[
                ("Rebalances".to_owned(), o.rebalances.to_string()),
                (
                    "Turnover a year".to_owned(),
                    pct(Some(o.turnover_per_year), 1, false),
                ),
                (
                    "Costs a year".to_owned(),
                    pct(Some(o.costs_per_year), 2, false),
                ),
                (
                    "Largest rebalance".to_owned(),
                    pct(Some(o.largest_trade), 1, false),
                ),
            ]);
            page.note(
            "Turnover is one-way: the share of the portfolio's value bought (or, equally, sold). \
             Trading costs are charged on the value traded both ways.",
        );
        }
        let (main, base) = match &self.history {
            Some(h) => (&h.strategy, h.benchmark.as_ref()),
            None => (o, self.benchmark.as_ref().map(|b| &b.tested)),
        };
        page.section(match &self.history {
            Some(_) => "Growth of 100, in and out of sample",
            None => "Growth of 100",
        });
        let mut growth: Vec<(&str, &crate::Series)> = vec![(s.name.as_str(), &main.growth)];
        if let (Some(b), Some(bench)) = (base, &self.benchmark) {
            growth.push((bench.name.as_str(), &b.growth));
        }
        sparks(&mut page, &growth, &|v| num(Some(v), 1));
        page.section("Calendar-year returns");
        let bench_years = base.map(|b| &b.years);
        let mut head = vec!["Year", s.name.as_str()];
        if let Some(b) = &self.benchmark {
            head.push(b.name.as_str());
            head.push("Difference");
        }
        let split = self.history.as_ref().map(|h| h.split);
        let rows: Vec<Vec<String>> = main
            .years
            .iter()
            .map(|y| {
                let out = split.is_some_and(|d| y.end >= d);
                let mut label = y.year.to_string();
                if !y.complete {
                    label.push('*');
                }
                if split.is_some_and(|d| straddles(y, d)) {
                    label.push('†');
                } else if out {
                    label.push_str(" out");
                }
                let mut row = vec![label, pct(Some(y.value), 1, true)];
                if self.benchmark.is_some() {
                    let b = bench_years
                        .and_then(|ys| ys.iter().find(|z| z.year == y.year))
                        .map(|z| z.value);
                    row.push(pct(b, 1, true));
                    row.push(pct(b.map(|b| shown_difference(b, y.value, 3)), 1, true));
                }
                row
            })
            .collect();
        let aligns = [L, R, R, R];
        page.table(&head, &rows, Some(&aligns));
        let mut note = String::new();
        if main.years.iter().any(|y| !y.complete) {
            note.push_str("* Part of the year. ");
        }
        if let Some(d) = split {
            note.push_str(&format!("Out of sample from {}.", month(d)));
            if let Some(y) = main.years.iter().find(|y| straddles(y, d)) {
                note.push_str(&format!(
                    " † {} is in sample to {} and out of sample after.",
                    y.year,
                    month(c.fit_end.unwrap_or(y.start))
                ));
            }
        }
        if !note.is_empty() {
            page.blank();
            page.note(note.trim_end());
        }
        let lines: Vec<String> = grouped_notes(&[s])
            .into_iter()
            .chain(failure_notes(c, &[s]))
            .collect();
        if !lines.is_empty() {
            page.blank();
            page.note(&lines.join(" "));
        }
        page.section("How the allocation was tested");
        method(&mut page, c, s.fitted, !s.fitted);
        footer(&mut page, c);
        page.finish()
    }
}

impl fmt::Display for BacktestReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}
