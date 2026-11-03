//! The reports as text for printing: 80 columns, in sections, with tables, bars and sparklines.
//!
//! Each report implements [`Display`](std::fmt::Display), and so does [`PortfolioReport`], whose
//! text is its six reports one after another.

use std::collections::BTreeMap;
use std::fmt;

use finalytics_core::{Date, Decimal};
use finalytics_ledger::HoldingKey;
use finalytics_perf::Linking;
use finalytics_risk::{CovarianceMethod, TailMethod};

use crate::context::{Provenance, label};
use crate::scenarios::ScenarioDefinition;
use crate::shapes::{Breakdown, Series};
use crate::{
    AttributionReport, HoldingsReport, OverviewReport, PerformanceReport, PortfolioReport,
    RiskReport, ScenariosReport,
};

/// The width of every line.
pub(crate) const WIDTH: usize = 80;

/// The longest label shown in a table's first column.
const LABEL: usize = 24;

const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const EIGHTHS: [char; 7] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];

// ── Formatting ───────────────────────────────────────────────────────────────────────────────

fn minus(text: String) -> String {
    text.replace('-', "−")
}

/// `x` as a percentage with `places` decimals, signed when `signed`; "–" when absent.
pub(crate) fn pct(x: Option<f64>, places: usize, signed: bool) -> String {
    match x {
        Some(v) if v.is_finite() => {
            let v = round(v * 100.0, places);
            minus(if signed {
                format!("{v:+.places$}%")
            } else {
                format!("{v:.places$}%")
            })
        }
        _ => "–".to_owned(),
    }
}

/// Rounds to `places` decimals, turning negative zero into zero.
pub(crate) fn round(v: f64, places: usize) -> f64 {
    let scale = 10f64.powi(places as i32);
    (v * scale).round() / scale + 0.0
}

/// A signed percentage with one decimal.
pub(crate) fn signed(x: impl Into<Option<f64>>) -> String {
    pct(x.into(), 1, true)
}

/// An unsigned percentage with one decimal.
pub(crate) fn share(x: impl Into<Option<f64>>) -> String {
    pct(x.into(), 1, false)
}

