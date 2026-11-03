//! The optimisation and backtest reports as self-contained HTML pages, in the style of the
//! portfolio report: inline styles and SVG charts, no scripts, light and dark themes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use finalytics_backtest::{Rebalance, Validation};
use finalytics_core::Date;

use crate::backtests::{
    BacktestContext, BacktestReport, CalendarReturn, ComparisonReport, RunReport, StrategyReport,
    WeightsAt,
};
use crate::html::{
    CSS, LOGO_DARK, LOGO_LIGHT, axis_pct, base64, card, column_path, esc, heat, legend, ticks,
    tiles,
};
use crate::shapes::Series;
use crate::text::{covariance_name, month_name, num, pct, period_word, plural};

/// The colours of the strategies, in order; a benchmark is drawn in the secondary ink.
pub(crate) const SERIES: [&str; 8] = [
    "var(--s1)",
    "var(--s2)",
    "var(--s3)",
    "var(--s4)",
    "var(--s5)",
    "var(--s6)",
    "var(--s7)",
    "var(--s8)",
];
pub(crate) const BENCHMARK: &str = "var(--ink2)";

/// The most assets a table of weights shows; the rest are shown together.
const ASSETS: usize = 12;

/// Weights smaller than this are shown as none.
pub(crate) const TINY: f64 = 5e-4;

pub(crate) fn colour(k: usize) -> &'static str {
    SERIES.get(k).copied().unwrap_or("var(--muted-fill)")
}

// ── Words ────────────────────────────────────────────────────────────────────────────────────

/// A month and year, such as "Nov 2016".
pub(crate) fn month(d: Date) -> String {
    format!("{} {}", month_name(d.month()), d.year())
}

/// From one month to another, such as "Nov 2001 to Oct 2026".
pub(crate) fn span(from: Date, to: Date) -> String {
    format!("{} to {}", month(from), month(to))
}

/// How often the returns are, such as "monthly".
pub(crate) fn frequency(ppy: f64) -> &'static str {
    match period_word(ppy) {
        "day" => "daily",
        "week" => "weekly",
        "month" => "monthly",
        "quarter" => "quarterly",
        "year" => "yearly",
        _ => "periodic",
    }
}

/// The rebalancing schedule in a few words, such as "rebalanced quarterly".
pub(crate) fn rebalancing(r: &Rebalance) -> String {
    match r {
        Rebalance::Monthly => "rebalanced monthly".to_owned(),
        Rebalance::Quarterly => "rebalanced quarterly".to_owned(),
        Rebalance::Yearly => "rebalanced yearly".to_owned(),
        Rebalance::Drift { band } => format!(
            "rebalanced when a weight drifts {} points",
            num(
                Some(band * 100.0),
                if (band * 100.0).fract() == 0.0 { 0 } else { 1 }
            )
        ),
        Rebalance::Never => "bought and held".to_owned(),
        _ => "rebalanced".to_owned(),
    }
}

/// The rebalancing schedule as a sentence, for the method notes.
pub(crate) fn rebalancing_sentence(r: &Rebalance) -> String {
    match r {
        Rebalance::Monthly | Rebalance::Quarterly | Rebalance::Yearly => format!(
            "{}, to the targets; between rebalances the weights drift with prices.",
            capitalise(rebalancing(r).trim_start_matches("rebalanced "))
        ),
        Rebalance::Drift { band } => format!(
            "At the end of any period in which a weight is more than {} away from its target; \
             otherwise the weights drift with prices.",
            pct(Some(*band), 1, false)
        ),
        Rebalance::Never => "Never: the weights are bought once and drift with prices.".to_owned(),
        _ => "As scheduled.".to_owned(),
    }
}

/// The trading costs in a few words, such as "10 bp trading costs".
pub(crate) fn costs(bps: f64) -> String {
    if bps > 0.0 {
        format!(
            "{} bp trading costs",
            num(Some(bps), if bps.fract() == 0.0 { 0 } else { 1 })
        )
    } else {
        "no trading costs".to_owned()
    }
}

/// The validation in a few words, such as "Walk-forward on 5 years".
pub(crate) fn validation(c: &BacktestContext) -> String {
    match &c.settings.validation {
        Validation::InSample => "In sample".to_owned(),
        Validation::Holdout { .. } => format!("Holdout from {}", month(c.test_start)),
        Validation::WalkForward {
            years,
            periods,
            expanding,
        } => {
            let window = match (years, periods) {
                (Some(y), _) => years_words(*y),
                (None, Some(p)) => plural(*p, period_word(c.periods_per_year)),
                (None, None) => "a window".to_owned(),
            };
            if *expanding {
                format!("Walk-forward, expanding from {window}")
            } else {
                format!("Walk-forward on {window}")
            }
        }
        _ => "Validated".to_owned(),
    }
}

fn years_words(y: f64) -> String {
    if (y - y.round()).abs() < 1e-9 {
        plural(y.round() as usize, "year")
    } else {
        format!("{} months", (y * 12.0).round())
    }
}

/// What the scored periods are: "Out of sample" or "In sample".
pub(crate) fn tested_label(c: &BacktestContext) -> &'static str {
    match c.settings.validation {
        Validation::InSample => "In sample",
        _ => "Out of sample",
    }
}

pub(crate) fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub(crate) fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A name inside a sentence: "Risk parity" as "risk parity", but "S&P 500" as it is.
pub(crate) fn lower_name(name: &str) -> String {
    if name.chars().nth(1).is_some_and(char::is_lowercase) {
        lower_first(name)
    } else {
        name.to_owned()
    }
}

/// The limits on the weights in a sentence.
pub(crate) fn limits_sentence(c: &BacktestContext) -> String {
    let limits: Vec<String> = c.limits.iter().map(|l| limit_in_sentence(l)).collect();
    limits.join("; ")
}

/// A limit inside a sentence: "At most 40% in any asset" as "at most 40% in any asset", but a
/// limit that starts with an asset's or a group's name, such as "Intl Equity: 0% to 30%", as it
/// is.
pub(crate) fn limit_in_sentence(limit: &str) -> String {
    match limit.strip_prefix("At most ") {
        Some(rest) => format!("at most {rest}"),
        None => limit.to_owned(),
    }
}

/// Names joined as in a sentence: "a, b and c".
pub(crate) fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The strategies' notes, each once, naming the strategies it concerns.
pub(crate) fn grouped_notes(strategies: &[&StrategyReport]) -> Vec<String> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for s in strategies {
        for note in &s.notes {
            match groups.iter_mut().find(|(n, _)| n == note) {
                Some((_, names)) => names.push(s.name.clone()),
                None => groups.push((note.clone(), vec![s.name.clone()])),
            }
        }
    }
    groups
        .into_iter()
        .map(|(note, names)| {
            let names: Vec<String> = names
                .iter()
                .enumerate()
                .map(|(k, n)| if k == 0 { n.clone() } else { lower_name(n) })
                .collect();
            format!("{}: {}", and_list(&names), lower_first(&note))
        })
        .collect()
}

/// A sentence for each strategy whose refits found no weights.
pub(crate) fn failure_notes(strategies: &[&StrategyReport]) -> Vec<String> {
    strategies
        .iter()
        .filter(|s| !s.failures.is_empty())
        .map(|s| {
            let mut reasons: Vec<&str> = s.failures.iter().map(|f| f.reason.as_str()).collect();
            reasons.sort_unstable();
            reasons.dedup();
            let first = s.failures.iter().map(|f| f.date).min();
            let last = s.failures.iter().map(|f| f.date).max();
            let when = match (first, last) {
                (Some(a), Some(b)) if a != b => format!(", from {} to {}", month(a), month(b)),
                (Some(a), _) => format!(", in {}", month(a)),
                _ => String::new(),
            };
            format!(
                "{} found no weights at {}{when} ({}); it kept the weights it held.",
                s.name,
                plural(s.failures.len(), "refit"),
                reasons.join(", ")
            )
        })
        .collect()
}

/// The assets worth showing in tables of weights, largest first when there are many, with the
/// rest together.
pub(crate) fn shown_assets(
    c: &BacktestContext,
    weights: &[&BTreeMap<String, f64>],
) -> (Vec<String>, Vec<String>) {
    let largest = |a: &String| {
        weights
            .iter()
            .map(|w| w.get(a).copied().unwrap_or(0.0))
            .fold(0.0, f64::max)
    };
    let held: Vec<String> = c
        .assets
        .iter()
        .filter(|a| largest(a) >= TINY)
        .cloned()
        .collect();
    if held.len() <= ASSETS {
        return (held, Vec::new());
    }
    let mut order = held;
    order.sort_by(|a, b| largest(b).total_cmp(&largest(a)));
    let rest = order.split_off(ASSETS);
    (order, rest)
}

// ── Charts ───────────────────────────────────────────────────────────────────────────────────

/// One line of a chart over dates.
pub(crate) struct Line<'a> {
    pub(crate) name: &'a str,
    pub(crate) colour: &'a str,
    pub(crate) series: &'a Series,
}

/// A chart's history split in two, with the part before `at` shaded.
pub(crate) struct Shade<'a> {
    at: Date,
    left: &'a str,
    right: &'a str,
    bottom: bool,
}

/// A date axis: from the first date to the last, with labels at whole years, or months over a
/// short span.
struct Axis {
    first: Date,
    days: f64,
    left: f64,
    width: f64,
}