/// An amount with thousands separators and `places` decimals.
pub(crate) fn money_places(x: f64, places: usize) -> String {
    if !x.is_finite() {
        return "–".to_owned();
    }
    let text = format!("{:.places$}", round(x, places));
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => ("−", rest),
        None => ("", text.as_str()),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    match fraction {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

pub(crate) fn money(x: f64) -> String {
    money_places(x, 0)
}

pub(crate) fn dec(x: Decimal) -> f64 {
    f64::try_from(x).unwrap_or(f64::NAN)
}

pub(crate) fn num(x: Option<f64>, places: usize) -> String {
    match x {
        Some(v) if v.is_finite() => minus(format!("{:.places$}", round(v, places))),
        _ => "–".to_owned(),
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The short name of month `m`, from 1 for January.
pub(crate) fn month_name(m: u32) -> &'static str {
    MONTHS[(m.clamp(1, 12) - 1) as usize]
}

pub(crate) fn date(d: Date) -> String {
    format!(
        "{} {} {}",
        d.day(),
        MONTHS[d.month() as usize - 1],
        d.year()
    )
}

pub(crate) fn optional_date(d: Option<Date>) -> String {
    d.map_or_else(|| "–".to_owned(), date)
}

/// A group's name as shown: "Cash" for cash.
pub(crate) fn group(name: &str) -> String {
    match name {
        "cash" => "Cash".to_owned(),
        "household" => "Household".to_owned(),
        other => other.to_owned(),
    }
}

/// A classification scheme's name as words: `asset_class` is "asset class".
pub(crate) fn words(scheme: &str) -> String {
    scheme.replace('_', " ")
}

pub(crate) fn holding(key: &HoldingKey) -> String {
    label(key)
}

pub(crate) fn width(text: &str) -> usize {
    text.chars().count()
}

/// `text` cut to `max` characters, ending in "…" when cut.
pub(crate) fn fit(text: &str, max: usize) -> String {
    if width(text) <= max {
        text.to_owned()
    } else {
        let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

pub(crate) fn pad_right(text: &str, w: usize) -> String {
    format!("{text}{}", " ".repeat(w.saturating_sub(width(text))))
}

pub(crate) fn pad_left(text: &str, w: usize) -> String {
    format!("{}{text}", " ".repeat(w.saturating_sub(width(text))))
}

/// A horizontal bar of `share` of `cells` characters, in eighths.
pub(crate) fn bar(share: f64, cells: usize) -> String {
    let filled = if share.is_finite() {
        share.clamp(0.0, 1.0) * cells as f64
    } else {
        0.0
    };
    let full = filled.floor() as usize;
    let mut out = "█".repeat(full);
    let rest = filled - full as f64;
    if rest > 1.0 / 14.0 && full < cells {
        out.push(EIGHTHS[((rest * 7.0) as usize).min(6)]);
    }
    pad_right(&out, cells)
}

/// A sparkline of `values` in `cells` characters.
pub(crate) fn spark(values: &[f64], cells: usize) -> String {
    let values: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if values.is_empty() {
        return " ".repeat(cells);
    }
    let step = values.len() as f64 / cells as f64;
    let points: Vec<f64> = (0..cells)
        .map(|i| values[((i as f64 * step) as usize).min(values.len() - 1)])
        .collect();
    let (lo, hi) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    points
        .iter()
        .map(|v| {
            if hi > lo {
                SPARK[(((v - lo) / (hi - lo)) * 7.0).round().clamp(0.0, 7.0) as usize]
            } else {
                SPARK[0]
            }
        })
        .collect()
}

/// What one valuation period is, from the number in a year.
pub(crate) fn period_word(periods_per_year: f64) -> &'static str {
    match periods_per_year.round() as i64 {
        252 | 250 | 260 | 365 => "day",
        52 => "week",
        12 => "month",
        4 => "quarter",
        1 => "year",
        _ => "period",
    }
}

pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub(crate) fn covariance_name(method: &CovarianceMethod) -> &'static str {
    match method {
        CovarianceMethod::Sample => "the sample covariance",
        CovarianceMethod::Ewma { .. } => "an exponentially weighted average",
        CovarianceMethod::LedoitWolf => "Ledoit–Wolf",
        CovarianceMethod::Oas => "oracle approximating shrinkage",
        _ => "the chosen estimator",
    }
}

pub(crate) fn tail_name(method: &TailMethod) -> &'static str {
    match method {
        TailMethod::Historical => "Historical",
        TailMethod::Parametric => "Parametric",
        TailMethod::CornishFisher => "Cornish–Fisher",
        _ => "Other",
    }
}

// ── Layout ───────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Align {
    Left,
    Right,
}

/// A page of text, line by line.
pub(crate) struct Page {
    lines: Vec<String>,
    name: Option<String>,
}

impl Page {
    pub(crate) fn new(name: Option<&str>) -> Page {
        Page {
            lines: Vec::new(),
            name: name.map(str::to_owned),
        }
    }

    pub(crate) fn finish(self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }

    pub(crate) fn line(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref().trim_end();
        self.lines.push(fit(text, WIDTH));
    }

    pub(crate) fn blank(&mut self) {
        self.lines.push(String::new());
    }

    pub(crate) fn rule(&mut self, c: char) {
        self.lines.push(c.to_string().repeat(WIDTH));
    }

    fn header(&mut self, title: &str, p: &Provenance) {
        if !self.lines.is_empty() {
            self.blank();
        }
        self.rule('━');
        let left = format!("FINALYTICS  ·  {}", title.to_uppercase());
        let right = format!("as of {}", date(p.as_of));
        self.line(format!(
            "{left}{}",
            pad_left(&right, WIDTH.saturating_sub(width(&left)))
        ));
        if let Some(name) = self.name.clone() {
            self.line(name);
        }
        let s = &p.source;
        self.line(format!(
            "{} · {} · {} · {} · from {}",
            plural(s.accounts, "account"),
            plural(s.instruments, "instrument"),
            plural(s.transactions, "transaction"),
            p.base_currency,
            date(p.start)
        ));
        self.rule('━');
    }

    pub(crate) fn section(&mut self, title: &str) {
        self.blank();
        self.line(title);
        self.line("─".repeat(width(title).min(WIDTH)));
    }

    pub(crate) fn note(&mut self, text: &str) {
        for line in wrap(text, WIDTH - 2) {
            self.line(format!("  {line}"));
        }
    }

    fn footer(&mut self, p: &Provenance) {
        let issues = p.warnings.issues();
        self.blank();
        self.line(format!(
            "  Finalytics {} · inputs {} · {}",
            p.version,
            p.source.digest.chars().take(12).collect::<String>(),
            plural(issues.len(), "warning")
        ));
        for issue in issues.iter().take(3) {
            self.line(format!("  ! {}", fit(&issue.message, WIDTH - 4)));
        }
        if issues.len() > 3 {
            self.line(format!("  ! and {} more", issues.len() - 3));
        }
    }

    /// Label and value pairs in two columns.
    pub(crate) fn pairs(&mut self, items: &[(String, String)]) {
        if items.is_empty() {
            return;
        }
        let half = items.len().div_ceil(2);
        let (left, right) = items.split_at(half);
        let lw = left.iter().map(|(k, _)| width(k)).max().unwrap_or(0);
        let lv = left.iter().map(|(_, v)| width(v)).max().unwrap_or(0);
        let rw = right.iter().map(|(k, _)| width(k)).max().unwrap_or(0);
        let rv = right.iter().map(|(_, v)| width(v)).max().unwrap_or(0);
        let two = 2 + lw + 2 + lv + 6 + rw + 2 + rv <= WIDTH;
        if !two {
            for (k, v) in items {
                self.line(format!("  {}  {v}", pad_right(k, lw.max(rw))));
            }
            return;
        }
        for (i, (k, v)) in left.iter().enumerate() {
            let mut text = format!("  {}  {}", pad_right(k, lw), pad_left(v, lv));
            if let Some((k2, v2)) = right.get(i) {
                text.push_str(&format!(
                    "      {}  {}",
                    pad_right(k2, rw),
                    pad_left(v2, rv)
                ));
            }
            self.line(text);
        }
    }

    /// A table: the first column is left-aligned, the others as `aligns` says (right by
    /// default). A table too wide for the page is split into several, each repeating the first
    /// column; long headings wrap onto two lines.
    pub(crate) fn table(
        &mut self,
        headings: &[&str],
        rows: &[Vec<String>],
        aligns: Option<&[Align]>,
    ) {
        self.table_spaced(headings, rows, aligns, 2);
    }

    fn table_spaced(
        &mut self,
        headings: &[&str],
        rows: &[Vec<String>],
        aligns: Option<&[Align]>,
        gap: usize,
    ) {
        let n = headings.len();
        let align = |i: usize| match aligns {
            Some(a) => a.get(i).copied().unwrap_or(Align::Right),
            None if i == 0 => Align::Left,
            None => Align::Right,
        };
        let rows: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                (0..n)
                    .map(|i| {
                        let cell = r.get(i).cloned().unwrap_or_default();
                        if i == 0 { fit(&cell, LABEL) } else { cell }
                    })
                    .collect()
            })
            .collect();
        let cells = |i: usize| rows.iter().map(|r| width(&r[i])).max().unwrap_or(0);
        // Headings on one line when the table fits; otherwise on two where that narrows a
        // column.
        let one_line: Vec<usize> = (0..n).map(|i| cells(i).max(width(headings[i]))).collect();
        let total = 2 + one_line.iter().sum::<usize>() + gap * n.saturating_sub(1);
        let mut heads: Vec<(String, Option<String>)> = Vec::with_capacity(n);
        let mut widths = Vec::with_capacity(n);
        for (i, h) in headings.iter().enumerate() {
            let c = cells(i);
            let (one, two) = split_heading(h);
            match two {
                Some(two) if total > WIDTH && width(h) > c => {
                    widths.push(width(&one).max(width(&two)).max(c));
                    heads.push((one, Some(two)));
                }
                _ => {
                    widths.push(one_line[i]);
                    heads.push((h.to_string(), None));
                }
            }
        }
        // Columns after the first, in groups that fit.
        let first = widths.first().copied().unwrap_or(0);
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let mut used = 2 + first;
        for (i, w) in widths.iter().enumerate().skip(1) {
            let need = gap + w;
            if !current.is_empty() && used + need > WIDTH {
                groups.push(std::mem::take(&mut current));
                used = 2 + first;
            }
            current.push(i);
            used += need;
        }
        if !current.is_empty() || groups.is_empty() {
            groups.push(current);
        }
        for (g, columns) in groups.iter().enumerate() {
            if g > 0 {
                self.blank();
            }
            let order: Vec<usize> = std::iter::once(0).chain(columns.iter().copied()).collect();
            let render = |cells: &dyn Fn(usize) -> String| {
                let parts: Vec<String> = order
                    .iter()
                    .map(|&i| match align(i) {
                        Align::Left => pad_right(&cells(i), widths[i]),
                        Align::Right => pad_left(&cells(i), widths[i]),
                    })
                    .collect();
                format!("  {}", parts.join(&" ".repeat(gap)))
            };
            if order.iter().any(|&i| heads[i].1.is_some()) {
                self.line(render(&|i| {
                    if heads[i].1.is_some() {
                        heads[i].0.clone()
                    } else {
                        String::new()
                    }
                }));
                self.line(render(&|i| match &heads[i].1 {
                    Some(second) => second.clone(),
                    None => heads[i].0.clone(),
                }));
            } else {
                self.line(render(&|i| heads[i].0.clone()));
            }
            for row in &rows {
                self.line(render(&|i| row[i].clone()));
            }
        }
    }

    /// One bar per slice of a breakdown, with its weight and value.
    fn bars(&mut self, breakdown: &Breakdown, names: &dyn Fn(&str) -> String) {
        let labels: Vec<String> = breakdown
            .slices
            .iter()
            .map(|s| fit(&names(&s.name), 18))
            .collect();
        let lw = labels.iter().map(|l| width(l)).max().unwrap_or(0).max(15);
        let values: Vec<String> = breakdown.slices.iter().map(|s| money(s.value)).collect();
        let vw = values.iter().map(|v| width(v)).max().unwrap_or(0);
        let cells = WIDTH
            .saturating_sub(2 + lw + 1 + 1 + 6 + 2 + vw)
            .clamp(10, 30);
        for ((s, l), v) in breakdown.slices.iter().zip(&labels).zip(&values) {
            self.line(format!(
                "  {} {} {}  {}",
                pad_right(l, lw),
                bar(s.weight, cells),
                pad_left(&share(s.weight), 6),
                pad_left(v, vw)
            ));
        }
    }

    /// A series as a sparkline, labelled, with its last value.
    fn sparkline(&mut self, name: &str, series: &Series, cells: usize, last: String) {
        self.line(format!(
            "  {} {}  {last}",
            pad_right(&fit(name, 12), 12),
            spark(&series.values, cells)
        ));
    }
}

/// A heading split at the space nearest its middle, if it has one.
fn split_heading(h: &str) -> (String, Option<String>) {
    let spaces: Vec<usize> = h
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .map(|(i, _)| i)
        .collect();
    // The split that makes the wider line narrowest; of equals, the later.
    let Some(&best) = spaces
        .iter()
        .rev()
        .min_by_key(|&&i| width(&h[..i]).max(width(&h[i + 1..])))
    else {
        return (h.to_owned(), None);
    };
    (h[..best].to_owned(), Some(h[best + 1..].to_owned()))
}

/// `text` in lines of at most `max` characters, broken at spaces.
pub(crate) fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && width(&current) + 1 + width(word) > max {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(crate) const R: Align = Align::Right;
pub(crate) const L: Align = Align::Left;

/// The account names of a holdings report, by account identifier.
pub(crate) fn account_names(h: &HoldingsReport) -> BTreeMap<String, String> {
    h.accounts
        .iter()
        .filter_map(|a| a.name.clone().map(|n| (a.account.to_string(), n)))
        .collect()
}

/// A correlation grid without the holdings that have no risk, and their names.
pub(crate) fn with_risk(grid: &crate::Grid) -> (Vec<String>, Vec<Vec<Option<f64>>>, Vec<String>) {
    let keep: Vec<usize> = (0..grid.rows.len())
        .filter(|&i| {
            grid.values
                .get(i)
                .is_some_and(|row| row.iter().enumerate().any(|(j, v)| j != i && v.is_some()))
        })
        .collect();
    let riskless = (0..grid.rows.len())
        .filter(|i| !keep.contains(i))
        .map(|i| grid.rows[i].clone())
        .collect();
    let rows = keep.iter().map(|&i| grid.rows[i].clone()).collect();
    let values = keep
        .iter()
        .map(|&i| {
            keep.iter()
                .map(|&j| grid.values[i].get(j).copied().flatten())
                .collect()
        })
        .collect();
    (rows, values, riskless)
}

// ── The reports ──────────────────────────────────────────────────────────────────────────────

pub(crate) fn overview(page: &mut Page, o: &OverviewReport) {
    let p = &o.provenance;
    let ccy = p.base_currency;
    let h = &o.headline;
    let period = period_word(p.options.periods_per_year);
    page.header("Portfolio overview", p);
    page.section("Key figures");
    let mut items = vec![
        (format!("Value ({ccy})"), money(h.value)),
        (
            format!("Return, last {period}"),
            pct(h.period_return, 2, true),
        ),
        ("Return, year to date".to_owned(), signed(h.year_to_date)),
        (
            "Return since inception".to_owned(),
            signed(h.since_inception),
        ),
        (
            "Annualised".to_owned(),
            signed(h.since_inception_annualised),
        ),
        ("Volatility".to_owned(), share(h.volatility)),
        ("Sharpe ratio".to_owned(), num(h.sharpe_ratio, 2)),
        ("Maximum drawdown".to_owned(), signed(h.max_drawdown)),
    ];
    if let (Some(c), Some(var)) = (h.confidence, &h.value_at_risk) {
        items.push((
            format!("Value at risk, {}, 1 {period}", pct(Some(c), 0, false)),
            money(var.value),
        ));
    }
    if let Some(es) = &h.expected_shortfall {
        items.push((format!("Expected shortfall, 1 {period}"), money(es.value)));
    }
    page.pairs(&items);

    if !o.growth.values.is_empty() {
        page.section("Growth of 100");
        let last = |s: &Series| num(s.values.last().copied(), 1);
        page.sparkline("Portfolio", &o.growth, 50, last(&o.growth));
        if let Some(b) = &o.benchmark_growth {
            let name = p
                .options
                .benchmark
                .as_ref()
                .map_or_else(|| "Benchmark".to_owned(), |b| b.to_string());
            page.sparkline(&name, b, 50, last(b));
        }
        if let (Some(first), Some(end)) = (o.growth.dates.first(), o.growth.dates.last()) {
            let (a, b) = (date(*first), date(*end));
            page.line(format!(
                "               {a}{}",
                pad_left(&b, 50usize.saturating_sub(width(&a)))
            ));
        }
    }
    for b in o.allocation.iter().skip(1) {
        page.section(&format!("Allocation by {}", words(&b.by)));
        page.bars(b, &group);
    }
    if !o.holdings.is_empty() {
        page.section("Holdings: expected return and risk");
        let rows: Vec<Vec<String>> = o
            .holdings
            .iter()
            .map(|x| {
                vec![
                    holding(&x.holding),
                    share(x.weight),
                    format!(
                        "{}{}",
                        signed(x.expected_return),
                        if x.expected_return_given { "*" } else { "" }
                    ),
                    share(x.volatility),
                    share(x.risk_share),
                ]
            })
            .collect();
        page.table(
            &[
                "Holding",
                "Weight",
                "Exp. return",
                "Volatility",
                "Share of risk",
            ],
            &rows,
            None,
        );
        if let Some(risk) = &o.risk {
            let given = if o.holdings.iter().any(|x| x.expected_return_given) {
                "Expected returns marked * are given; the others are historical means"
            } else {
                "Expected returns are historical means"
            };
            page.note(&format!(
                "{given}; covariance by {} over {}.",
                covariance_name(&risk.method),
                plural(risk.observations, period)
            ));
        }
    }
    if let (Some(frontier), Some(current)) = (&o.frontier, &o.current)
        && !frontier.is_empty()
    {
        page.section("Efficient frontier (invested holdings, long only)");
        let lowest = frontier
            .iter()
            .min_by(|a, b| a.volatility.total_cmp(&b.volatility));
        let similar = frontier.iter().min_by(|a, b| {
            (a.volatility - current.volatility)
                .abs()
                .total_cmp(&(b.volatility - current.volatility).abs())
        });
        let mut rows = vec![vec![
            "Current portfolio".to_owned(),
            signed(current.expected_return),
            share(current.volatility),
        ]];
        if let Some(x) = lowest {
            rows.push(vec![
                "Lowest volatility".to_owned(),
                signed(x.expected_return),
                share(x.volatility),
            ]);
        }
        if let Some(x) = similar {
            rows.push(vec![
                "At a similar volatility".to_owned(),
                signed(x.expected_return),
                share(x.volatility),
            ]);
        }
        page.table(&["", "Exp. return", "Volatility"], &rows, None);
    }
    page.footer(p);
}

pub(crate) fn holdings(page: &mut Page, h: &HoldingsReport) {
    let p = &h.provenance;
    let ccy = p.base_currency;
    page.header("Holdings", p);
    page.section(&format!("Positions · total {} {ccy}", money(dec(h.value))));
    let rows: Vec<Vec<String>> = h
        .holdings
        .iter()
        .map(|x| {
            let cash = matches!(x.holding, HoldingKey::Cash(_));
            vec![
                holding(&x.holding),
                money(dec(x.quantity)),
                x.price
                    .map_or_else(String::new, |v| money_places(dec(v), 2)),
                money(dec(x.value_base)),
                share(x.weight),
                if cash {
                    String::new()
                } else {
                    x.unrealised_base
                        .map_or_else(String::new, |v| money(dec(v)))
                },
                if cash {
                    String::new()
                } else {
                    x.unrealised_return.map_or_else(String::new, signed)
                },
            ]
        })
        .collect();
    page.table(
        &[
            "Holding",
            "Quantity",
            "Price",
            &format!("Value ({ccy})"),
            "Weight",
            "Unrealised",
            "Return",
        ],
        &rows,
        None,
    );
    if !h.accounts.is_empty() {
        page.section("Accounts");
        let rows: Vec<Vec<String>> = h
            .accounts
            .iter()
            .map(|a| {
                let cash: f64 = a.cash.iter().map(|c| dec(c.value_base)).sum();
                vec![
                    a.name.clone().unwrap_or_else(|| a.account.to_string()),
                    money(dec(a.value_base)),
                    share(a.weight),
                    money(cash),
                ]
            })
            .collect();
        page.table(
            &[
                "Account",
                &format!("Value ({ccy})"),
                "Weight",
                &format!("Cash ({ccy})"),
            ],
            &rows,
            None,
        );
    }
    let c = &h.concentration;
    page.section("Concentration");
    let mut items = Vec::new();
    if let Some(top) = c.largest.first() {
        items.push((
            "Largest holding".to_owned(),
            format!("{} {}", fit(&top.name, 16), share(top.weight)),
        ));
    }
    items.push(("Top five".to_owned(), share(c.top_five)));
    items.push((
        "Effective holdings".to_owned(),
        num(c.effective_holdings, 1),
    ));
    items.push(("Herfindahl index".to_owned(), num(c.herfindahl, 3)));
    page.pairs(&items);
    let names: BTreeMap<String, String> = h
        .accounts
        .iter()
        .map(|a| {
            (
                a.account.to_string(),
                a.name.clone().unwrap_or_else(|| a.account.to_string()),
            )
        })
        .collect();
    for b in h
        .breakdowns
        .iter()
        .filter(|b| b.by == "currency" || b.by == "account")
    {
        page.section(&format!("Allocation by {}", words(&b.by)));
        page.bars(b, &|name| {
            names.get(name).cloned().unwrap_or_else(|| group(name))
        });
    }
    page.section(&format!("Tax lots · {} open", h.lots.len()));
    if h.lots.is_empty() {
        page.note("No open lots.");
    } else {
        let days = p.options.long_term_after_days;
        let (long, short): (Vec<_>, Vec<_>) = h.lots.iter().partition(|x| x.long_term);
        let total =
            |lots: &[&crate::LotRow]| -> f64 { lots.iter().map(|x| dec(x.unrealised_base)).sum() };
        page.table(
            &["", "Lots", &format!("Unrealised ({ccy})")],
            &[
                vec![
                    format!("Long-term, over {days} days"),
                    long.len().to_string(),
                    money(total(&long)),
                ],
                vec![
                    "Short-term".to_owned(),
                    short.len().to_string(),
                    money(total(&short)),
                ],
            ],
            None,
        );
        let losses = h
            .lots
            .iter()
            .filter(|x| x.unrealised_base < Decimal::ZERO)
            .count();
        page.note(&format!("{} at a loss.", plural(losses, "lot")));
        let mut largest: Vec<&crate::LotRow> = h.lots.iter().collect();
        largest.sort_by_key(|x| std::cmp::Reverse(x.unrealised_base));
        page.blank();
        page.line("  Largest unrealised gains");
        let rows: Vec<Vec<String>> = largest
            .iter()
            .take(5)
            .map(|x| {
                vec![
                    x.lot.to_string(),
                    x.account.to_string(),
                    x.instrument.to_string(),
                    date(x.acquired),
                    money(dec(x.quantity)),
                    money(dec(x.unrealised_base)),
                    if x.long_term { "long" } else { "short" }.to_owned(),
                ]
            })
            .collect();
        page.table(
            &[
                "Lot",
                "Account",
                "Holding",
                "Acquired",
                "Quantity",
                &format!("Gain ({ccy})"),
                "Term",
            ],
            &rows,
            Some(&[L, L, L, R, R, R, L]),
        );
    }
    page.section(&format!("Realised gains and income ({ccy})"));
    let years: Vec<i32> = h
        .realised
        .by_year
        .keys()
        .chain(h.income.by_year.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let year_labels: Vec<String> = years.iter().map(i32::to_string).collect();
    let mut headings: Vec<&str> = vec![""];
    headings.extend(year_labels.iter().map(String::as_str));
    headings.push("Total");
    let row = |name: &str, by_year: &BTreeMap<i32, Decimal>, total: Decimal| {
        let mut cells = vec![name.to_owned()];
        cells.extend(
            years
                .iter()
                .map(|y| money(by_year.get(y).copied().map_or(0.0, dec))),
        );
        cells.push(money(dec(total)));
        cells
    };
    page.table(
        &headings,
        &[
            row("Realised gains", &h.realised.by_year, h.realised.total),
            row("Income", &h.income.by_year, h.income.total),
        ],
        None,
    );
    page.note(&format!(
        "Realised: {} short-term, {} long-term.",
        money(dec(h.realised.short_term)),
        money(dec(h.realised.long_term))
    ));
    page.footer(p);
}

pub(crate) fn performance(
    page: &mut Page,
    r: &PerformanceReport,
    names: &BTreeMap<String, String>,
) {
    let p = &r.provenance;
    let ccy = p.base_currency;
    let benchmark = r.benchmark_growth.is_some();
    page.header("Performance", p);
    page.section("Returns by period");
    let rows: Vec<Vec<String>> = r
        .periods
        .iter()
        .map(|x| {
            let mut row = vec![
                x.label.clone(),
                optional_date(x.start),
                signed(x.cumulative),
                signed(x.annualised),
                signed(x.money_weighted),
            ];
            if benchmark {
                row.push(signed(x.benchmark));
                row.push(signed(x.excess_arithmetic));
            }
            row
        })
        .collect();
    let mut headings = vec!["Period", "From", "Return", "Annualised", "Money-wtd"];
    if benchmark {
        headings.extend(["Benchmark", "Excess"]);
    }
    page.table(&headings, &rows, None);
    page.note(
        "Time-weighted unless noted; money-weighted returns are annual rates; – where the \
         history is too short.",
    );
    if !r.monthly.rows.is_empty() {
        page.section("Monthly returns (%)");
        let rows: Vec<Vec<String>> = r
            .monthly
            .rows
            .iter()
            .zip(&r.monthly.values)
            .map(|(year, values)| {
                std::iter::once(year.clone())
                    .chain(values.iter().map(|v| match v {
                        Some(v) => num(Some(*v * 100.0), 1),
                        None => "·".to_owned(),
                    }))
                    .collect()
            })
            .collect();
        let columns: Vec<String> = r
            .monthly
            .columns
            .iter()
            .map(|m| m.chars().take(if m == "Year" { 4 } else { 3 }).collect())
            .collect();
        let mut headings: Vec<&str> = vec![""];
        headings.extend(columns.iter().map(String::as_str));
        page.table_spaced(&headings, &rows, None, 1);
    }
    if !r.calendar_years.is_empty() {
        page.section("Calendar years");
        let rows: Vec<Vec<String>> = r
            .calendar_years
            .iter()
            .map(|y| {
                let mut row = vec![
                    format!("{}{}", y.year, if y.complete { "" } else { "*" }),
                    date(y.start),
                    date(y.end),
                    signed(y.portfolio),
                ];
                if benchmark {
                    row.push(signed(y.benchmark));
                    row.push(signed(y.excess));
                }
                row
            })
            .collect();
        let mut headings = vec!["Year", "From", "To", "Portfolio"];
        if benchmark {
            headings.extend(["Benchmark", "Excess"]);
        }
        page.table(&headings, &rows, None);
        if r.calendar_years.iter().any(|y| !y.complete) {
            page.note("* part of the year");
        }
    }
    page.section("Contribution to return, since inception");
    let mut ranked: Vec<&finalytics_perf::Contribution> = r.contributions.iter().collect();
    ranked.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
    let mut rows: Vec<Vec<String>> = ranked
        .iter()
        .map(|x| {
            vec![
                holding(&x.holding),
                signed(x.contribution),
                signed(x.return_base),
                signed(x.currency_effect),
                share(x.average_weight),
            ]
        })
        .collect();
    if r.expenses != 0.0 {
        rows.push(vec![
            "Fees and taxes".to_owned(),
            pct(Some(r.expenses), 2, true),
            String::new(),
            String::new(),
            String::new(),
        ]);
    }
    page.table(
        &[
            "Holding",
            "Contribution",
            "Return",
            "Currency",
            "Avg weight",
        ],
        &rows,
        None,
    );
    for g in &r.groups {
        page.blank();
        let heading = format!("By {}", words(&g.scheme));
        let rows: Vec<Vec<String>> = g
            .groups
            .iter()
            .map(|x| {
                vec![
                    group(&x.group),
                    signed(x.contribution),
                    share(x.average_weight),
                ]
            })
            .collect();
        page.table(&[&heading, "Contribution", "Avg weight"], &rows, None);
    }
    if let Some(hh) = &r.household {
        page.section("Accounts");
        let rows: Vec<Vec<String>> = hh
            .accounts
            .iter()
            .chain(std::iter::once(&hh.household))
            .map(|x| {
                vec![
                    names
                        .get(&x.name)
                        .cloned()
                        .unwrap_or_else(|| group(&x.name)),
                    money(x.value_start),
                    money(x.value_end),
                    money(x.flows),
                    money(x.gain),
                    signed(x.time_weighted),
                    signed(x.money_weighted),
                ]
            })
            .collect();
        page.table(
            &[
                "Account",
                "Start",
                "End",
                "Net flows",
                "Gain",
                "Return",
                "Money-wtd",
            ],
            &rows,
            None,
        );
    }
    if !r.flows.is_empty() {
        page.section(&format!("Flows and income ({ccy})"));
        let mut by_year: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
        for m in &r.flows {
            let e = by_year
                .entry(m.month.chars().take(4).collect())
                .or_default();
            e.0 += m.inflows;
            e.1 += m.outflows;
            e.2 += m.income;
        }
        let rows: Vec<Vec<String>> = by_year
            .iter()
            .map(|(y, (i, o, inc))| vec![y.clone(), money(*i), money(*o), money(*inc)])
            .collect();
        page.table(&["Year", "In", "Out", "Income"], &rows, None);
    }
    page.footer(p);
}

pub(crate) fn risk(page: &mut Page, r: &RiskReport) {
    let p = &r.provenance;
    let ccy = p.base_currency;
    let period = period_word(p.options.periods_per_year);
    let s = &r.statistics;
    page.header("Risk", p);
    let every = match period {
        "day" => "daily",
        "week" => "weekly",
        "month" => "monthly",
        "quarter" => "quarterly",
        "year" => "yearly",
        _ => "periodic",
    };
    page.section(&format!(
        "Statistics · {}, annualised",
        plural(s.observations, &format!("{every} return"))
    ));
    page.pairs(&[
        ("Return".to_owned(), signed(s.annualised_return)),
        ("Volatility".to_owned(), share(s.volatility)),
        ("Downside deviation".to_owned(), share(s.downside_deviation)),
        ("Sharpe ratio".to_owned(), num(s.sharpe_ratio, 2)),
        ("Sortino ratio".to_owned(), num(s.sortino_ratio, 2)),
        ("Calmar ratio".to_owned(), num(s.calmar_ratio, 2)),
        ("Maximum drawdown".to_owned(), signed(s.max_drawdown)),
        (format!("Best {period}"), pct(Some(s.best), 2, true)),
        (format!("Worst {period}"), pct(Some(s.worst), 2, true)),
        ("Skewness".to_owned(), num(s.skewness, 2)),
        ("Excess kurtosis".to_owned(), num(s.excess_kurtosis, 2)),
    ]);
    if let Some(rel) = &r.relative {
        page.section("Against the benchmark");
        page.pairs(&[
            ("Active return".to_owned(), signed(rel.active_return)),
            ("Tracking error".to_owned(), share(rel.tracking_error)),
            (
                "Information ratio".to_owned(),
                num(rel.information_ratio, 2),
            ),
            ("Beta".to_owned(), num(rel.beta, 2)),
            ("Alpha".to_owned(), signed(rel.alpha)),
            ("Correlation".to_owned(), num(rel.correlation, 2)),
        ]);
    }
    if !r.tail.is_empty() {
        page.section(&format!(
            "Value at risk and expected shortfall, 1 {period} ({ccy})"
        ));
        let rows: Vec<Vec<String>> = r
            .tail
            .iter()
            .map(|x| {
                vec![
                    pct(Some(x.confidence), 0, false),
                    tail_name(&x.method).to_owned(),
                    money(x.value_at_risk.value),
                    pct(Some(x.value_at_risk.fraction), 2, false),
                    money(x.expected_shortfall.value),
                    pct(Some(x.expected_shortfall.fraction), 2, false),
                ]
            })
            .collect();
        page.table(
            &[
                "Confidence",
                "Method",
                "Value at risk",
                "",
                "Exp. shortfall",
                "",
            ],
            &rows,
            Some(&[L, L, R, R, R, R]),
        );
    }
    if !r.drawdown.values.is_empty() {
        page.section("Drawdowns");
        page.line(format!("  {}", spark(&r.drawdown.values, 60)));
        if !r.drawdowns.is_empty() {
            page.blank();
            let rows: Vec<Vec<String>> = r
                .drawdowns
                .iter()
                .take(5)
                .map(|x| {
                    vec![
                        date(x.peak),
                        date(x.trough),
                        x.recovery.map_or_else(|| "not yet".to_owned(), date),
                        signed(x.depth),
                        x.days_to_trough.to_string(),
                        x.days_to_recovery
                            .map_or_else(|| "–".to_owned(), |d| d.to_string()),
                    ]
                })
                .collect();
            page.table(
                &[
                    "Peak",
                    "Trough",
                    "Recovery",
                    "Depth",
                    "Days down",
                    "Days back",
                ],
                &rows,
                Some(&[L, L, L, R, R, R]),
            );
        }
    }
    if let Some(hr) = &r.holdings {
        let d = &hr.decomposition;
        page.section(&format!(
            "Where the risk comes from · volatility {}",
            share(d.volatility)
        ));
        let mut ranked: Vec<_> = d.contributions.iter().collect();
        ranked.sort_by(|a, b| b.share.unwrap_or(0.0).total_cmp(&a.share.unwrap_or(0.0)));
        let lw = ranked
            .iter()
            .map(|x| width(&fit(&x.label, 14)))
            .max()
            .unwrap_or(8);
        for x in ranked {
            page.line(format!(
                "  {} weight {}   risk {} {}",
                pad_right(&fit(&x.label, 14), lw),
                pad_left(&share(x.weight), 6),
                bar(x.share.unwrap_or(0.0), 30),
                pad_left(&share(x.share), 6)
            ));
        }
        for g in &hr.groups {
            page.blank();
            let mut ranked: Vec<_> = g.groups.iter().collect();
            ranked.sort_by(|a, b| b.share.unwrap_or(0.0).total_cmp(&a.share.unwrap_or(0.0)));
            let names: Vec<String> = ranked.iter().map(|x| fit(&group(&x.group), 16)).collect();
            let lw = names.iter().map(|n| width(n)).max().unwrap_or(8);
            for (x, name) in ranked.iter().zip(&names) {
                page.line(format!(
                    "  {} weight {}   risk {} {}",
                    pad_right(name, lw),
                    pad_left(&share(x.weight), 6),
                    bar(x.share.unwrap_or(0.0), 24),
                    pad_left(&share(x.share), 6)
                ));
            }
        }
        page.section("Correlation");
        let (names, values, riskless) = with_risk(&hr.correlation);
        let columns: Vec<String> = names.iter().map(|l| l.replace(" cash", "")).collect();
        let mut headings: Vec<&str> = vec![""];
        headings.extend(columns.iter().map(String::as_str));
        let rows: Vec<Vec<String>> = names
            .iter()
            .zip(&values)
            .map(|(row, values)| {
                std::iter::once(row.clone())
                    .chain(values.iter().map(|v| match v {
                        Some(v) => num(Some(*v), 2),
                        None => "·".to_owned(),
                    }))
                    .collect()
            })
            .collect();
        page.table(&headings, &rows, None);
        if !riskless.is_empty() {
            page.note(&format!(
                "{} has no risk and is left out.",
                riskless.join(", ")
            ));
        }
    }
    if !r.rolling.is_empty() {
        page.section(&format!(
            "Rolling {}-{period} figures (last value)",
            p.options.rolling_window
        ));
        for s in &r.rolling {
            let last = s.values.last().copied();
            let shown = match s.unit {
                crate::Unit::Fraction => share(last),
                _ => num(last, 2),
            };
            let name = s.name.split(" (").next().unwrap_or(&s.name).to_owned();
            page.line(format!(
                "  {} {}  {shown}",
                pad_right(&fit(&name, 24), 24),
                spark(&s.values, 40)
            ));
        }
    }
    page.footer(p);
}

pub(crate) fn attribution(page: &mut Page, a: &AttributionReport) {
    let p = &a.provenance;
    let at = &a.attribution;
    page.header("Attribution", p);
    let linking = match at.linking {
        Linking::Carino => "Cariño",
        Linking::Menchero => "Menchero",
        _ => "linked",
    };
    page.section(&format!(
        "Brinson–Fachler by {}, {} linked ({linking})",
        words(&a.scheme),
        plural(a.periods.len(), "period")
    ));
    page.pairs(&[
        (
            "Portfolio return".to_owned(),
            pct(Some(at.portfolio_return), 2, true),
        ),
        (
            "Benchmark return".to_owned(),
            pct(Some(at.benchmark_return), 2, true),
        ),
        (
            "Excess return".to_owned(),
            pct(Some(at.excess_return), 2, true),
        ),
        ("Allocation".to_owned(), pct(Some(at.allocation), 2, true)),
        ("Selection".to_owned(), pct(Some(at.selection), 2, true)),
        ("Interaction".to_owned(), pct(Some(at.interaction), 2, true)),
    ]);
    page.blank();
    let two = |x: f64| pct(Some(x), 2, true);
    let mut rows: Vec<Vec<String>> = at
        .segments
        .iter()
        .map(|s| {
            vec![
                group(&s.name),
                two(s.allocation),
                two(s.selection),
                two(s.interaction),
                two(s.total),
            ]
        })
        .collect();
    rows.push(vec![
        "Total".to_owned(),
        two(at.allocation),
        two(at.selection),
        two(at.interaction),
        two(at.excess_return),
    ]);
    page.table(
        &["Segment", "Allocation", "Selection", "Interaction", "Total"],
        &rows,
        None,
    );
    if let Some(twr) = a.time_weighted {
        page.note(&format!(
            "Segment returns are measured over each period, so the linked total can differ \
             slightly from the time-weighted return over the same span ({}).",
            pct(Some(twr), 2, true)
        ));
    }
    page.footer(p);
}

pub(crate) fn scenarios(page: &mut Page, s: &ScenariosReport) {
    let p = &s.provenance;
    let ccy = p.base_currency;
    page.header("Scenarios", p);
    page.section(&format!(
        "Effect on today's portfolio · value {} {ccy}",
        money(s.value)
    ));
    let rows: Vec<Vec<String>> = s
        .summary
        .iter()
        .map(|x| {
            vec![
                x.name.clone(),
                money(x.change),
                signed(x.portfolio_return),
                x.worst.as_ref().map_or_else(|| "–".to_owned(), holding),
                x.worst_change.map_or_else(|| "–".to_owned(), money),
                x.volatility_after.map_or_else(|| "–".to_owned(), share),
            ]
        })
        .collect();
    page.table(
        &[
            "Scenario",
            "Change",
            "Return",
            "Hardest hit",
            "",
            "Vol after",
        ],
        &rows,
        Some(&[L, R, R, L, R, R]),
    );
    for outcome in s.scenarios.iter().filter(|o| o.result.is_none()) {
        let reason = outcome
            .problems
            .first()
            .map_or_else(String::new, |i| format!(": {}", i.message));
        page.note(&format!("{} could not be run{reason}", outcome.name));
    }
    if !s.changes.rows.is_empty() {
        page.section(&format!("Change in value by holding ({ccy})"));
        let mut headings: Vec<&str> = vec!["Holding"];
        headings.extend(s.changes.columns.iter().map(String::as_str));
        let rows: Vec<Vec<String>> = s
            .changes
            .rows
            .iter()
            .zip(&s.changes.values)
            .map(|(row, values)| {
                std::iter::once(row.clone())
                    .chain(
                        values
                            .iter()
                            .map(|v| v.map_or_else(|| "–".to_owned(), money)),
                    )
                    .collect()
            })
            .collect();
        page.table(&headings, &rows, None);
    }
    for named in &p.options.scenarios {
        if let ScenarioDefinition::Historical { window } = &named.scenario {
            page.note(&format!(
                "{}: the price and rate changes from {} to {}, on today's holdings.",
                named.name,
                date(window.start),
                date(window.end)
            ));
        }
    }
    page.footer(p);
}

impl fmt::Display for OverviewReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        overview(&mut page, self);
        f.write_str(&page.finish())
    }
}