impl Axis {
    fn new(dates: impl Iterator<Item = Date> + Clone, left: f64, width: f64) -> Option<Axis> {
        let first = dates.clone().min()?;
        let last = dates.max()?;
        let days = f64::from(first.days_until(last)).max(1.0);
        Some(Axis {
            first,
            days,
            left,
            width,
        })
    }

    fn x(&self, d: Date) -> f64 {
        self.left + self.width * f64::from(self.first.days_until(d)) / self.days
    }

    /// The labels along the bottom of a chart `height` high.
    fn labels(&self, height: f64) -> String {
        let mut out = String::new();
        let last_year = self
            .first
            .add_days(self.days as i32)
            .map_or(self.first.year(), |d| d.year());
        let years = last_year - self.first.year();
        if self.days < 400.0 {
            let mut d = self.first.end_of_month();
            while let Ok(next) = d.add_days(1) {
                if f64::from(self.first.days_until(next)) > self.days {
                    break;
                }
                let _ = write!(
                    out,
                    r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{} {}</text>"#,
                    self.x(next),
                    height - 8.0,
                    month_name(next.month()),
                    next.year() % 100
                );
                d = next.end_of_month();
            }
            return out;
        }
        let every = if years <= 12 {
            1
        } else if years <= 18 {
            2
        } else {
            5
        };
        for year in (self.first.year() + 1)..=last_year {
            if year % every != 0 {
                continue;
            }
            if let Ok(jan) = Date::from_ymd(year, 1, 1) {
                let _ = write!(
                    out,
                    r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{year}</text>"#,
                    self.x(jan),
                    height - 8.0
                );
            }
        }
        out
    }
}

/// The value of `series` at `date`, or at the last date before it.
fn value_at(series: &Series, date: Date) -> Option<f64> {
    let k = series.dates.partition_point(|d| *d <= date);
    k.checked_sub(1)
        .and_then(|k| series.values.get(k).copied())
        .filter(|v| v.is_finite())
}

/// A line chart over dates, with hover bands that show each date's values.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lines(
    series: &[Line<'_>],
    format: &dyn Fn(f64) -> String,
    tick: &dyn Fn(f64) -> String,
    height: f64,
    zero: bool,
    shade: Option<&Shade<'_>>,
    legend_values: Option<&[String]>,
) -> String {
    let (w, h, l, r, t, b) = (880.0, height, 56.0, 64.0, 12.0, 28.0);
    let dates = series.iter().flat_map(|s| s.series.dates.iter().copied());
    let Some(axis) = Axis::new(dates, l, w - l - r) else {
        return String::new();
    };
    let values: Vec<f64> = series
        .iter()
        .flat_map(|s| s.series.values.iter().copied())
        .filter(|v| v.is_finite())
        .collect();
    if values.is_empty() {
        return String::new();
    }
    let mut lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let mut hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    let grid = ticks(lo, hi, 5);
    let (lo, hi) = (lo.min(grid[0]), hi.max(grid.last().copied().unwrap_or(hi)));
    let span = if hi > lo { hi - lo } else { 1.0 };
    let y = |v: f64| t + (h - t - b) * (hi - v) / span;
    let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
    if let Some(s) = shade {
        let sx = axis.x(s.at);
        let ly = if s.bottom { h - b - 8.0 } else { t + 16.0 };
        let _ = write!(
            out,
            r#"<rect x="{l}" y="{t}" width="{:.1}" height="{}" style="fill:var(--mid);opacity:.55"/><line class="base" x1="{sx:.1}" x2="{sx:.1}" y1="{t}" y2="{}" style="stroke-dasharray:3 3"/><text class="lab" x="{}" y="{ly}">{}</text><text class="lab" x="{:.1}" y="{ly}">{}</text>"#,
            (sx - l).max(0.0),
            h - t - b,
            h - b,
            l + 8.0,
            esc(s.left),
            sx + 8.0,
            esc(s.right)
        );
    }
    for v in &grid {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            esc(&tick(*v))
        );
    }
    out.push_str(&axis.labels(h));
    if zero && lo < 0.0 && hi > 0.0 {
        let _ = write!(
            out,
            r#"<line class="base" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/>"#,
            w - r,
            y(0.0),
            y(0.0)
        );
    }
    let mut ends: Vec<f64> = series
        .iter()
        .filter_map(|s| s.series.values.last().map(|v| y(*v)))
        .collect();
    ends.sort_by(f64::total_cmp);
    let end_labels = legend_values.is_none()
        && series.len() <= 4
        && ends.windows(2).all(|p| p[1] - p[0] >= 16.0);
    for s in series {
        let points: Vec<(f64, f64)> = s
            .series
            .dates
            .iter()
            .zip(&s.series.values)
            .filter(|(_, v)| v.is_finite())
            .map(|(d, v)| (axis.x(*d), y(*v)))
            .collect();
        let Some(&(last_x, last_y)) = points.last() else {
            continue;
        };
        let path: String = points
            .iter()
            .enumerate()
            .map(|(k, (px, py))| format!("{}{px:.1},{py:.1}", if k == 0 { "M" } else { " L" }))
            .collect();
        let _ = write!(
            out,
            r#"<path class="line" d="{path}" style="stroke:{}"/><circle class="dot" cx="{last_x:.1}" cy="{last_y:.1}" r="4" style="fill:{}"/>"#,
            s.colour, s.colour
        );
        if end_labels && let Some(v) = s.series.values.last() {
            let _ = write!(
                out,
                r#"<text class="end" x="{:.1}" y="{:.1}">{}</text>"#,
                last_x + 10.0,
                last_y + 4.0,
                esc(&format(*v))
            );
        }
    }
    // Hover bands over the dates of the longest series.
    if let Some(base) = series
        .iter()
        .map(|s| &s.series.dates)
        .max_by_key(|d| d.len())
    {
        let n = base.len();
        let bands = 110.min(n);
        for k in 0..bands {
            let (i0, i1) = (k * n / bands, ((k + 1) * n / bands).min(n - 1));
            let i = ((i0 + i1) / 2).min(n - 1);
            let mut tip = month(base[i]);
            for s in series {
                if let Some(v) = value_at(s.series, base[i]) {
                    let _ = write!(tip, "\n{}: {}", s.name, format(v));
                }
            }
            let x0 = axis.x(base[i0]);
            let x1 = if i1 > i0 { axis.x(base[i1]) } else { x0 + 1.0 };
            let _ = write!(
                out,
                r#"<rect class="hit" x="{x0:.1}" y="{t}" width="{:.1}" height="{}"><title>{}</title></rect>"#,
                (x1 - x0).max(1.0) + 0.5,
                h - t - b,
                esc(&tip)
            );
        }
    }
    out.push_str("</svg>");
    if series.len() > 1 {
        let keys: Vec<(String, &str)> = series
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let name = match legend_values.and_then(|v| v.get(k)) {
                    Some(v) => format!("{} {v}", s.name),
                    None if end_labels => s.name.to_owned(),
                    None => format!(
                        "{} {}",
                        s.name,
                        s.series
                            .values
                            .last()
                            .map_or_else(String::new, |v| format(*v))
                    ),
                };
                (name, s.colour)
            })
            .collect();
        legend(&keys) + &out
    } else {
        out
    }
}

/// Grouped columns, one group per year, with the groups before `shade` shaded.
fn year_columns(
    categories: &[String],
    series: &[(&str, &str, Vec<Option<f64>>)],
    height: f64,
    shade: Option<(usize, &str, &str)>,
) -> String {
    if categories.is_empty() || series.is_empty() {
        return String::new();
    }
    let (w, h, l, r, t, b) = (880.0, height, 56.0, 16.0, 24.0, 28.0);
    let values: Vec<f64> = series
        .iter()
        .flat_map(|s| s.2.iter().flatten().copied())
        .filter(|v| v.is_finite())
        .collect();
    let lo = values.iter().copied().fold(0.0, f64::min);
    let hi = values.iter().copied().fold(0.0, f64::max);
    let grid = ticks(lo, hi, 5);
    let (lo, hi) = (
        grid[0].min(0.0),
        grid.last().copied().unwrap_or(hi).max(0.0),
    );
    let span = if hi > lo { hi - lo } else { 1.0 };
    let y = |v: f64| t + (h - t - b) * (hi - v) / span;
    let band = (w - l - r) / categories.len() as f64;
    let m = series.len() as f64;
    let cw = 14f64.min((band - 4.0) / m - 1.0).max(1.0);
    let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
    if let Some((i, left, right)) = shade {
        let _ = write!(
            out,
            r#"<rect x="{l}" y="{}" width="{:.1}" height="{}" style="fill:var(--mid);opacity:.55"/><text class="lab" x="{}" y="{}">{}</text><text class="lab" x="{:.1}" y="{}">{}</text>"#,
            t - 12.0,
            band * i as f64,
            h - t - b + 12.0,
            l + 8.0,
            t + 2.0,
            esc(left),
            l + band * i as f64 + 8.0,
            t + 2.0,
            esc(right)
        );
    }
    let step = grid.get(1).map_or(1.0, |g1| g1 - grid[0]);
    for v in &grid {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            esc(&pct(Some(*v), usize::from(step < 0.01), false))
        );
    }
    let _ = write!(
        out,
        r#"<line class="base" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/>"#,
        w - r,
        y(0.0),
        y(0.0)
    );
    let every = if categories.len() <= 12 { 1 } else { 5 };
    for (k, category) in categories.iter().enumerate() {
        let cx = l + band * (k as f64 + 0.5);
        let start = cx - (cw + 1.0) * m / 2.0;
        let mut tip = category.clone();
        for (j, (name, colour, vs)) in series.iter().enumerate() {
            let Some(v) = vs.get(k).copied().flatten() else {
                continue;
            };
            let _ = write!(tip, "\n{name}: {}", pct(Some(v), 1, true));
            let x = start + j as f64 * (cw + 1.0);
            let _ = write!(
                out,
                r#"<path d="{}" style="fill:{colour}"/>"#,
                column_path(x, y(0.0), cw, y(v), v >= 0.0)
            );
        }
        let year: i32 = category
            .chars()
            .take(4)
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        if every == 1 || year % every == 0 {
            let _ = write!(
                out,
                r#"<text class="tick" x="{cx:.1}" y="{}" text-anchor="middle">{}</text>"#,
                h - 8.0,
                esc(&category.chars().take(4).collect::<String>())
            );
        }
        let _ = write!(
            out,
            r#"<rect class="hit" x="{:.1}" y="{t}" width="{band:.1}" height="{}"><title>{}</title></rect>"#,
            l + band * k as f64,
            h - t - b,
            esc(&tip)
        );
    }
    out.push_str("</svg>");
    let keys: Vec<(String, &str)> = series
        .iter()
        .map(|(n, c, _)| ((*n).to_owned(), *c))
        .collect();
    legend(&keys) + &out
}