impl fmt::Display for HoldingsReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        holdings(&mut page, self);
        f.write_str(&page.finish())
    }
}

impl fmt::Display for PerformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        performance(&mut page, self, &BTreeMap::new());
        f.write_str(&page.finish())
    }
}

impl fmt::Display for RiskReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        risk(&mut page, self);
        f.write_str(&page.finish())
    }
}

impl fmt::Display for AttributionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        attribution(&mut page, self);
        f.write_str(&page.finish())
    }
}

impl fmt::Display for ScenariosReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut page = Page::new(None);
        scenarios(&mut page, self);
        f.write_str(&page.finish())
    }
}

impl PortfolioReport {
    /// The six reports as text for printing, 80 columns wide, with the portfolio's `name` (or
    /// the report's own), if given, under each report's title.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let mut page = Page::new(name.or(self.name.as_deref()));
        overview(&mut page, &self.overview);
        holdings(&mut page, &self.holdings);
        performance(&mut page, &self.performance, &account_names(&self.holdings));
        risk(&mut page, &self.risk);
        if let Some(a) = &self.attribution {
            attribution(&mut page, a);
        }
        if let Some(s) = &self.scenarios {
            scenarios(&mut page, s);
        }
        page.finish()
    }
}

impl fmt::Display for PortfolioReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_formatted_for_reading() {
        assert_eq!(money(1234567.4), "1,234,567");
        assert_eq!(money(-950.0), "−950");
        assert_eq!(money(0.4), "0");
        assert_eq!(money(-0.4), "0");
        assert_eq!(money_places(548.655, 2), "548.66");
        assert_eq!(pct(Some(0.0412), 1, true), "+4.1%");
        assert_eq!(pct(Some(-0.00001), 1, true), "+0.0%");
        assert_eq!(pct(Some(-0.129), 1, true), "−12.9%");
        assert_eq!(pct(None, 1, true), "–");
        assert_eq!(share(0.05), "5.0%");
        assert_eq!(
            date("2026-10-30".parse().unwrap_or_else(|e| panic!("{e}"))),
            "30 Oct 2026"
        );
    }

    #[test]
    fn bars_and_sparklines_have_their_width() {
        assert_eq!(width(&bar(0.5, 20)), 20);
        assert_eq!(width(&bar(1.5, 20)), 20);
        assert_eq!(bar(0.0, 4), "    ");
        assert_eq!(width(&spark(&[1.0, 2.0, 3.0], 10)), 10);
        assert_eq!(spark(&[1.0, 2.0], 2), "▁█");
    }

    #[test]
    fn wide_tables_are_split() {
        let mut page = Page::new(None);
        let headings: Vec<String> = (0..12).map(|i| format!("Column {i}")).collect();
        let heads: Vec<&str> = std::iter::once("Name")
            .chain(headings.iter().map(String::as_str))
            .collect();
        let row: Vec<String> = std::iter::once("first".to_owned())
            .chain((0..12).map(|i| format!("{}", i * 1000)))
            .collect();
        page.table(&heads, &[row], None);
        let text = page.finish();
        assert!(text.lines().all(|l| width(l) <= WIDTH), "{text}");
        assert!(
            text.lines()
                .filter(|l| l.trim_start().starts_with("first"))
                .count()
                > 1
        );
    }

    #[test]
    fn headings_wrap_at_a_space() {
        assert_eq!(
            split_heading("Spring 2025 replay"),
            ("Spring 2025".to_owned(), Some("replay".to_owned()))
        );
        assert_eq!(split_heading("Change"), ("Change".to_owned(), None));
    }
}