/// A strategy's volatility and return, in sample and out of sample.
struct Point<'a> {
    name: &'a str,
    colour: &'a str,
    in_sample: (f64, f64),
    tested: (f64, f64),
}

/// Each strategy's volatility and return in sample (hollow) and out of sample (filled).
fn scatter(points: &[Point<'_>]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let (w, h, l, r, t, b) = (880.0, 320.0, 56.0, 190.0, 30.0, 40.0);
    let xs: Vec<f64> = points
        .iter()
        .flat_map(|p| [p.in_sample.0, p.tested.0])
        .collect();
    let ys: Vec<f64> = points
        .iter()
        .flat_map(|p| [p.in_sample.1, p.tested.1])
        .collect();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let gx = ticks(min(&xs) - 0.005, max(&xs) + 0.005, 5);
    let gy = ticks(min(&ys) - 0.003, max(&ys) + 0.003, 5);
    let (x0, x1) = (gx[0], gx.last().copied().unwrap_or(1.0));
    let (y0, y1) = (gy[0], gy.last().copied().unwrap_or(1.0));
    let x = |v: f64| l + (w - l - r) * (v - x0) / (x1 - x0).max(1e-12);
    let y = |v: f64| t + (h - t - b) * (y1 - v) / (y1 - y0).max(1e-12);
    let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
    let ystep = gy.get(1).map_or(1.0, |g| g - gy[0]);
    for v in &gy {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            esc(&pct(Some(*v), usize::from(ystep < 0.01), false))
        );
    }
    for v in &gx {
        let _ = write!(
            out,
            r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
            x(*v),
            h - 20.0,
            esc(&axis_pct(*v, false))
        );
    }
    let _ = write!(
        out,
        r#"<text class="tick" x="{:.0}" y="{}" text-anchor="middle">Volatility, a year</text><text class="tick" x="{}" y="{}">Return, a year</text>"#,
        (l + w - r) / 2.0,
        h - 2.0,
        l - 44.0,
        t - 16.0
    );
    // Labels at the right, in the order of the out-of-sample points, kept apart.
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|a, b| y(points[*a].tested.1).total_cmp(&y(points[*b].tested.1)));
    let mut placed: Vec<f64> = Vec::new();
    for &k in &order {
        let want = y(points[k].tested.1);
        let at = placed.last().map_or(want, |p| want.max(p + 15.0));
        placed.push(at);
    }
    for (&k, ly) in order.iter().zip(&placed) {
        let Point {
            name,
            colour,
            in_sample: ins,
            tested: oos,
        } = &points[k];
        let _ = write!(
            out,
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" style="stroke:{colour};stroke-width:1.5;stroke-dasharray:3 3;opacity:.7"/><circle cx="{:.1}" cy="{:.1}" r="5" style="fill:var(--surface);stroke:{colour};stroke-width:2"><title>{}</title></circle><circle class="dot" cx="{:.1}" cy="{:.1}" r="6" style="fill:{colour}"><title>{}</title></circle><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{ly:.1}" style="stroke:var(--grid);stroke-width:1"/><text class="lab" x="{:.1}" y="{:.1}">{}</text>"#,
            x(ins.0),
            y(ins.1),
            x(oos.0),
            y(oos.1),
            x(ins.0),
            y(ins.1),
            esc(&format!(
                "{name}, in sample\nReturn {}, volatility {}",
                pct(Some(ins.1), 1, false),
                pct(Some(ins.0), 1, false)
            )),
            x(oos.0),
            y(oos.1),
            esc(&format!(
                "{name}, out of sample\nReturn {}, volatility {}",
                pct(Some(oos.1), 1, false),
                pct(Some(oos.0), 1, false)
            )),
            x(oos.0) + 8.0,
            y(oos.1),
            w - r + 6.0,
            w - r + 12.0,
            ly + 4.0,
            esc(name)
        );
    }
    out.push_str("</svg>");
    out
}

/// Weights over time, stacked to one.
fn stacked(series: &[(&Series, &str)]) -> String {
    let Some((first, _)) = series.first() else {
        return String::new();
    };
    let n = first.dates.len();
    if n < 2 {
        return String::new();
    }
    let (w, h, l, r, t, b) = (880.0, 240.0, 56.0, 24.0, 12.0, 28.0);
    let Some(axis) = Axis::new(first.dates.iter().copied(), l, w - l - r) else {
        return String::new();
    };
    let totals: Vec<f64> = (0..n)
        .map(|i| series.iter().map(|(s, _)| s.values[i].max(0.0)).sum())
        .collect();
    let top = totals.iter().copied().fold(1.0, f64::max);
    let y = |v: f64| t + (h - t - b) * (1.0 - v / top);
    let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
    for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(v),
            y(v),
            l - 8.0,
            y(v) + 4.0,
            esc(&pct(Some(v), 0, false))
        );
    }
    out.push_str(&axis.labels(h));
    let mut below = vec![0.0; n];
    for (s, colour) in series {
        let above: Vec<f64> = below
            .iter()
            .zip(&s.values)
            .map(|(b, v)| b + v.max(0.0))
            .collect();
        let upper: Vec<String> = (0..n)
            .map(|i| format!("{:.1},{:.1}", axis.x(first.dates[i]), y(above[i])))
            .collect();
        let lower: Vec<String> = (0..n)
            .rev()
            .map(|i| format!("{:.1},{:.1}", axis.x(first.dates[i]), y(below[i])))
            .collect();
        let _ = write!(
            out,
            r#"<path d="M{} L{} Z" style="fill:{colour};stroke:var(--surface);stroke-width:1"/>"#,
            upper.join(" L"),
            lower.join(" L")
        );
        below = above;
    }
    let bands = 110.min(n);
    for k in 0..bands {
        let (i0, i1) = (k * n / bands, ((k + 1) * n / bands).min(n - 1));
        let i = ((i0 + i1) / 2).min(n - 1);
        let mut tip = month(first.dates[i]);
        for (s, _) in series {
            let _ = write!(
                tip,
                "\n{}: {}",
                s.name,
                pct(s.values.get(i).copied(), 1, false)
            );
        }
        let x0 = axis.x(first.dates[i0]);
        let x1 = axis.x(first.dates[i1]);
        let _ = write!(
            out,
            r#"<rect class="hit" x="{x0:.1}" y="{t}" width="{:.1}" height="{}"><title>{}</title></rect>"#,
            (x1 - x0).max(1.0) + 0.5,
            h - t - b,
            esc(&tip)
        );
    }
    out.push_str("</svg>");
    let keys: Vec<(String, &str)> = series.iter().map(|(s, c)| (s.name.clone(), *c)).collect();
    legend(&keys) + &out
}

/// The style of a cell shaded by weight, darker for larger, up to `scale`.
pub(crate) fn weight_style(v: f64, scale: f64) -> String {
    let p = if scale > 0.0 {
        (v / scale * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let ink = if p > 58.0 { "#fff" } else { "var(--ink)" };
    format!(r#"style="background:color-mix(in oklab, var(--pos) {p:.0}%, var(--mid));color:{ink}""#)
}

/// One row per asset and one column per refit, as wide as the time the weights were held
/// until the next refit or `end`, shaded by weight.
fn weight_strip(weights: &[WeightsAt], end: Date, rows: &[String], scale: f64) -> String {
    if weights.is_empty() {
        return String::new();
    }
    let value =
        |w: &WeightsAt, row: usize| -> f64 { w.weights.get(&rows[row]).copied().unwrap_or(0.0) };
    let (w, l, r, row) = (880.0, 112.0, 8.0, 15.0);
    let h = row * rows.len() as f64 + 22.0;
    let dates = weights.iter().map(|w| w.date).chain(std::iter::once(end));
    let Some(axis) = Axis::new(dates, l, w - l - r) else {
        return String::new();
    };
    let edge = |i: usize| axis.x(weights.get(i).map_or(end, |w| w.date));
    let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
    for (a, name) in rows.iter().enumerate() {
        let _ = write!(
            out,
            r#"<text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            l - 8.0,
            a as f64 * row + 11.0,
            esc(name)
        );
        for (i, wt) in weights.iter().enumerate() {
            let p = (value(wt, a) / scale * 100.0).clamp(0.0, 100.0);
            let _ = write!(
                out,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{}" style="fill:color-mix(in oklab, var(--pos) {p:.0}%, var(--mid))"/>"#,
                edge(i),
                a as f64 * row,
                (edge(i + 1) - edge(i)).max(0.5) + 0.3,
                row - 2.0
            );
        }
    }
    out.push_str(&axis.labels(h + 4.0));
    for (i, wt) in weights.iter().enumerate() {
        let mut tip = month(wt.date);
        for (a, name) in rows.iter().enumerate() {
            let v = value(wt, a);
            if v >= TINY {
                let _ = write!(tip, "\n{name}: {}", pct(Some(v), 1, false));
            }
        }
        let _ = write!(
            out,
            r#"<rect class="hit" x="{:.1}" y="0" width="{:.1}" height="{}"><title>{}</title></rect>"#,
            edge(i),
            (edge(i + 1) - edge(i)).max(0.5),
            row * rows.len() as f64,
            esc(&tip)
        );
    }
    out.push_str("</svg>");
    out
}

// ── Layout ───────────────────────────────────────────────────────────────────────────────────

/// A table whose cells are HTML already; the columns from `right_from` on are right-aligned.
pub(crate) fn table(
    head: &[String],
    rows: &[Vec<String>],
    right_from: usize,
    class: &str,
) -> String {
    let mut out = format!(r#"<div class="tw"><table class="{class}"><thead><tr>"#);
    for (k, h) in head.iter().enumerate() {
        let cls = if k >= right_from { r#" class="r""# } else { "" };
        let _ = write!(out, "<th{cls}>{}</th>", esc(h));
    }
    out.push_str("</tr></thead><tbody>");
    for row in rows {
        out.push_str("<tr>");
        for (k, c) in row.iter().enumerate() {
            let cls = if k >= right_from { r#" class="r""# } else { "" };
            let _ = write!(out, "<td{cls}>{c}</td>");
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table></div>");
    out
}

/// A strategy's name with its colour key.
pub(crate) fn key(name: &str, colour: &str) -> String {
    format!(
        r#"<span class="key"><i style="background:{colour}"></i>{}</span>"#,
        esc(name)
    )
}

pub(crate) fn section(anchor: &str, title: &str, subtitle: &str, body: &str) -> String {
    format!(
        r#"<section id="{anchor}"><header class="sh"><h2>{}</h2><p>{}</p></header>{body}</section>"#,
        esc(title),
        esc(subtitle)
    )
}

/// A table of label and text rows, for the method notes.
pub(crate) fn notes_table(rows: &[(&str, String)]) -> String {
    let mut out = String::from("<table><tbody>");
    for (label, text) in rows {
        let _ = write!(
            out,
            r#"<tr><th style="white-space:nowrap">{}</th><td>{}</td></tr>"#,
            esc(label),
            esc(text)
        );
    }
    out.push_str("</tbody></table>");
    out
}

/// The parts of a report page around its sections.
pub(crate) struct Frame<'a> {
    pub(crate) title: &'a str,
    pub(crate) brand: Option<&'a str>,
    pub(crate) meta: &'a str,
    pub(crate) nav: &'a [(&'a str, &'a str)],
    pub(crate) version: &'a str,
    pub(crate) digest: &'a str,
}

/// A report page: the frame around `body`.
pub(crate) fn frame(f: &Frame<'_>, body: &str) -> String {
    let Frame {
        title,
        brand,
        meta,
        nav,
        version,
        digest,
    } = *f;
    let nav: String = nav
        .iter()
        .map(|(anchor, label)| format!(r##"<a href="#{anchor}">{}</a>"##, esc(label)))
        .collect();
    let brand = brand.map_or_else(String::new, |b| {
        format!(r#"<div class="brand">{}</div>"#, esc(b))
    });
    let digest: String = digest.chars().take(12).collect();
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="generator" content="Finalytics {version}"><title>{title}</title><style>{CSS}</style></head>
<body><div class="page">
<div class="top"><div><div class="logo"><img class="lg-light" alt="Finalytics" src="data:image/png;base64,{light}"><img class="lg-dark" alt="Finalytics" src="data:image/png;base64,{dark}"></div>
<h1>{title}</h1>{brand}</div>
<div class="meta">{meta}</div></div>
<nav>{nav}</nav>
{body}
<div class="foot">Every figure is computed by Finalytics {version} from the assets' returns; the same inputs give the same report, identified by the input digest <code>{digest}</code>.</div>
</div></body></html>
"#,
        version = esc(version),
        title = esc(title),
        light = base64(LOGO_LIGHT),
        dark = base64(LOGO_DARK),
    )
}

/// The name of a calendar year, with the part of it covered when not all of it is.
fn year_label(y: &CalendarReturn) -> String {
    if y.complete {
        return y.year.to_string();
    }
    let starts = y.start.month() > 1;
    let ends = y.end.month() < 12 || y.end.day() < 24;
    match (starts, ends) {
        (true, true) => format!(
            "{} {} to {}",
            y.year,
            month_name(y.start.month()),
            month_name(y.end.month())
        ),
        (true, false) => format!("{} from {}", y.year, month_name(y.start.month())),
        (false, true) => format!("{} to {}", y.year, month_name(y.end.month())),
        (false, false) => y.year.to_string(),
    }
}

/// The value of each run's statistic, with the best of the strategies (not the benchmark)
/// in bold.
fn bold_best(
    values: &[Option<f64>],
    strategies: usize,
    higher: bool,
    format: &dyn Fn(f64) -> String,
) -> Vec<String> {
    let best = values[..strategies.min(values.len())]
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .reduce(|a, b| if (b > a) == higher { b } else { a });
    values
        .iter()
        .enumerate()
        .map(|(k, v)| match v {
            Some(v) if v.is_finite() => {
                let text = esc(&format(*v));
                if k < strategies && strategies > 1 && Some(*v) == best {
                    format!("<b>{text}</b>")
                } else {
                    text
                }
            }
            _ => "–".to_owned(),
        })
        .collect()
}

pub(crate) fn percent(v: f64) -> String {
    pct(Some(v), 1, false)
}

pub(crate) fn ratio(v: f64) -> String {
    num(Some(v), 2)
}

pub(crate) fn level(v: f64) -> String {
    num(Some(v), 0)
}

pub(crate) fn level_tick(v: f64) -> String {
    num(Some(v), 0)
}

fn pct_tick(v: f64) -> String {
    axis_pct(v, false)
}

fn ratio_tick(v: f64) -> String {
    num(Some(v), 1)
}

/// The notes on how a backtest was run: assets, validation, estimates, limits, rebalancing,
/// costs and ratios.
fn method_rows(
    c: &BacktestContext,
    optimised: bool,
    in_sample: bool,
) -> Vec<(&'static str, String)> {
    let period = period_word(c.periods_per_year);
    let assets = if c.assets.len() <= 8 {
        and_list(&c.assets)
    } else {
        plural(c.assets.len(), "asset")
    };
    let cash = c.risk_free_asset.as_ref().map_or_else(String::new, |a| {
        format!("; {a} as the risk-free asset, held as cash")
    });
    let mut rows = vec![(
        "Assets",
        format!(
            "{assets}{cash}. {} returns, {}.",
            capitalise(frequency(c.periods_per_year)),
            span(c.start, c.end)
        ),
    )];
    let validation = match &c.settings.validation {
        Validation::InSample => {
            "In sample: weights fitted once to the whole history and scored on it, \
             with knowledge of the very returns they are scored on."
                .to_owned()
        }
        Validation::Holdout { .. } => format!(
            "A holdout: weights fitted to the history to {} and scored on {}, returns they had not seen.",
            month(c.fit_end.unwrap_or(c.start)),
            span(c.test_start, c.test_end)
        ),
        Validation::WalkForward { expanding, .. } => {
            let window = c.window.map_or_else(String::new, |w| plural(w, period));
            let fitted = if *expanding {
                format!("all the returns so far (at first, {window})")
            } else {
                format!("the previous {window} of returns")
            };
            format!(
                "Walk-forward. At each rebalance from {}, each method is fitted to {fitted} and its \
                 weights are held until the next. Every return scored is one the method had not \
                 seen when it chose its weights.",
                month(c.test_start)
            )
        }
        _ => "As set.".to_owned(),
    };
    rows.push(("Validation", validation));
    if optimised {
        rows.push((
            "Estimates",
            format!(
                "{} covariance and historical mean returns, annualised by {}.",
                capitalise(covariance_name(&c.covariance)),
                num(Some(c.periods_per_year), 0)
            ),
        ));
        rows.push((
            "Limits",
            if c.limits.is_empty() {
                "Fully invested and long only.".to_owned()
            } else {
                format!("{}; fully invested.", c.limits.join("; "))
            },
        ));
    }
    rows.push(("Rebalancing", rebalancing_sentence(&c.settings.rebalance)));
    rows.push((
        "Trading costs",
        if c.settings.trading_cost_bps > 0.0 {
            format!(
                "{} bp of the value traded, paid from the next period's return.",
                num(Some(c.settings.trading_cost_bps), 1).trim_end_matches(".0")
            )
        } else {
            "None.".to_owned()
        },
    ));
    if in_sample {
        let text = match c.settings.validation {
            Validation::WalkForward { .. } => {
                "The same methods fitted once to all the history and held over the same periods, \
                 for comparison."
                    .to_owned()
            }
            _ => format!(
                "The same weights over the history they were fitted to, {}.",
                span(c.start, c.fit_end.unwrap_or(c.start))
            ),
        };
        rows.push(("In sample", text));
    }
    rows.push((
        "Ratios",
        format!(
            "Sharpe and Sortino ratios against the {} return over each period scored ({} a year over the test); value at risk and expected shortfall from history, at 95% over one {period}.",
            c.risk_free_asset.as_deref().unwrap_or("risk-free"),
            percent(c.risk_free_rate)
        ),
    ));
    rows
}

// ── The comparison report ────────────────────────────────────────────────────────────────────

impl ComparisonReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own):
    /// the methods side by side, their growth, returns by year, drawdowns and rolling Sharpe
    /// ratios, in sample against out of sample, their weights, and how they were tested.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let name = name.or(self.name.as_deref());
        let c = &self.context;
        let tested = tested_label(c);
        let range = span(c.test_start, c.test_end);
        let mut entries: Vec<(&StrategyReport, &str)> = self
            .strategies
            .iter()
            .enumerate()
            .map(|(k, s)| (s, colour(k)))
            .collect();
        let n = entries.len();
        if let Some(b) = &self.benchmark {
            entries.push((b, BENCHMARK));
        }
        let strategies: Vec<&StrategyReport> = self.strategies.iter().collect();
        let compared = !strategies.is_empty() && strategies.iter().all(|s| s.in_sample.is_some());
        let sharpe = |s: &StrategyReport| s.tested.statistics.sharpe_ratio.unwrap_or(f64::NAN);

        // Summary.
        let mut items: Vec<(String, String, Option<String>)> = Vec::new();
        let mut by_sharpe: Vec<&StrategyReport> = strategies.clone();
        by_sharpe.sort_by(|a, b| sharpe(b).total_cmp(&sharpe(a)));
        if let Some(top) = by_sharpe.first() {
            let sub = by_sharpe.get(1).map_or_else(
                || top.name.clone(),
                |next| {
                    format!(
                        "{}; next {} {}",
                        top.name,
                        lower_name(&next.name),
                        ratio(sharpe(next))
                    )
                },
            );
            items.push((
                "Highest Sharpe ratio".to_owned(),
                ratio(sharpe(top)),
                Some(sub),
            ));
        }
        if let Some(s) = strategies.iter().max_by(|a, b| {
            a.tested
                .statistics
                .max_drawdown
                .total_cmp(&b.tested.statistics.max_drawdown)
        }) {
            items.push((
                "Smallest drawdown".to_owned(),
                percent(s.tested.statistics.max_drawdown),
                Some(s.name.clone()),
            ));
        }
        if let Some(s) = strategies.iter().min_by(|a, b| {
            a.tested
                .turnover_per_year
                .total_cmp(&b.tested.turnover_per_year)
        }) {
            items.push((
                "Lowest turnover".to_owned(),
                format!("{} a year", pct(Some(s.tested.turnover_per_year), 0, false)),
                Some(s.name.clone()),
            ));
        }
        if compared {
            let fall = |s: &StrategyReport| {
                s.in_sample
                    .as_ref()
                    .and_then(|i| i.statistics.sharpe_ratio)
                    .map_or(f64::NAN, |i| i - sharpe(s))
            };
            if let Some(s) = strategies.iter().max_by(|a, b| fall(a).total_cmp(&fall(b))) {
                let before = s.in_sample.as_ref().and_then(|i| i.statistics.sharpe_ratio);
                items.push((
                    format!("Largest fall {}", lower_first(tested)),
                    format!("{} → {}", num(before, 2), ratio(sharpe(s))),
                    Some(format!("{}, Sharpe ratio", s.name)),
                ));
            }
        } else if let Some(s) = strategies.iter().max_by(|a, b| {
            a.tested
                .statistics
                .annualised_return
                .total_cmp(&b.tested.statistics.annualised_return)
        }) {
            items.push((
                "Highest return".to_owned(),
                format!("{} a year", percent(s.tested.statistics.annualised_return)),
                Some(s.name.clone()),
            ));
        }
        let column = |f: &dyn Fn(&RunReport) -> Option<f64>| -> Vec<Option<f64>> {
            entries.iter().map(|(s, _)| f(&s.tested)).collect()
        };
        let cols = [
            bold_best(
                &column(&|r| Some(r.statistics.annualised_return)),
                n,
                true,
                &percent,
            ),
            bold_best(
                &column(&|r| Some(r.statistics.volatility)),
                n,
                false,
                &percent,
            ),
            bold_best(&column(&|r| r.statistics.sharpe_ratio), n, true, &ratio),
            bold_best(&column(&|r| r.statistics.sortino_ratio), n, true, &ratio),
            bold_best(
                &column(&|r| Some(r.statistics.max_drawdown)),
                n,
                true,
                &percent,
            ),
            bold_best(
                &column(&|r| Some(-r.tail_risk.expected_shortfall)),
                n,
                true,
                &|v| percent(-v),
            ),
            bold_best(&column(&|r| Some(r.turnover_per_year)), n, false, &|v| {
                pct(Some(v), 0, false)
            }),
        ];
        let rows: Vec<Vec<String>> = entries
            .iter()
            .enumerate()
            .map(|(k, (s, col))| {
                let mut row = vec![key(&s.name, col)];
                row.extend(cols.iter().map(|c| c[k].clone()));
                row
            })
            .collect();
        let period = period_word(c.periods_per_year);
        let mut note = format!(
            "Return, volatility and turnover are a year; drawdown is the largest fall; CVaR is the \
             average loss in the worst 5% of {period}s.{}",
            if n > 1 {
                " Bold marks the best in each column."
            } else {
                ""
            }
        );
        if !c.limits.is_empty() {
            let _ = write!(note, " Limits: {}.", limits_sentence(c));
        }
        for line in grouped_notes(&strategies)
            .into_iter()
            .chain(failure_notes(&strategies))
        {
            let _ = write!(note, " {line}");
        }
        let mut summary = tiles(&items);
        summary.push_str(&card(
            &format!("{tested}, {range}"),
            &table(
                &[
                    "Method",
                    "Return",
                    "Volatility",
                    "Sharpe",
                    "Sortino",
                    "Drawdown",
                    "CVaR 95%",
                    "Turnover",
                ]
                .map(str::to_owned),
                &rows,
                1,
                "cmp",
            ),
            Some(&note),
        ));

        // Performance.
        let growth: Vec<Line<'_>> = entries
            .iter()
            .map(|(s, col)| Line {
                name: &s.name,
                colour: col,
                series: &s.tested.growth,
            })
            .collect();
        let mut performance = card(
            "Growth of 100",
            &lines(&growth, &level, &level_tick, 280.0, false, None, None),
            None,
        );
        if let Some((first, _)) = entries.first() {
            let years: Vec<&CalendarReturn> = first.tested.years.iter().collect();
            let scale = entries
                .iter()
                .flat_map(|(s, _)| s.tested.years.iter().map(|y| y.value.abs()))
                .fold(0.0, f64::max);
            let mut grid =
                String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
            for (s, _) in &entries {
                let _ = write!(grid, "<th>{}</th>", esc(&s.name));
            }
            grid.push_str("</tr></thead><tbody>");
            for y in &years {
                let _ = write!(grid, r#"<tr><th class="yr">{}</th>"#, esc(&year_label(y)));
                for (s, _) in &entries {
                    let v = s
                        .tested
                        .years
                        .iter()
                        .find(|z| z.year == y.year)
                        .map(|z| z.value);
                    let text = pct(v, 1, false);
                    let _ = write!(
                        grid,
                        r#"<td {} title="{} · {}: {}">{}</td>"#,
                        heat(v, scale),
                        esc(&s.name),
                        y.year,
                        esc(&text),
                        esc(&text)
                    );
                }
                grid.push_str("</tr>");
            }
            grid.push_str("</tbody></table></div>");
            performance.push_str(&card("Calendar-year returns", &grid, None));
        }

        // Risk.
        let falls: Vec<Line<'_>> = entries
            .iter()
            .map(|(s, col)| Line {
                name: &s.name,
                colour: col,
                series: &s.tested.drawdown,
            })
            .collect();
        let worst: Vec<String> = entries
            .iter()
            .map(|(s, _)| format!("{} at worst", percent(s.tested.statistics.max_drawdown)))
            .collect();
        let mut risk = card(
            "Drawdown",
            &lines(&falls, &percent, &pct_tick, 240.0, true, None, Some(&worst)),
            None,
        );
        let rolling: Vec<Line<'_>> = entries
            .iter()
            .filter_map(|(s, col)| {
                s.tested.rolling_sharpe.as_ref().map(|series| Line {
                    name: &s.name,
                    colour: col,
                    series,
                })
            })
            .collect();
        if !rolling.is_empty() {
            let last: Vec<String> = rolling
                .iter()
                .map(|l| {
                    l.series
                        .values
                        .last()
                        .map_or_else(String::new, |v| ratio(*v))
                })
                .collect();
            risk.push_str(&card(
                "Sharpe ratio over the past 3 years",
                &lines(
                    &rolling,
                    &ratio,
                    &ratio_tick,
                    240.0,
                    true,
                    None,
                    Some(&last),
                ),
                None,
            ));
        }

        // In and out of sample.
        let mut robustness = String::new();
        if compared {
            let points: Vec<Point<'_>> = entries
                .iter()
                .take(n)
                .filter_map(|(s, col)| {
                    let i = s.in_sample.as_ref()?;
                    Some(Point {
                        name: s.name.as_str(),
                        colour: col,
                        in_sample: (i.statistics.volatility, i.statistics.annualised_return),
                        tested: (
                            s.tested.statistics.volatility,
                            s.tested.statistics.annualised_return,
                        ),
                    })
                })
                .collect();
            let explain = match c.settings.validation {
                Validation::WalkForward { .. } => {
                    "In sample, each method is fitted to the whole history and scored on the same \
                     periods: it sees the returns it is scored on, so it looks better than it can \
                     be. The gap to the out-of-sample point is the cost of estimating from the past."
                        .to_owned()
                }
                _ => format!(
                    "In sample is the history the weights were fitted to, {}; out of sample is {}.",
                    span(c.start, c.fit_end.unwrap_or(c.start)),
                    range
                ),
            };
            let legend_html = r#"<div class="legend"><span class="key"><i class="hollow"></i>In sample</span><span class="key"><i style="background:var(--ink2)"></i>Out of sample</span></div>"#;
            robustness.push_str(&card(
                "In sample and out of sample",
                &format!("{legend_html}{}", scatter(&points)),
                Some(&explain),
            ));
            let rows: Vec<Vec<String>> = entries
                .iter()
                .take(n)
                .filter_map(|(s, col)| {
                    let i = s.in_sample.as_ref()?;
                    let o = &s.tested;
                    Some(vec![
                        key(&s.name, col),
                        esc(&num(i.statistics.sharpe_ratio, 2)),
                        esc(&num(o.statistics.sharpe_ratio, 2)),
                        esc(
                            &match (i.statistics.sharpe_ratio, o.statistics.sharpe_ratio) {
                                (Some(a), Some(b)) => num(Some(b - a), 2),
                                _ => "–".to_owned(),
                            },
                        ),
                        esc(&percent(i.statistics.annualised_return)),
                        esc(&percent(o.statistics.annualised_return)),
                        esc(&percent(i.statistics.max_drawdown)),
                        esc(&percent(o.statistics.max_drawdown)),
                    ])
                })
                .collect();
            robustness.push_str(&card(
                "What each method promised and delivered",
                &table(
                    &[
                        "Method",
                        "Sharpe in",
                        "Sharpe out",
                        "Change",
                        "Return in",
                        "Return out",
                        "Drawdown in",
                        "Drawdown out",
                    ]
                    .map(str::to_owned),
                    &rows,
                    1,
                    "cmp",
                ),
                None,
            ));
        }

        // Weights.
        let latest: Vec<(&StrategyReport, &WeightsAt)> = strategies
            .iter()
            .filter_map(|s| s.latest().map(|w| (*s, w)))
            .collect();
        let mut weights_html = String::new();
        if let Some((_, first)) = latest.first() {
            let maps: Vec<&BTreeMap<String, f64>> = strategies
                .iter()
                .flat_map(|s| s.weights.iter().map(|w| &w.weights))
                .collect();
            let (assets, rest) = shown_assets(c, &maps);
            let scale = maps
                .iter()
                .flat_map(|m| m.values().copied())
                .fold(0.0, f64::max);
            let value = |w: &WeightsAt, asset: Option<&String>| -> f64 {
                match asset {
                    Some(a) => w.weights.get(a).copied().unwrap_or(0.0),
                    None => rest
                        .iter()
                        .map(|a| w.weights.get(a).copied().unwrap_or(0.0))
                        .sum(),
                }
            };
            let mut columns: Vec<Option<&String>> = assets.iter().map(Some).collect();
            if !rest.is_empty() {
                columns.push(None);
            }
            let label = |a: Option<&String>| {
                a.cloned()
                    .unwrap_or_else(|| format!("{} others", rest.len()))
            };
            let mut grid =
                String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
            // Methods down the side when there are few assets, and across the top otherwise.
            let across = columns.len() <= 8;
            if across {
                for a in &columns {
                    let _ = write!(grid, "<th>{}</th>", esc(&label(*a)));
                }
                grid.push_str("</tr></thead><tbody>");
                for (s, w) in &latest {
                    let _ = write!(grid, "<tr><th>{}</th>", esc(&s.name));
                    for a in &columns {
                        let v = value(w, *a);
                        let _ = write!(
                            grid,
                            "<td {}>{}</td>",
                            weight_style(v, scale),
                            esc(&weight_text(v))
                        );
                    }
                    grid.push_str("</tr>");
                }
            } else {
                for (s, _) in &latest {
                    let _ = write!(grid, "<th>{}</th>", esc(&s.name));
                }
                grid.push_str("</tr></thead><tbody>");
                for a in &columns {
                    let _ = write!(grid, "<tr><th>{}</th>", esc(&label(*a)));
                    for (_, w) in &latest {
                        let v = value(w, *a);
                        let _ = write!(
                            grid,
                            "<td {}>{}</td>",
                            weight_style(v, scale),
                            esc(&weight_text(v))
                        );
                    }
                    grid.push_str("</tr>");
                }
            }
            grid.push_str("</tbody></table></div>");
            let title = if latest.iter().all(|(s, _)| s.weights.len() == 1) {
                format!("Weights, fitted to {}", month(first.date))
            } else {
                format!("Weights at the last refit, {}", month(first.date))
            };
            weights_html.push_str(&card(&title, &grid, None));
            let changing: Vec<&&StrategyReport> = strategies
                .iter()
                .filter(|s| {
                    s.weights.len() > 1
                        && s.weights.windows(2).any(|p| p[0].weights != p[1].weights)
                })
                .collect();
            if !changing.is_empty() {
                let mut strips = String::new();
                for s in &changing {
                    let _ = write!(
                        strips,
                        r#"<h3 style="margin:14px 0 6px;font-size:14px">{} <span style="color:var(--muted);font-weight:400">· turnover {} a year</span></h3>"#,
                        esc(&s.name),
                        esc(&pct(Some(s.tested.turnover_per_year), 0, false))
                    );
                    strips.push_str(&weight_strip(&s.weights, s.tested.end, &assets, scale));
                }
                let mut note = format!(
                    "Darker is a larger weight, up to {}.",
                    pct(Some(scale), 0, false)
                );
                if !rest.is_empty() {
                    let _ = write!(
                        note,
                        " The {} assets with the largest weights are shown; {} others are left out.",
                        assets.len(),
                        rest.len()
                    );
                }
                let steady: Vec<String> = strategies
                    .iter()
                    .filter(|s| !changing.iter().any(|x| x.name == s.name))
                    .map(|s| s.name.clone())
                    .collect();
                if !steady.is_empty() {
                    let _ = write!(
                        note,
                        " {} kept the same weights throughout.",
                        and_list(&steady)
                    );
                }
                if c.settings.trading_cost_bps > 0.0 {
                    let costs: Vec<String> = strategies
                        .iter()
                        .map(|s| {
                            format!(
                                "{} {}",
                                s.name,
                                pct(Some(s.tested.costs_per_year), 2, false)
                            )
                        })
                        .collect();
                    let _ = write!(note, " Trading costs a year: {}.", costs.join("; "));
                }
                weights_html.push_str(&card("Weights at each refit", &strips, Some(&note)));
            }
        }

        // Method.
        let optimised = !strategies.is_empty();
        let method = card(
            "How the methods were tested",
            &notes_table(&method_rows(c, optimised, compared)),
            None,
        );

        let mut nav = vec![
            ("c-summary", "Summary"),
            ("c-performance", "Performance"),
            ("c-risk", "Risk"),
        ];
        let mut body = section(
            "c-summary",
            "Summary",
            &format!("Each method, scored {}", lower_first(tested)),
            &summary,
        );
        body.push_str(&section(
            "c-performance",
            "Performance",
            &format!("Growth and returns by year, {}", lower_first(tested)),
            &performance,
        ));
        body.push_str(&section(
            "c-risk",
            "Risk",
            "How far and how long each method fell",
            &risk,
        ));
        if compared {
            nav.push(("c-robustness", "In and out of sample"));
            body.push_str(&section(
                "c-robustness",
                "In and out of sample",
                "How much of each method's promise survived",
                &robustness,
            ));
        }
        if !weights_html.is_empty() {
            nav.push(("c-weights", "Weights"));
            body.push_str(&section(
                "c-weights",
                "Weights",
                "What each method held, and how often it traded",
                &weights_html,
            ));
        }
        nav.push(("c-method", "Method"));
        body.push_str(&section(
            "c-method",
            "Method",
            "How the comparison was run",
            &method,
        ));
        let meta = format!(
            "{}, {range}<br>{} · {} · {} · {} · {}{}",
            esc(&validation(c)),
            esc(&plural(n, "method")),
            esc(&plural(c.assets.len(), "asset")),
            frequency(c.periods_per_year),
            esc(&rebalancing(&c.settings.rebalance)),
            esc(&costs(c.settings.trading_cost_bps)),
            c.risk_free_asset
                .as_ref()
                .map_or_else(String::new, |a| format!(" · risk-free {}", esc(a)))
        );
        let title = name.unwrap_or("Comparison report");
        let brand = name.map(|_| "Comparison report");
        frame(
            &Frame {
                title,
                brand,
                meta: &meta,
                nav: &nav,
                version: &c.version,
                digest: &c.digest,
            },
            &body,
        )
    }
}

/// A weight as shown in a table: none when tiny.
pub(crate) fn weight_text(v: f64) -> String {
    if v.abs() < TINY {
        "–".to_owned()
    } else {
        pct(Some(v), 0, false)
    }
}

// ── The backtest report ──────────────────────────────────────────────────────────────────────

impl BacktestReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own, or
    /// the strategy's): the allocation and its statistics against the benchmark, growth and
    /// drawdown, the weights held and the cost of trading, and returns by year and month.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let s = &self.strategy;
        let title = name.or(self.name.as_deref()).unwrap_or(&s.name);
        let bench = self.benchmark.as_ref();
        let tested = tested_label(c);
        let period = period_word(c.periods_per_year);
        let split = c.settings.validation != Validation::InSample;

        // Summary: the allocation.
        let mut summary = String::new();
        if let Some(w) = s.latest() {
            // In the order, and the colours, of the weights held, then by size.
            let rank = |a: &str| {
                self.held
                    .iter()
                    .position(|h| h.name == a)
                    .unwrap_or(usize::MAX)
            };
            let mut parts: Vec<(&String, f64)> = w
                .weights
                .iter()
                .map(|(a, v)| (a, *v))
                .filter(|(_, v)| *v >= 0.005)
                .collect();
            parts.sort_by(|a, b| rank(a.0).cmp(&rank(b.0)).then(b.1.total_cmp(&a.1)));
            let cash = 1.0 - w.weights.values().sum::<f64>();
            let mut segs = String::new();
            let mut keys: Vec<(String, &str)> = Vec::new();
            for (k, (asset, v)) in parts.iter().enumerate() {
                let slot = match rank(asset) {
                    usize::MAX => k,
                    r => r,
                };
                let col = if slot < SERIES.len() - 1 {
                    colour(slot)
                } else {
                    "var(--muted-fill)"
                };
                let inside = if *v >= 0.07 {
                    pct(Some(*v), 0, false)
                } else {
                    String::new()
                };
                let _ = write!(
                    segs,
                    r#"<div class="seg" style="flex:{v:.4};background:{col}" title="{}: {}"><span>{inside}</span></div>"#,
                    esc(asset),
                    esc(&pct(Some(*v), 1, false))
                );
                keys.push((format!("{asset} {}", pct(Some(*v), 1, false)), col));
            }
            if cash >= 0.005 {
                let name = c
                    .risk_free_asset
                    .clone()
                    .unwrap_or_else(|| "Cash".to_owned());
                let _ = write!(
                    segs,
                    r#"<div class="seg" style="flex:{cash:.4};background:var(--muted-fill)" title="{}: {}"><span></span></div>"#,
                    esc(&name),
                    esc(&pct(Some(cash), 1, false))
                );
                keys.push((
                    format!("{name} {}", pct(Some(cash), 1, false)),
                    "var(--muted-fill)",
                ));
            }
            let fitted = s.weights.len() > 1 || s.in_sample.is_some() || !c.limits.is_empty();
            let how = match c.settings.validation {
                _ if !fitted => format!("{}: fixed weights.", s.name),
                Validation::InSample => format!(
                    "{}, fitted to the returns from {}.",
                    s.name,
                    span(c.start, c.end)
                ),
                Validation::Holdout { .. } => format!(
                    "{}, fitted to the returns from {}.",
                    s.name,
                    span(c.start, c.fit_end.unwrap_or(c.start))
                ),
                _ => format!("{}, as refitted at {}.", s.name, month(w.date)),
            };
            let mut note = how;
            if !c.limits.is_empty() {
                let _ = write!(note, " Limits: {}.", limits_sentence(c));
            }
            let _ = write!(
                note,
                " {}.",
                capitalise(&rebalancing(&c.settings.rebalance))
            );
            if let Some(b) = bench {
                let mut parts: Vec<(&String, f64)> = b
                    .weights
                    .first()
                    .map(|w| {
                        w.weights
                            .iter()
                            .map(|(a, v)| (a, *v))
                            .filter(|(_, v)| *v >= TINY)
                            .collect()
                    })
                    .unwrap_or_default();
                parts.sort_by(|x, y| y.1.total_cmp(&x.1));
                let held: Vec<String> = parts
                    .iter()
                    .map(|(a, v)| format!("{} {a}", pct(Some(*v), 0, false)))
                    .collect();
                if held.is_empty() {
                    let _ = write!(note, " Benchmark: {}.", b.name);
                } else {
                    let _ = write!(note, " {}: {}, held the same way.", b.name, and_list(&held));
                }
            }
            for line in grouped_notes(&[s]).into_iter().chain(failure_notes(&[s])) {
                let _ = write!(note, " {line}");
            }
            let title = match c.settings.validation {
                Validation::WalkForward { .. } => {
                    format!("The allocation at the last refit, {}", month(w.date))
                }
                _ => "The allocation".to_owned(),
            };
            summary.push_str(&card(
                &title,
                &format!(
                    r#"<div class="segbar" role="img" aria-label="{}">{segs}</div>{}"#,
                    esc(&title),
                    legend(&keys)
                ),
                Some(&note),
            ));
        }
        let o = &s.tested;
        let i = s.in_sample.as_ref();
        let bo = bench.map(|b| &b.tested);
        let bi = bench.and_then(|b| b.in_sample.as_ref());
        let suffix = if split {
            format!(", {}", lower_first(tested))
        } else {
            String::new()
        };
        let mut items = vec![(
            format!("Return{suffix}"),
            format!("{} a year", percent(o.statistics.annualised_return)),
            bo.map(|b| {
                format!(
                    "{} {}",
                    bench.map_or("", |b| b.name.as_str()),
                    percent(b.statistics.annualised_return)
                )
            }),
        )];
        items.push((
            "Volatility".to_owned(),
            percent(o.statistics.volatility),
            bo.map(|b| {
                format!(
                    "{} {}",
                    bench.map_or("", |b| b.name.as_str()),
                    percent(b.statistics.volatility)
                )
            }),
        ));
        let mut sharpe_sub: Vec<String> = Vec::new();
        if let Some(i) = i {
            sharpe_sub.push(format!("{} in sample", num(i.statistics.sharpe_ratio, 2)));
        }
        if let Some(b) = bo {
            sharpe_sub.push(format!(
                "{} {}",
                bench.map_or("", |b| b.name.as_str()),
                num(b.statistics.sharpe_ratio, 2)
            ));
        }
        items.push((
            "Sharpe ratio".to_owned(),
            num(o.statistics.sharpe_ratio, 2),
            (!sharpe_sub.is_empty()).then(|| sharpe_sub.join(" · ")),
        ));
        items.push((
            "Maximum drawdown".to_owned(),
            percent(o.statistics.max_drawdown),
            bo.map(|b| {
                format!(
                    "{} {}",
                    bench.map_or("", |b| b.name.as_str()),
                    percent(b.statistics.max_drawdown)
                )
            }),
        ));
        if let Some(rel) = &o.relative {
            items.push((
                "Tracking error".to_owned(),
                percent(rel.tracking_error),
                Some(format!(
                    "Information ratio {}",
                    num(rel.information_ratio, 2)
                )),
            ));
        }
        summary.push_str(&tiles(&items));
        // The statistics table.
        let mut runs: Vec<(String, &RunReport)> = Vec::new();
        match i {
            Some(i) => {
                runs.push((format!("{}, in sample", s.name), i));
                runs.push((format!("{}, {}", s.name, lower_first(tested)), o));
            }
            None => runs.push((s.name.clone(), o)),
        }
        if let Some(b) = bench {
            match (bi, bo) {
                (Some(bi), Some(bo)) => {
                    runs.push((format!("{}, in sample", b.name), bi));
                    runs.push((format!("{}, {}", b.name, lower_first(tested)), bo));
                }
                (None, Some(bo)) => runs.push((b.name.clone(), bo)),
                _ => {}
            }
        }
        let row = |label: &str, f: &dyn Fn(&RunReport) -> String| -> Vec<String> {
            std::iter::once(esc(label))
                .chain(runs.iter().map(|(_, r)| esc(&f(r))))
                .collect()
        };
        let mut rows = vec![
            row("Return a year", &|r| {
                percent(r.statistics.annualised_return)
            }),
            row("Volatility", &|r| percent(r.statistics.volatility)),
            row("Sharpe ratio", &|r| num(r.statistics.sharpe_ratio, 2)),
            row("Sortino ratio", &|r| num(r.statistics.sortino_ratio, 2)),
            row("Maximum drawdown", &|r| percent(r.statistics.max_drawdown)),
            row("Calmar ratio", &|r| num(r.statistics.calmar_ratio, 2)),
            row(&format!("Best {period}"), &|r| {
                pct(Some(r.statistics.best), 1, true)
            }),
            row(&format!("Worst {period}"), &|r| {
                pct(Some(r.statistics.worst), 1, true)
            }),
            row(&format!("Value at risk, 95%, 1 {period}"), &|r| {
                percent(r.tail_risk.value_at_risk)
            }),
            row(&format!("Expected shortfall, 95%, 1 {period}"), &|r| {
                percent(r.tail_risk.expected_shortfall)
            }),
        ];
        if o.relative.is_some() {
            rows.push(row("Tracking error", &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| percent(x.tracking_error))
            }));
            rows.push(row("Information ratio", &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| num(x.information_ratio, 2))
            }));
            rows.push(row("Beta to the benchmark", &|r| {
                r.relative
                    .as_ref()
                    .map_or_else(String::new, |x| num(x.beta, 2))
            }));
        }
        let head: Vec<String> = std::iter::once(String::new())
            .chain(runs.iter().map(|(n, _)| n.clone()))
            .collect();
        let note = match (&c.settings.validation, i) {
            (Validation::Holdout { .. }, Some(i)) => format!(
                "In sample: {}, the history the weights were fitted to. Out of sample: {}.",
                span(i.start, i.end),
                span(o.start, o.end)
            ),
            (Validation::WalkForward { .. }, Some(_)) => format!(
                "Out of sample: walk-forward, {}. In sample: weights fitted to the whole history, over the same periods.",
                span(o.start, o.end)
            ),
            _ => format!(
                "In sample: weights fitted to {} and scored on it.",
                span(o.start, o.end)
            ),
        };
        summary.push_str(&card(
            "Statistics",
            &table(&head, &rows, 1, ""),
            Some(&note),
        ));

        // Growth and drawdown.
        let (main, base, shade) = match &self.history {
            Some(h) => (&h.strategy, h.benchmark.as_ref(), Some(h.split)),
            None => (o, bo, None),
        };
        let bench_name = bench.map_or("Benchmark", |b| b.name.as_str());
        let mut growth_lines = vec![Line {
            name: &s.name,
            colour: colour(0),
            series: &main.growth,
        }];
        let mut fall_lines = vec![Line {
            name: &s.name,
            colour: colour(0),
            series: &main.drawdown,
        }];
        if let Some(b) = base {
            growth_lines.push(Line {
                name: bench_name,
                colour: colour(1),
                series: &b.growth,
            });
            fall_lines.push(Line {
                name: bench_name,
                colour: colour(1),
                series: &b.drawdown,
            });
        }
        let top_shade = shade.map(|at| Shade {
            at,
            left: "In sample",
            right: "Out of sample",
            bottom: false,
        });
        let bottom_shade = shade.map(|at| Shade {
            at,
            left: "In sample",
            right: "Out of sample",
            bottom: true,
        });
        let mut worst = vec![format!(
            "{} at worst{}",
            percent(o.statistics.max_drawdown),
            if split {
                format!(" {}", lower_first(tested))
            } else {
                String::new()
            }
        )];
        if let Some(b) = bo {
            worst.push(percent(b.statistics.max_drawdown));
        }
        let growth_html = card(
            "Growth of 100",
            &lines(
                &growth_lines,
                &level,
                &level_tick,
                280.0,
                false,
                top_shade.as_ref(),
                None,
            ),
            None,
        ) + &card(
            "Drawdown",
            &lines(
                &fall_lines,
                &percent,
                &pct_tick,
                220.0,
                true,
                bottom_shade.as_ref(),
                Some(&worst),
            ),
            None,
        );
        let growth_subtitle = match &self.history {
            Some(_) => "The whole history, with the holdout shaded apart".to_owned(),
            None => format!("{tested}, {}", span(o.start, o.end)),
        };

        // Weights and trading.
        let mut weights_html = String::new();
        let shown: Vec<(&Series, &str)> = self
            .held
            .iter()
            .enumerate()
            .map(|(k, series)| {
                let last = k + 1 == self.held.len();
                let cash = last
                    && c.risk_free_asset
                        .as_deref()
                        .map_or(series.name == "Cash", |a| a == series.name);
                (series, if cash { "var(--muted-fill)" } else { colour(k) })
            })
            .collect();
        if !shown.is_empty() {
            weights_html.push_str(&card(
                &format!("Weights held, {}", lower_first(tested)),
                &stacked(&shown),
                Some("Between rebalances the weights drift with prices; at each rebalance they return to the targets."),
            ));
        }
        weights_html.push_str(&tiles(&[
            (
                "Rebalances".to_owned(),
                o.rebalances.to_string(),
                Some(capitalise(&rebalancing(&c.settings.rebalance))),
            ),
            (
                "Turnover".to_owned(),
                format!("{} a year", pct(Some(o.turnover_per_year), 1, false)),
                None,
            ),
            (
                "Trading costs".to_owned(),
                format!("{} a year", pct(Some(o.costs_per_year), 2, false)),
                Some(if c.settings.trading_cost_bps > 0.0 {
                    format!(
                        "{} of the value traded",
                        costs(c.settings.trading_cost_bps).trim_end_matches(" trading costs")
                    )
                } else {
                    "None".to_owned()
                }),
            ),
            (
                "Largest rebalance".to_owned(),
                pct(Some(o.largest_trade), 1, false),
                Some("Of the portfolio's value".to_owned()),
            ),
        ]));

        // Returns.
        let years_of = match &self.history {
            Some(h) => &h.strategy.years,
            None => &o.years,
        };
        let bench_years = match &self.history {
            Some(h) => h.benchmark.as_ref().map(|b| &b.years),
            None => bo.map(|b| &b.years),
        };
        let categories: Vec<String> = years_of.iter().map(year_label).collect();
        let mut cols = vec![(
            s.name.as_str(),
            colour(0),
            years_of.iter().map(|y| Some(y.value)).collect::<Vec<_>>(),
        )];
        if let Some(by) = bench_years {
            cols.push((
                bench_name,
                colour(1),
                years_of
                    .iter()
                    .map(|y| by.iter().find(|z| z.year == y.year).map(|z| z.value))
                    .collect(),
            ));
        }
        // The first year out of sample is the first that starts out of sample.
        let split_year = self.history.as_ref().map(|h| {
            years_of
                .iter()
                .position(|y| y.start >= h.split)
                .unwrap_or(years_of.len())
        });
        let partial: Vec<String> = years_of
            .iter()
            .filter(|y| !y.complete)
            .map(year_label)
            .collect();
        let mut returns_html = card(
            "Calendar-year returns",
            &year_columns(
                &categories,
                &cols,
                240.0,
                split_year.map(|k| (k, "In sample", "Out of sample")),
            ),
            (!partial.is_empty())
                .then(|| format!("Part years: {}.", partial.join(", ")))
                .as_deref(),
        );
        let m = &o.months;
        let scale = m
            .values
            .iter()
            .flatten()
            .flatten()
            .map(|v| v.abs())
            .fold(0.0, f64::max);
        let mut grid = String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
        for col in &m.columns {
            let _ = write!(grid, "<th>{}</th>", esc(col));
        }
        grid.push_str(r#"<th class="r">Year</th></tr></thead><tbody>"#);
        for (year, values) in m.rows.iter().zip(&m.values) {
            let _ = write!(grid, r#"<tr><th class="yr">{}</th>"#, esc(year));
            for (k, v) in values.iter().enumerate() {
                let text = v.map_or_else(String::new, |v| pct(Some(v), 1, false));
                let _ = write!(
                    grid,
                    r#"<td {} title="{} {}: {}">{}</td>"#,
                    heat(*v, scale),
                    esc(m.columns.get(k).map_or("", String::as_str)),
                    esc(year),
                    esc(&text),
                    esc(&text)
                );
            }
            let total = values.iter().flatten().fold(1.0, |g, r| g * (1.0 + r)) - 1.0;
            let _ = write!(
                grid,
                r#"<td class="r"><b>{}</b></td></tr>"#,
                esc(&pct(Some(total), 1, false))
            );
        }
        grid.push_str("</tbody></table></div>");
        returns_html.push_str(&card(
            &format!("Monthly returns, {}", lower_first(tested)),
            &grid,
            None,
        ));

        let method = card(
            "How the allocation was tested",
            &notes_table(&method_rows(
                c,
                s.weights.len() > 1
                    || c.settings.validation != Validation::InSample
                    || !c.limits.is_empty(),
                i.is_some(),
            )),
            None,
        );

        let mut body = section(
            "b-summary",
            "Summary",
            &if split {
                "The allocation, and how it did on returns it had not seen".to_owned()
            } else {
                "The allocation, and how it did".to_owned()
            },
            &summary,
        );
        body.push_str(&section(
            "b-growth",
            "Growth and drawdown",
            &growth_subtitle,
            &growth_html,
        ));
        body.push_str(&section(
            "b-weights",
            "Weights and trading",
            "What drifting and rebalancing cost",
            &weights_html,
        ));
        body.push_str(&section(
            "b-returns",
            "Returns",
            "By year and by month",
            &returns_html,
        ));
        body.push_str(&section(
            "b-method",
            "Method",
            "How the backtest was run",
            &method,
        ));
        let first_line = match (&c.settings.validation, i) {
            (Validation::Holdout { .. }, Some(i)) => format!(
                "In sample {} · out of sample {}",
                span(i.start, i.end),
                span(o.start, o.end)
            ),
            _ => format!("{}, {}", validation(c), span(o.start, o.end)),
        };
        let held = s
            .latest()
            .map_or(0, |w| w.weights.values().filter(|v| **v >= 0.005).count());
        let meta = format!(
            "{}<br>{} · {} · {} · {}{}",
            esc(&first_line),
            esc(&plural(held, "asset")),
            frequency(c.periods_per_year),
            esc(&rebalancing(&c.settings.rebalance)),
            esc(&costs(c.settings.trading_cost_bps)),
            bench.map_or_else(String::new, |b| format!(" · against {}", esc(&b.name)))
        );
        frame(
            &Frame {
                title,
                brand: Some("Backtest report"),
                meta: &meta,
                nav: &[
                    ("b-summary", "Summary"),
                    ("b-growth", "Growth"),
                    ("b-weights", "Weights"),
                    ("b-returns", "Returns"),
                    ("b-method", "Method"),
                ],
                version: &c.version,
                digest: &c.digest,
            },
            &body,
        )
    }
}
