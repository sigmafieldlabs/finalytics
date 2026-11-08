//! The reports as one self-contained HTML page: inline styles and SVG charts, no scripts and no
//! external resources, in light and dark themes, ready to print.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use finalytics_core::{Date, Decimal};
use finalytics_ledger::HoldingKey;
use finalytics_opt::FrontierPoint;
use finalytics_perf::Linking;

use crate::context::Provenance;
use crate::overview::{HoldingRiskReturn, PortfolioPoint};
use crate::scenarios::ScenarioDefinition;
use crate::shapes::{Bin, Slice};
use crate::text::{
    covariance_name, date, dec, group, holding, money, money_places, num, pct, period_word, plural,
    share, signed, tail_name, words,
};
use crate::{
    AttributionReport, HoldingsReport, OverviewReport, PerformanceReport, PortfolioReport,
    RiskReport, ScenariosReport, Unit,
};

pub(crate) const LOGO_LIGHT: &[u8] = include_bytes!("../assets/logo-light.png");
pub(crate) const LOGO_DARK: &[u8] = include_bytes!("../assets/logo-dark.png");

/// The first three colours of a chart's series; any further slices of a segmented bar are muted.
const SERIES: [&str; 3] = ["var(--s1)", "var(--s2)", "var(--s3)"];

// ── Text and numbers ─────────────────────────────────────────────────────────────────────────

/// The opening tag of a chart `w` by `h` in its own units. A chart as wide as the page keeps
/// a minimum width on narrow screens, where its card scrolls, so that its text stays legible.
pub(crate) fn svg_open(w: f64, h: f64) -> String {
    let class = if w >= 600.0 { r#" class="chart""# } else { "" };
    format!(r#"<svg{class} viewBox="0 0 {w} {h}" role="img">"#)
}

/// Places the labels of points on a chart beside their points, so that no label overlaps
/// another, a point or the edge of the area given: each tries the right of its point, then the
/// left, above, below and the diagonals, and is left out when none fits.
pub(crate) struct Labels {
    /// The area labels must stay inside: left, top, right and bottom.
    area: (f64, f64, f64, f64),
    /// The boxes taken by the points and the labels placed so far.
    taken: Vec<(f64, f64, f64, f64)>,
}

impl Labels {
    /// Labels inside `area` (left, top, right, bottom) for a chart with `points` of `radius`.
    pub(crate) fn new(area: (f64, f64, f64, f64), points: &[(f64, f64)], radius: f64) -> Labels {
        Labels {
            area,
            taken: points
                .iter()
                .map(|(x, y)| (x - radius, y - radius, x + radius, y + radius))
                .collect(),
        }
    }

    /// Where to write `text` at `size` beside the point at (`x`, `y`): the position of its
    /// baseline and its anchor, or `None` when there is no room.
    pub(crate) fn place(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        size: f64,
    ) -> Option<(f64, f64, &'static str)> {
        let width = text.chars().count() as f64 * size * 0.56;
        let gap = 9.0;
        let candidates = [
            (x + gap, y + size * 0.35, "start"),
            (x - gap, y + size * 0.35, "end"),
            (x, y - gap, "middle"),
            (x, y + gap + size * 0.8, "middle"),
            (x + gap, y - gap, "start"),
            (x + gap, y + gap + size * 0.8, "start"),
            (x - gap, y - gap, "end"),
            (x - gap, y + gap + size * 0.8, "end"),
        ];
        let (left, top, right, bottom) = self.area;
        for (lx, ly, anchor) in candidates {
            let x0 = match anchor {
                "start" => lx,
                "end" => lx - width,
                _ => lx - width / 2.0,
            };
            let rect = (x0, ly - size * 0.8, x0 + width, ly + size * 0.2);
            let inside = rect.0 >= left && rect.2 <= right && rect.1 >= top && rect.3 <= bottom;
            let clear = self
                .taken
                .iter()
                .all(|t| rect.2 <= t.0 || rect.0 >= t.2 || rect.3 <= t.1 || rect.1 >= t.3);
            if inside && clear {
                self.taken.push(rect);
                return Some((lx, ly, anchor));
            }
        }
        None
    }
}

/// `chart`, an SVG chart `h` high, with `title` along its vertical axis.
pub(crate) fn y_title(chart: &str, title: &str, h: f64) -> String {
    match chart.rfind("</svg>") {
        Some(end) => format!(
            r#"{}<text class="tick" transform="translate(10 {:.1}) rotate(-90)" text-anchor="middle">{}</text>{}"#,
            &chart[..end],
            h / 2.0,
            esc(title),
            &chart[end..]
        ),
        None => chart.to_owned(),
    }
}

/// The colour of a label on a fill of `colour`: black or white, whichever has the higher
/// contrast with that series' colour in the theme.
pub(crate) fn on(colour: &str) -> String {
    match colour.strip_prefix("var(--") {
        Some(name) => format!("var(--on-{name}"),
        None => "#fff".to_owned(),
    }
}

pub(crate) fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A compact amount for an axis: 12K, 1.2M.
fn compact(x: f64) -> String {
    let a = x.abs();
    let whole = |v: f64| (v - v.round()).abs() < 1e-9;
    let text = if a >= 1e6 {
        format!("{:.1}M", x / 1e6)
    } else if a >= 1e3 && whole(x / 1e3) {
        format!("{:.0}K", x / 1e3)
    } else if a >= 1e3 {
        format!("{:.1}K", x / 1e3)
    } else {
        money(x)
    };
    text.replace('-', "−")
}

pub(crate) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * k)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Round numbers for an axis from `lo` to `hi`, about `n` steps apart.
pub(crate) fn ticks(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let (lo, hi) = if lo.is_finite() && hi.is_finite() {
        (lo.min(hi), lo.max(hi))
    } else {
        (0.0, 1.0)
    };
    let mut span = hi - lo;
    if span == 0.0 {
        span = if hi != 0.0 { hi.abs() } else { 1.0 };
    }
    let mut step = 10f64.powf((span / n as f64).log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if span / (step * m) <= n as f64 {
            step *= m;
            break;
        }
    }
    let start = (lo / step + 1e-9).floor() * step;
    let end = (hi / step - 1e-9).ceil() * step;
    let count = ((end - start) / step).round().max(1.0) as usize;
    (0..=count)
        .map(|k| ((start + k as f64 * step) * 1e12).round() / 1e12)
        .collect()
}

/// A percentage for an axis, with as few decimals as its step needs.
pub(crate) fn axis_pct(v: f64, signed: bool) -> String {
    if v.abs() < 1e-12 {
        return "0%".to_owned();
    }
    let whole = ((v * 100.0) - (v * 100.0).round()).abs() < 1e-6;
    pct(Some(v), if whole { 0 } else { 1 }, signed)
}

// ── Charts ───────────────────────────────────────────────────────────────────────────────────

pub(crate) fn legend(items: &[(String, &str)]) -> String {
    let mut out = String::from(r#"<div class="legend">"#);
    for (name, colour) in items {
        let _ = write!(
            out,
            r#"<span class="key"><i style="background:{colour}"></i>{}</span>"#,
            esc(name)
        );
    }
    out.push_str("</div>");
    out
}

struct Line<'a> {
    name: String,
    values: &'a [f64],
    colour: &'a str,
}

/// A line chart over dates, with hover bands that show each date's values.
fn line_chart(
    dates: &[Date],
    series: &[Line<'_>],
    format: &dyn Fn(f64) -> String,
    height: f64,
    area: bool,
    zero: bool,
) -> String {
    let n = dates.len();
    if n < 2 || series.is_empty() {
        return String::new();
    }
    let (w, h, l, r, t, b) = (880.0, height, 56.0, 64.0, 12.0, 28.0);
    let values: Vec<f64> = series
        .iter()
        .flat_map(|s| s.values.iter().copied())
        .filter(|v| v.is_finite())
        .collect();
    let mut lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let mut hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    let grid = ticks(lo, hi, 5);
    let (lo, hi) = (lo.min(grid[0]), hi.max(grid.last().copied().unwrap_or(hi)));
    let span = if hi > lo { hi - lo } else { 1.0 };
    let x = |i: usize| l + (w - l - r) * i as f64 / (n - 1) as f64;
    let y = |v: f64| t + (h - t - b) * (hi - v) / span;
    let mut out = crate::html::svg_open(w, h);
    for v in &grid {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            esc(&format(*v))
        );
    }
    let span_days = dates[0].days_until(dates[n - 1]);
    for i in 1..n {
        let new_year = dates[i].year() != dates[i - 1].year();
        let new_month = span_days < 400 && dates[i].month() != dates[i - 1].month();
        if new_year || new_month {
            let text = if span_days < 400 {
                let (y_, m, _) = dates[i].ymd();
                format!("{} {}", crate::text::month_name(m), y_ % 100)
            } else {
                dates[i].year().to_string()
            };
            let _ = write!(
                out,
                r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
                x(i),
                h - 8.0,
                esc(&text)
            );
        }
    }
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
        .filter_map(|s| s.values.last().map(|v| y(*v)))
        .collect();
    ends.sort_by(f64::total_cmp);
    let end_labels = ends.windows(2).all(|p| p[1] - p[0] >= 16.0);
    for s in series {
        let points: Vec<(f64, f64)> = s
            .values
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_finite())
            .map(|(i, v)| (x(i), y(*v)))
            .collect();
        let Some(&(last_x, last_y)) = points.last() else {
            continue;
        };
        let path = points
            .iter()
            .enumerate()
            .map(|(k, (px, py))| format!("{}{px:.1},{py:.1}", if k == 0 { "M" } else { " L" }))
            .collect::<String>();
        if area {
            let base = y(if zero { 0.0 } else { lo });
            let _ = write!(
                out,
                r#"<path d="{path} L{last_x:.1},{base:.1} L{:.1},{base:.1} Z" style="fill:{};opacity:.1"/>"#,
                points[0].0, s.colour
            );
        }
        let _ = write!(
            out,
            r#"<path class="line" d="{path}" style="stroke:{}"/><circle class="dot" cx="{last_x:.1}" cy="{last_y:.1}" r="4" style="fill:{}"/>"#,
            s.colour, s.colour
        );
        if end_labels && let Some(v) = s.values.last() {
            let _ = write!(
                out,
                r#"<text class="end" x="{:.1}" y="{:.1}">{}</text>"#,
                last_x + 10.0,
                last_y + 4.0,
                esc(&format(*v))
            );
        }
    }
    let bands = 110.min(n);
    for k in 0..bands {
        let (i0, i1) = (k * n / bands, (k + 1) * n / bands);
        let i = ((i0 + i1) / 2).min(n - 1);
        let mut tip = date(dates[i]);
        for s in series {
            if let Some(v) = s.values.get(i).filter(|v| v.is_finite()) {
                let _ = write!(tip, "\n{}: {}", s.name, format(*v));
            }
        }
        let _ = write!(
            out,
            r#"<rect class="hit" x="{:.1}" y="{t}" width="{:.1}" height="{}"><title>{}</title></rect>"#,
            x(i0),
            x(i1.min(n - 1)) - x(i0) + 0.5,
            h - t - b,
            esc(&tip)
        );
    }
    out.push_str("</svg>");
    if series.len() > 1 {
        let keys: Vec<(String, &str)> = series
            .iter()
            .map(|s| {
                let name = if end_labels {
                    s.name.clone()
                } else {
                    format!(
                        "{} {}",
                        s.name,
                        s.values.last().map_or_else(String::new, |v| format(*v))
                    )
                };
                (name, s.colour)
            })
            .collect();
        legend(&keys) + &out
    } else {
        out
    }
}

/// A bar with a rounded data end and a square baseline end.
fn bar_path(x: f64, y: f64, w: f64, h: f64, rightward: bool) -> String {
    if w <= 0.0 {
        return String::new();
    }
    let r = 4f64.min(w / 2.0).min(h / 2.0);
    if rightward {
        format!(
            "M{x:.1},{y:.1} H{:.1} Q{:.1},{y:.1} {:.1},{:.1} V{:.1} Q{:.1},{:.1} {:.1},{:.1} H{x:.1} Z",
            x + w - r,
            x + w,
            x + w,
            y + r,
            y + h - r,
            x + w,
            y + h,
            x + w - r,
            y + h
        )
    } else {
        format!(
            "M{:.1},{y:.1} H{:.1} Q{x:.1},{y:.1} {x:.1},{:.1} V{:.1} Q{x:.1},{:.1} {:.1},{:.1} H{:.1} Z",
            x + w,
            x + r,
            y + r,
            y + h - r,
            y + h,
            x + r,
            y + h,
            x + w
        )
    }
}

/// A column with a rounded top, or bottom for a negative value.
pub(crate) fn column_path(x: f64, y0: f64, w: f64, top: f64, up: bool) -> String {
    let h = (y0 - top).abs();
    if h <= 0.0 || w <= 0.0 {
        return String::new();
    }
    let r = 4f64.min(w / 2.0).min(h);
    if up {
        format!(
            "M{x:.1},{y0:.1} V{:.1} Q{x:.1},{top:.1} {:.1},{top:.1} H{:.1} Q{:.1},{top:.1} {:.1},{:.1} V{y0:.1} Z",
            top + r,
            x + r,
            x + w - r,
            x + w,
            x + w,
            top + r
        )
    } else {
        format!(
            "M{x:.1},{y0:.1} V{:.1} Q{x:.1},{top:.1} {:.1},{top:.1} H{:.1} Q{:.1},{top:.1} {:.1},{:.1} V{y0:.1} Z",
            top - r,
            x + r,
            x + w - r,
            x + w,
            x + w,
            top - r
        )
    }
}

/// Horizontal bars from one baseline; negative values extend left.
fn hbars(items: &[(String, f64)], format: &dyn Fn(f64) -> String, colour: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    let (width, lw, vw, row) = (880.0, 150.0, 80.0, 30.0);
    let lo = items.iter().map(|(_, v)| *v).fold(0.0, f64::min);
    let hi = items.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let span = if hi > lo { hi - lo } else { 1.0 };
    let left = lw + if lo < 0.0 { vw } else { 0.0 };
    let right = width - if hi > 0.0 { vw } else { 8.0 };
    let scale = (right - left) / span;
    let x0 = left - lo * scale;
    let h = row * items.len() as f64 + 8.0;
    let mut out = svg_open(width, h);
    let _ = write!(
        out,
        r#"<line class="base" x1="{x0:.1}" x2="{x0:.1}" y1="0" y2="{h}"/>"#
    );
    for (k, (name, v)) in items.iter().enumerate() {
        let y = 4.0 + k as f64 * row;
        let bw = v.abs() * scale;
        let x = if *v >= 0.0 { x0 } else { x0 - bw };
        let (vx, anchor) = if *v >= 0.0 {
            (x + bw + 8.0, "start")
        } else {
            (x - 8.0, "end")
        };
        let _ = write!(
            out,
            r#"<g><title>{}: {}</title><text class="lab" x="{}" y="{}" text-anchor="end">{}</text><path d="{}" style="fill:{colour}"/><text class="val" x="{vx:.1}" y="{}" text-anchor="{anchor}">{}</text></g>"#,
            esc(name),
            esc(&format(*v)),
            lw - 12.0,
            y + 17.0,
            esc(name),
            bar_path(x, y + 5.0, bw, 18.0, *v >= 0.0),
            y + 18.0,
            esc(&format(*v))
        );
    }
    out.push_str("</svg>");
    out
}

struct Columns<'a> {
    name: String,
    values: Vec<Option<f64>>,
    colour: &'a str,
}

/// Grouped columns: one column per series in each category.
fn columns(
    categories: &[String],
    series: &[Columns<'_>],
    format: &dyn Fn(f64) -> String,
    height: f64,
) -> String {
    if categories.is_empty() || series.is_empty() {
        return String::new();
    }
    let (w, h, l, r, t, b) = (880.0, height, 56.0, 16.0, 20.0, 30.0);
    let values: Vec<f64> = series
        .iter()
        .flat_map(|s| s.values.iter().flatten().copied())
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
    let cw = 24f64.min(band * 0.6 / series.len() as f64);
    let mut out = crate::html::svg_open(w, h);
    for v in &grid {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            esc(&format(*v))
        );
    }
    let _ = write!(
        out,
        r#"<line class="base" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/>"#,
        w - r,
        y(0.0),
        y(0.0)
    );
    let m = series.len() as f64;
    for (k, category) in categories.iter().enumerate() {
        let cx = l + band * (k as f64 + 0.5);
        let x = cx - cw * m / 2.0 - (m - 1.0);
        for (j, s) in series.iter().enumerate() {
            let Some(v) = s.values.get(k).copied().flatten() else {
                continue;
            };
            let xj = x + j as f64 * (cw + 2.0);
            let _ = write!(
                out,
                r#"<g><title>{} · {}: {}</title><path d="{}" style="fill:{}"/></g>"#,
                esc(category),
                esc(&s.name),
                esc(&format(v)),
                column_path(xj, y(0.0), cw, y(v), v >= 0.0),
                s.colour
            );
        }
        let _ = write!(
            out,
            r#"<text class="tick" x="{cx:.1}" y="{}" text-anchor="middle">{}</text>"#,
            h - 10.0,
            esc(category)
        );
    }
    out.push_str("</svg>");
    if series.len() > 1 {
        let keys: Vec<(String, &str)> = series.iter().map(|s| (s.name.clone(), s.colour)).collect();
        legend(&keys) + &out
    } else {
        out
    }
}

/// The tall segmented bar: one whole split into parts, with their shares inside where they fit.
fn segmented(slices: &[Slice], names: &dyn Fn(&str) -> String, label: &str, ccy: &str) -> String {
    let mut parts = String::new();
    let mut keys: Vec<(String, &str)> = Vec::new();
    for (k, s) in slices.iter().enumerate() {
        let colour = SERIES.get(k).copied().unwrap_or("var(--muted-fill)");
        let name = names(&s.name);
        let inside = if s.weight >= 0.07 {
            pct(Some(s.weight), 0, false)
        } else {
            String::new()
        };
        let _ = write!(
            parts,
            r#"<div class="seg" style="flex:{:.4};background:{colour};color:{}" title="{}: {} {ccy} ({})"><span>{inside}</span></div>"#,
            s.weight.max(0.0),
            on(colour),
            esc(&name),
            esc(&money(s.value)),
            esc(&share(s.weight))
        );
        keys.push((format!("{name} {}", share(s.weight)), colour));
    }
    format!(
        r#"<div class="segbar" role="img" aria-label="{}">{parts}</div>{}"#,
        esc(label),
        legend(&keys)
    )
}

/// The style of a heatmap cell: blue for positive, red for negative, grey at zero.
pub(crate) fn heat(value: Option<f64>, scale: f64) -> String {
    let Some(v) = value.filter(|v| v.is_finite()) else {
        return r#"class="na""#.to_owned();
    };
    let p = if scale > 0.0 {
        (v.abs() / scale * 100.0).min(100.0)
    } else {
        0.0
    };
    let pole = if v >= 0.0 { "var(--pos)" } else { "var(--neg)" };
    let ink = if p > 58.0 { "#fff" } else { "var(--ink)" };
    format!(r#"style="background:color-mix(in oklab, {pole} {p:.0}%, var(--mid));color:{ink}""#)
}

fn histogram(bins: &[Bin], period: &str) -> String {
    let (Some(first), Some(last)) = (bins.first(), bins.last()) else {
        return String::new();
    };
    let (w, h, l, r, t, b) = (880.0, 200.0, 56.0, 16.0, 12.0, 28.0);
    let top = bins
        .iter()
        .map(|x| x.share.max(x.normal_share))
        .fold(0.0, f64::max);
    let grid = ticks(0.0, top, 5);
    let hi = grid.last().copied().unwrap_or(1.0).max(1e-12);
    let (lo_x, hi_x) = (first.lower, last.upper);
    let span = if hi_x > lo_x { hi_x - lo_x } else { 1.0 };
    let x = |v: f64| l + (w - l - r) * (v - lo_x) / span;
    let y = |v: f64| t + (h - t - b) * (hi - v) / hi;
    let mut out = crate::html::svg_open(w, h);
    for v in &grid {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            axis_pct(*v, false)
        );
    }
    for bin in bins {
        let (x0, x1) = (x(bin.lower) + 1.0, x(bin.upper) - 1.0);
        let _ = write!(
            out,
            r#"<g><title>{} to {}: {} ({})</title><path d="{}" style="fill:var(--s1)"/></g>"#,
            pct(Some(bin.lower), 2, true),
            pct(Some(bin.upper), 2, true),
            plural(bin.count, period),
            share(bin.share),
            column_path(x0, y(0.0), x1 - x0, y(bin.share), true)
        );
    }
    let curve = bins
        .iter()
        .enumerate()
        .map(|(k, bin)| {
            format!(
                "{}{:.1},{:.1}",
                if k == 0 { "M" } else { " L" },
                x((bin.lower + bin.upper) / 2.0),
                y(bin.normal_share)
            )
        })
        .collect::<String>();
    let _ = write!(
        out,
        r#"<path class="line" d="{curve}" style="stroke:var(--s2)"/>"#
    );
    for v in ticks(lo_x, hi_x, 6) {
        if v >= lo_x && v <= hi_x {
            let _ = write!(
                out,
                r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
                x(v),
                h - 8.0,
                axis_pct(v, true)
            );
        }
    }
    let _ = write!(
        out,
        r#"<line class="base" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/></svg>"#,
        w - r,
        y(0.0),
        y(0.0)
    );
    legend(&[
        (format!("Returns, one {period} each"), "var(--s1)"),
        ("Normal distribution".to_owned(), "var(--s2)"),
    ]) + &out
}

fn frontier_chart(
    points: &[FrontierPoint],
    current: &PortfolioPoint,
    holdings: &[HoldingRiskReturn],
) -> String {
    let (w, h, l, r, t, b) = (880.0, 272.0, 56.0, 120.0, 24.0, 34.0);
    let xs: Vec<f64> = points
        .iter()
        .map(|p| p.volatility)
        .chain(std::iter::once(current.volatility))
        .chain(holdings.iter().map(|x| x.volatility))
        .collect();
    let ys: Vec<f64> = points
        .iter()
        .map(|p| p.expected_return)
        .chain(std::iter::once(current.expected_return))
        .chain(holdings.iter().map(|x| x.expected_return))
        .collect();
    let xt = ticks(0.0, xs.iter().copied().fold(0.0, f64::max), 5);
    let yt = ticks(
        ys.iter().copied().fold(0.0, f64::min),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        5,
    );
    let x_hi = xt.last().copied().unwrap_or(1.0).max(1e-12);
    let (y_lo, y_hi) = (yt[0], yt.last().copied().unwrap_or(1.0));
    let y_span = if y_hi > y_lo { y_hi - y_lo } else { 1.0 };
    let x = |v: f64| l + (w - l - r) * v / x_hi;
    let y = |v: f64| t + (h - t - b) * (y_hi - v) / y_span;
    let mut out = crate::html::svg_open(w, h);
    for v in &yt {
        let _ = write!(
            out,
            r#"<line class="grid" x1="{l}" x2="{}" y1="{:.1}" y2="{:.1}"/><text class="tick" x="{}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - r,
            y(*v),
            y(*v),
            l - 8.0,
            y(*v) + 4.0,
            axis_pct(*v, true)
        );
    }
    for v in &xt {
        let _ = write!(
            out,
            r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
            x(*v),
            h - 16.0,
            axis_pct(*v, false)
        );
    }
    let _ = write!(
        out,
        r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">Volatility, a year</text><text class="tick" x="{}" y="{}">Expected return, a year</text>"#,
        (l + w - r) / 2.0,
        h - 2.0,
        l - 44.0,
        t - 10.0
    );
    let path = points
        .iter()
        .enumerate()
        .map(|(k, p)| {
            format!(
                "{}{:.1},{:.1}",
                if k == 0 { "M" } else { " L" },
                x(p.volatility),
                y(p.expected_return)
            )
        })
        .collect::<String>();
    let _ = write!(
        out,
        r#"<path class="line" d="{path}" style="stroke:var(--s1)"/>"#
    );
    // The holdings, the largest labelled first, each label where it overlaps nothing.
    let centres: Vec<(f64, f64)> = holdings
        .iter()
        .map(|i| (x(i.volatility), y(i.expected_return)))
        .chain(std::iter::once((
            x(current.volatility),
            y(current.expected_return),
        )))
        .collect();
    let mut labels = Labels::new((l, t, w - 4.0, h - b), &centres, 6.0);
    let mut by_weight: Vec<&HoldingRiskReturn> = holdings.iter().collect();
    by_weight.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    for item in by_weight {
        let name = holding(&item.holding);
        let (cx, cy) = (x(item.volatility), y(item.expected_return));
        let _ = write!(
            out,
            r#"<g><title>{}: return {}, volatility {}</title><circle class="dot" cx="{cx:.1}" cy="{cy:.1}" r="4" style="fill:var(--s3)"/></g>"#,
            esc(&name),
            signed(item.expected_return),
            share(item.volatility),
        );
        if let Some((lx, ly, anchor)) = labels.place(cx, cy, &name, 13.0) {
            let _ = write!(
                out,
                r#"<text class="lab" x="{lx:.1}" y="{ly:.1}" text-anchor="{anchor}">{}</text>"#,
                esc(&name)
            );
        }
    }
    let _ = write!(
        out,
        r#"<g><title>Current portfolio: return {}, volatility {}</title><circle class="dot" cx="{:.1}" cy="{:.1}" r="6" style="fill:var(--s2)"/></g></svg>"#,
        signed(current.expected_return),
        share(current.volatility),
        x(current.volatility),
        y(current.expected_return)
    );
    legend(&[
        ("Efficient frontier".to_owned(), "var(--s1)"),
        ("Current portfolio".to_owned(), "var(--s2)"),
        ("Holdings".to_owned(), "var(--s3)"),
    ]) + &out
}

// ── Layout ───────────────────────────────────────────────────────────────────────────────────

/// A table: cells are text, escaped here; the columns in `right` are right-aligned.
fn table(headings: &[&str], rows: &[Vec<String>], right: &[usize]) -> String {
    let mut out = String::from(r#"<div class="tw"><table><thead><tr>"#);
    for (i, h) in headings.iter().enumerate() {
        let class = if right.contains(&i) { "r" } else { "" };
        let _ = write!(out, r#"<th class="{class}">{}</th>"#, esc(h));
    }
    out.push_str("</tr></thead><tbody>");
    for row in rows {
        out.push_str("<tr>");
        for (i, c) in row.iter().enumerate() {
            let class = if right.contains(&i) { "r" } else { "" };
            let _ = write!(out, r#"<td class="{class}">{}</td>"#, esc(c));
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table></div>");
    out
}

/// Every column but the first, right-aligned.
fn numbers(n: usize) -> Vec<usize> {
    (1..n).collect()
}

pub(crate) fn tiles(items: &[(String, String, Option<String>)]) -> String {
    let mut out = String::from(r#"<div class="tiles">"#);
    for (label, value, sub) in items {
        // A long value, such as "14.3% → 11.2%", in a smaller size, so that it keeps to one
        // line in a narrow tile.
        let class = if value.chars().count() > 9 {
            "tv long"
        } else {
            "tv"
        };
        let _ = write!(
            out,
            r#"<div class="tile"><div class="tl">{}</div><div class="{class}">{}</div>"#,
            esc(label),
            esc(value)
        );
        if let Some(sub) = sub {
            let _ = write!(out, r#"<div class="ts">{}</div>"#, esc(sub));
        }
        out.push_str("</div>");
    }
    out.push_str("</div>");
    out
}

pub(crate) fn card(title: &str, body: &str, note: Option<&str>) -> String {
    let note = note.map_or_else(String::new, |n| {
        format!(r#"<p class="note">{}</p>"#, esc(n))
    });
    format!(
        r#"<div class="card"><h3>{}</h3>{body}{note}</div>"#,
        esc(title)
    )
}

/// Cards side by side, two to a row.
pub(crate) fn pairs(cards: &[String]) -> String {
    cards
        .chunks(2)
        .map(|c| {
            if c.len() == 2 {
                format!(r#"<div class="two">{}{}</div>"#, c[0], c[1])
            } else {
                c[0].clone()
            }
        })
        .collect()
}

fn section(anchor: &str, title: &str, subtitle: &str, body: &str, p: &Provenance) -> String {
    format!(
        r#"<section id="{anchor}"><header class="sh"><h2>{}</h2><p>{}</p></header>{body}<p class="prov">Finalytics {} · {} report · inputs <code>{}</code> · {}</p></section>"#,
        esc(title),
        esc(subtitle),
        esc(&p.version),
        anchor,
        esc(&p.source.digest.chars().take(12).collect::<String>()),
        plural(p.warnings.issues().len(), "warning")
    )
}

fn heat_table(
    rows: &[String],
    columns: &[String],
    values: &[Vec<Option<f64>>],
    scale: f64,
    format: &dyn Fn(f64) -> String,
    diagonal: bool,
) -> String {
    let mut out = String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
    for c in columns {
        let _ = write!(out, "<th>{}</th>", esc(c));
    }
    out.push_str("</tr></thead><tbody>");
    for (i, (row, values)) in rows.iter().zip(values).enumerate() {
        let _ = write!(out, "<tr><th>{}</th>", esc(row));
        for (j, v) in values.iter().enumerate() {
            let text = v.map_or_else(String::new, format);
            if diagonal && i == j {
                let _ = write!(out, r#"<td class="diag">{}</td>"#, esc(&text));
            } else {
                let _ = write!(
                    out,
                    r#"<td {} title="{} · {}: {}">{}</td>"#,
                    heat(*v, scale),
                    esc(row),
                    esc(columns.get(j).map_or("", String::as_str)),
                    esc(&text),
                    esc(&text)
                );
            }
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table></div>");
    out
}

// ── The reports ──────────────────────────────────────────────────────────────────────────────

fn overview(o: &OverviewReport) -> String {
    let p = &o.provenance;
    let ccy = p.base_currency.to_string();
    let h = &o.headline;
    let period = period_word(p.options.periods_per_year);
    let mut body = String::new();
    let mut items = vec![
        (format!("Value ({ccy})"), money(h.value), None),
        ("Year to date".to_owned(), signed(h.year_to_date), None),
        (
            "Since inception".to_owned(),
            signed(h.since_inception),
            Some(format!("{} a year", signed(h.since_inception_annualised))),
        ),
        (
            "Volatility".to_owned(),
            share(h.volatility),
            Some(format!("Sharpe ratio {}", num(h.sharpe_ratio, 2))),
        ),
        ("Maximum drawdown".to_owned(), signed(h.max_drawdown), None),
    ];
    if let (Some(c), Some(var)) = (h.confidence, &h.value_at_risk) {
        items.push((
            format!("Value at risk, {}, 1 {period}", pct(Some(c), 0, false)),
            money(var.value),
            h.expected_shortfall
                .as_ref()
                .map(|es| format!("Expected shortfall {}", money(es.value))),
        ));
    }
    body.push_str(&tiles(&items));
    if o.growth.values.len() > 1 {
        let mut lines = vec![Line {
            name: "Portfolio".to_owned(),
            values: &o.growth.values,
            colour: SERIES[0],
        }];
        if let Some(b) = &o.benchmark_growth {
            lines.push(Line {
                name: benchmark_label(&b.name),
                values: &b.values,
                colour: SERIES[1],
            });
        }
        let format = |v: f64| {
            if (v - v.round()).abs() < 1e-9 {
                format!("{v:.0}")
            } else {
                format!("{v:.1}")
            }
        };
        body.push_str(&card(
            "Growth of 100",
            &y_title(
                &line_chart(&o.growth.dates, &lines, &format, 240.0, false, false),
                "Value of 100 invested",
                240.0,
            ),
            None,
        ));
    }
    let allocations: Vec<String> = o
        .allocation
        .iter()
        .skip(1)
        .map(|b| {
            card(
                &format!("Allocation by {}", words(&b.by)),
                &segmented(&b.slices, &group, &words(&b.by), &ccy),
                None,
            )
        })
        .collect();
    body.push_str(&pairs(&allocations));
    if let (Some(frontier), Some(current)) = (&o.frontier, &o.current)
        && !frontier.is_empty()
    {
        let method = o.risk.as_ref().map_or_else(
            || "the covariance".to_owned(),
            |r| covariance_name(&r.method),
        );
        let given = if o.holdings.iter().any(|x| x.expected_return_given) {
            "some expected returns are given and the others are historical means"
        } else {
            "expected returns are historical means"
        };
        body.push_str(&card(
            "Efficient frontier of the invested holdings",
            &frontier_chart(frontier, current, &o.holdings),
            Some(&format!(
                "Long only and fully invested in the holdings other than base-currency cash; \
                 {given}; {method}."
            )),
        ));
    }
    if !o.holdings.is_empty() {
        let rows: Vec<Vec<String>> = o
            .holdings
            .iter()
            .map(|x| {
                vec![
                    holding(&x.holding),
                    share(x.weight),
                    signed(x.expected_return),
                    share(x.volatility),
                    share(x.risk_share),
                ]
            })
            .collect();
        body.push_str(&card(
            "Holdings: expected return and risk",
            &table(
                &[
                    "Holding",
                    "Weight",
                    "Expected return",
                    "Volatility",
                    "Share of risk",
                ],
                &rows,
                &numbers(5),
            ),
            None,
        ));
    }
    section(
        "overview",
        "Overview",
        "Value, return and risk at a glance",
        &body,
        p,
    )
}

fn holdings(h: &HoldingsReport) -> String {
    let p = &h.provenance;
    let ccy = p.base_currency.to_string();
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
    let account_name = |id: &str| names.get(id).cloned().unwrap_or_else(|| group(id));
    let mut body = String::new();
    let rows: Vec<Vec<String>> = h
        .holdings
        .iter()
        .map(|x| {
            let cash = matches!(x.holding, HoldingKey::Cash(_));
            vec![
                holding(&x.holding),
                x.name.clone().unwrap_or_default(),
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
    body.push_str(&card(
        &format!("Positions · {} {ccy}", money(dec(h.value))),
        &table(
            &[
                "Holding",
                "Name",
                "Quantity",
                "Price",
                &format!("Value ({ccy})"),
                "Weight",
                "Unrealised",
                "Return",
            ],
            &rows,
            &[2, 3, 4, 5, 6, 7],
        ),
        None,
    ));
    let breakdowns: Vec<String> = ["account", "currency"]
        .iter()
        .filter_map(|by| h.breakdowns.iter().find(|b| b.by == *by))
        .map(|b| {
            card(
                &format!("By {}", words(&b.by)),
                &segmented(&b.slices, &account_name, &words(&b.by), &ccy),
                None,
            )
        })
        .collect();
    body.push_str(&pairs(&breakdowns));
    let c = &h.concentration;
    let long = h.lots.iter().filter(|x| x.long_term).count();
    let mut items = Vec::new();
    if let Some(top) = c.largest.first() {
        items.push((
            "Largest holding".to_owned(),
            format!("{} {}", top.name, share(top.weight)),
            Some("of the value invested, cash aside".to_owned()),
        ));
    }
    items.push((
        "Top five".to_owned(),
        share(c.top_five),
        Some("of the value invested".to_owned()),
    ));
    items.push((
        "Effective holdings".to_owned(),
        num(c.effective_holdings, 1),
        Some("1 ÷ Herfindahl index".to_owned()),
    ));
    items.push((
        "Open tax lots".to_owned(),
        h.lots.len().to_string(),
        Some(format!(
            "{long} long-term · {} short-term",
            h.lots.len() - long
        )),
    ));
    body.push_str(&tiles(&items));
    if !h.lots.is_empty() {
        let mut lots: Vec<&crate::LotRow> = h.lots.iter().collect();
        lots.sort_by_key(|x| std::cmp::Reverse(x.unrealised_base));
        let rows: Vec<Vec<String>> = lots
            .iter()
            .take(8)
            .map(|x| {
                vec![
                    x.lot.to_string(),
                    account_name(x.account.as_str()),
                    x.instrument.to_string(),
                    date(x.acquired),
                    money(dec(x.quantity)),
                    money(dec(x.cost_base)),
                    money(dec(x.unrealised_base)),
                    signed(x.unrealised_return),
                    if x.long_term { "Long" } else { "Short" }.to_owned(),
                ]
            })
            .collect();
        let total = |long_term: bool| -> f64 {
            h.lots
                .iter()
                .filter(|x| x.long_term == long_term)
                .map(|x| dec(x.unrealised_base))
                .sum()
        };
        body.push_str(&card(
            "Tax lots: largest unrealised gains",
            &table(
                &[
                    "Lot",
                    "Account",
                    "Holding",
                    "Acquired",
                    "Quantity",
                    &format!("Cost ({ccy})"),
                    &format!("Gain ({ccy})"),
                    "Return",
                    "Term",
                ],
                &rows,
                &[3, 4, 5, 6, 7],
            ),
            Some(&format!(
                "Unrealised: {} long-term, {} short-term; a lot is long-term when held for more \
                 than {} days.",
                money(total(true)),
                money(total(false)),
                p.options.long_term_after_days
            )),
        ));
    }
    let years: Vec<i32> = h
        .realised
        .by_year
        .keys()
        .chain(h.income.by_year.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if !years.is_empty() {
        let categories: Vec<String> = years.iter().map(i32::to_string).collect();
        let of = |m: &BTreeMap<i32, Decimal>, y: &i32| m.get(y).copied().map_or(0.0, dec);
        let chart = columns(
            &categories,
            &[
                Columns {
                    name: "Income".to_owned(),
                    values: years
                        .iter()
                        .map(|y| Some(of(&h.income.by_year, y)))
                        .collect(),
                    colour: SERIES[0],
                },
                Columns {
                    name: "Realised gains".to_owned(),
                    values: years
                        .iter()
                        .map(|y| Some(of(&h.realised.by_year, y)))
                        .collect(),
                    colour: SERIES[1],
                },
            ],
            &compact,
            180.0,
        );
        let mut headings: Vec<&str> = vec![""];
        headings.extend(categories.iter().map(String::as_str));
        headings.push("Total");
        let row = |name: &str, m: &BTreeMap<i32, Decimal>, total: Decimal| {
            let mut cells = vec![name.to_owned()];
            cells.extend(years.iter().map(|y| money(of(m, y))));
            cells.push(money(dec(total)));
            cells
        };
        let rows = vec![
            row("Income", &h.income.by_year, h.income.total),
            row("Realised gains", &h.realised.by_year, h.realised.total),
        ];
        body.push_str(&card(
            &format!("Realised gains and income ({ccy})"),
            &(chart + &table(&headings, &rows, &numbers(headings.len()))),
            None,
        ));
    }
    section(
        "holdings",
        "Holdings",
        "What the portfolio holds, what it cost, and its tax lots",
        &body,
        p,
    )
}

fn performance(r: &PerformanceReport, names: &BTreeMap<String, String>) -> String {
    let p = &r.provenance;
    let benchmark = r.benchmark_growth.is_some();
    let mut body = String::new();
    let rows: Vec<Vec<String>> = r
        .periods
        .iter()
        .filter(|x| x.cumulative.is_some())
        .map(|x| {
            let mut row = vec![
                x.label.clone(),
                x.start.map_or_else(|| "–".to_owned(), date),
                signed(x.cumulative),
                signed(x.annualised),
                signed(x.money_weighted),
            ];
            if benchmark {
                row.push(signed(x.benchmark));
                row.push(signed(
                    x.cumulative
                        .zip(x.benchmark)
                        .map(|(p, b)| crate::text::shown_difference(b, p, 3)),
                ));
            }
            row
        })
        .collect();
    let mut headings = vec!["Period", "From", "Return", "Annualised", "Money-weighted"];
    if benchmark {
        headings.extend(["Benchmark", "Excess"]);
    }
    let short = r.periods.iter().any(|x| x.cumulative.is_none());
    body.push_str(&card(
        "Returns by period",
        &table(&headings, &rows, &numbers(headings.len())),
        Some(if short {
            "Time-weighted unless noted; money-weighted returns, like the others, are annual \
             rates for a year or more and over the period for shorter ones. Periods longer than \
             the history are left out."
        } else {
            "Time-weighted unless noted; money-weighted returns, like the others, are annual \
             rates for a year or more and over the period for shorter ones."
        }),
    ));
    let m = &r.monthly;
    if !m.rows.is_empty() {
        let months = m.columns.len().saturating_sub(1);
        let scale = m
            .values
            .iter()
            .flat_map(|row| row.iter().take(months).flatten())
            .fold(0.0f64, |a, v| a.max(v.abs()));
        let mut heatmap =
            String::from(r#"<div class="tw"><table class="heat"><thead><tr><th></th>"#);
        for c in m.columns.iter().take(months) {
            let _ = write!(heatmap, "<th>{}</th>", esc(c));
        }
        heatmap.push_str(r#"<th class="yr">Year</th></tr></thead><tbody>"#);
        for (year, row) in m.rows.iter().zip(&m.values) {
            let _ = write!(heatmap, "<tr><th>{}</th>", esc(year));
            for (j, v) in row.iter().take(months).enumerate() {
                let text = v.map_or_else(String::new, |v| num(Some(v * 100.0), 1));
                let _ = write!(
                    heatmap,
                    r#"<td {} title="{} {}: {}">{}</td>"#,
                    heat(*v, scale),
                    esc(&m.columns[j]),
                    esc(year),
                    esc(&signed(*v)),
                    esc(&text)
                );
            }
            let _ = write!(
                heatmap,
                r#"<td class="yr">{}</td></tr>"#,
                esc(&signed(row.get(months).copied().flatten()))
            );
        }
        heatmap.push_str("</tbody></table></div>");
        body.push_str(&card("Monthly returns (%)", &heatmap, None));
    }
    if !r.calendar_years.is_empty() {
        let categories: Vec<String> = r
            .calendar_years
            .iter()
            .map(|y| format!("{}{}", y.year, if y.complete { "" } else { "*" }))
            .collect();
        let mut series = vec![Columns {
            name: "Portfolio".to_owned(),
            values: r.calendar_years.iter().map(|y| Some(y.portfolio)).collect(),
            colour: SERIES[0],
        }];
        if benchmark {
            series.push(Columns {
                name: benchmark_label(r.benchmark_growth.as_ref().map_or("", |b| b.name.as_str())),
                values: r.calendar_years.iter().map(|y| y.benchmark).collect(),
                colour: SERIES[1],
            });
        }
        let partial = r.calendar_years.iter().any(|y| !y.complete);
        body.push_str(&card(
            "Calendar years",
            &columns(&categories, &series, &|v| axis_pct(v, true), 220.0),
            partial.then_some("* part of the year"),
        ));
    }
    let mut ranked: Vec<&finalytics_perf::Contribution> = r.contributions.iter().collect();
    ranked.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
    let mut bars: Vec<(String, f64)> = ranked
        .iter()
        .map(|x| (holding(&x.holding), x.contribution))
        .collect();
    if r.expenses != 0.0 {
        bars.push(("Fees and taxes".to_owned(), r.expenses));
    }
    let mut contribution = hbars(
        &bars,
        &|v| pct(Some(v), if v.abs() < 0.001 { 2 } else { 1 }, true),
        SERIES[0],
    );
    for g in &r.groups {
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
        let heading = {
            let w = words(&g.scheme);
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        };
        contribution.push_str(&table(
            &[&heading, "Contribution", "Average weight"],
            &rows,
            &[1, 2],
        ));
    }
    body.push_str(&card(
        "Contribution to return since inception",
        &contribution,
        (r.expenses != 0.0).then_some(
            "Fees and taxes charged to the accounts are their own line; with the holdings' \
             contributions they add up to the return.",
        ),
    ));
    if let Some(hh) = &r.household {
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
        body.push_str(&card(
            "Accounts",
            &table(
                &[
                    "Account",
                    "Start",
                    "End",
                    "Net flows",
                    "Gain",
                    "Return",
                    "Money-weighted",
                ],
                &rows,
                &numbers(7),
            ),
            None,
        ));
    }
    let subtitle = if benchmark {
        "Time- and money-weighted returns against the benchmark"
    } else {
        "Time- and money-weighted returns"
    };
    section("performance", "Performance", subtitle, &body, p)
}

fn risk(r: &RiskReport) -> String {
    let p = &r.provenance;
    let ccy = p.base_currency.to_string();
    let period = period_word(p.options.periods_per_year);
    let s = &r.statistics;
    let mut body = String::new();
    let mut items = vec![
        (
            "Volatility".to_owned(),
            share(s.volatility),
            Some(format!("downside {}", share(s.downside_deviation))),
        ),
        (
            "Sharpe ratio".to_owned(),
            num(s.sharpe_ratio, 2),
            Some(format!(
                "Sortino {}; both against {} a year",
                num(s.sortino_ratio, 2),
                share(p.options.risk_free_rate)
            )),
        ),
        (
            "Maximum drawdown".to_owned(),
            signed(s.max_drawdown),
            Some(format!(
                "Calmar {}; CDaR 95% {}; Ulcer {}",
                num(s.calmar_ratio, 2),
                signed(s.conditional_drawdown_at_risk),
                share(s.ulcer_index)
            )),
        ),
    ];
    if let Some(rel) = &r.relative {
        items.push((
            "Tracking error".to_owned(),
            share(rel.tracking_error),
            Some(format!(
                "information ratio {}",
                num(rel.information_ratio, 2)
            )),
        ));
        items.push((
            "Beta".to_owned(),
            num(rel.beta, 2),
            Some(format!(
                "correlation {}; down {}, up {}",
                num(rel.correlation, 2),
                num(rel.downside_beta, 2),
                num(rel.upside_beta, 2)
            )),
        ));
        items.push((
            "Capture".to_owned(),
            format!("{} down", num(rel.downside_capture, 2)),
            Some(format!(
                "{} up: the share of the benchmark's falls and rises",
                num(rel.upside_capture, 2)
            )),
        ));
    }
    items.push((
        format!("Worst {period}"),
        pct(Some(s.worst), 2, true),
        Some(format!("best {}", pct(Some(s.best), 2, true))),
    ));
    body.push_str(&tiles(&items));
    if r.drawdown.values.len() > 1 {
        let chart = line_chart(
            &r.drawdown.dates,
            &[Line {
                name: "Drawdown".to_owned(),
                values: &r.drawdown.values,
                colour: SERIES[0],
            }],
            &|v| axis_pct(v, true),
            200.0,
            true,
            true,
        );
        let rows: Vec<Vec<String>> = r
            .drawdowns
            .iter()
            .take(5)
            .map(|x| {
                vec![
                    date(x.peak),
                    date(x.trough),
                    x.recovery.map_or_else(|| "Not yet".to_owned(), date),
                    signed(x.depth),
                    x.days_to_trough.to_string(),
                    x.days_to_recovery
                        .map_or_else(|| "–".to_owned(), |d| d.to_string()),
                ]
            })
            .collect();
        body.push_str(&card(
            "Drawdown",
            &(chart
                + &table(
                    &[
                        "Peak",
                        "Trough",
                        "Recovery",
                        "Depth",
                        "Days to trough",
                        "Days to recover",
                    ],
                    &rows,
                    &[3, 4, 5],
                )),
            None,
        ));
    }
    if !r.tail.is_empty() {
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
                    format!(
                        "{}{}",
                        pct(Some(x.expected_shortfall.fraction), 2, false),
                        if x.shortfall_at_var { "*" } else { "" }
                    ),
                ]
            })
            .collect();
        body.push_str(&card(
            &format!("Value at risk and expected shortfall, 1 {period}"),
            &table(
                &[
                    "Confidence",
                    "Method",
                    &format!("Value at risk ({ccy})"),
                    "",
                    &format!("Expected shortfall ({ccy})"),
                    "",
                ],
                &rows,
                &[2, 3, 4, 5],
            ),
            r.tail
                .iter()
                .any(|x| x.shortfall_at_var)
                .then_some(crate::text::SHORTFALL_AT_VAR),
        ));
    }
    if let Some(hr) = &r.holdings {
        let d = &hr.decomposition;
        let mut ranked: Vec<_> = d.contributions.iter().collect();
        ranked.sort_by(|a, b| b.share.unwrap_or(0.0).total_cmp(&a.share.unwrap_or(0.0)));
        let categories: Vec<String> = ranked.iter().map(|x| x.label.clone()).collect();
        body.push_str(&card(
            &format!(
                "Where the risk comes from · volatility {}",
                share(d.volatility)
            ),
            &columns(
                &categories,
                &[
                    Columns {
                        name: "Weight".to_owned(),
                        values: ranked.iter().map(|x| Some(x.weight)).collect(),
                        colour: SERIES[0],
                    },
                    Columns {
                        name: "Share of risk".to_owned(),
                        values: ranked.iter().map(|x| x.share).collect(),
                        colour: SERIES[1],
                    },
                ],
                &|v| axis_pct(v, false),
                220.0,
            ),
            Some(&format!(
                "{} over {}, annualised; cash in the base currency is riskless.",
                crate::backtest_html::capitalise(&covariance_name(&hr.method)),
                plural(hr.observations, period)
            )),
        ));
    }
    if !r.histogram.bins.is_empty() {
        body.push_str(&card(
            &format!(
                "{} returns",
                match period {
                    "day" => "Daily",
                    "week" => "Weekly",
                    "month" => "Monthly",
                    _ => "Periodic",
                }
            ),
            &histogram(&r.histogram.bins, period),
            Some(&format!(
                "Skewness {}, excess kurtosis {}.",
                num(s.skewness, 2),
                num(s.excess_kurtosis, 2)
            )),
        ));
    }
    if let Some(hr) = &r.holdings {
        let (rows, values, riskless) = crate::text::with_risk(&hr.correlation);
        let note = (!riskless.is_empty())
            .then(|| format!("{} has no risk and is left out.", riskless.join(", ")));
        body.push_str(&card(
            "Correlation",
            &heat_table(&rows, &rows, &values, 1.0, &|v| num(Some(v), 2), true),
            note.as_deref(),
        ));
    }
    if let Some(rolling) = r
        .rolling
        .iter()
        .find(|s| s.name.starts_with("Rolling volatility"))
        && rolling.values.len() > 1
    {
        body.push_str(&card(
            &format!(
                "Rolling volatility, {}",
                plural(p.options.rolling_window, period)
            ),
            &line_chart(
                &rolling.dates,
                &[Line {
                    name: "Rolling volatility".to_owned(),
                    values: &rolling.values,
                    colour: SERIES[0],
                }],
                &|v| match rolling.unit {
                    Unit::Fraction => axis_pct(v, false),
                    _ => num(Some(v), 2),
                },
                180.0,
                false,
                false,
            ),
            None,
        ));
    }
    section(
        "risk",
        "Risk",
        "Volatility, drawdowns, tail risk and where the risk comes from",
        &body,
        p,
    )
}

fn attribution(a: &AttributionReport) -> String {
    let p = &a.provenance;
    let at = &a.attribution;
    let two = |x: f64| pct(Some(x), 2, true);
    let mut body = tiles(&[
        ("Portfolio".to_owned(), two(at.portfolio_return), None),
        ("Benchmark".to_owned(), two(at.benchmark_return), None),
        ("Excess".to_owned(), two(at.excess_return), None),
        ("Allocation".to_owned(), two(at.allocation), None),
        ("Selection".to_owned(), two(at.selection), None),
        ("Interaction".to_owned(), two(at.interaction), None),
    ]);
    let segments = &at.segments;
    let categories: Vec<String> = segments.iter().map(|s| group(&s.name)).collect();
    let chart = columns(
        &categories,
        &[
            Columns {
                name: "Allocation".to_owned(),
                values: segments.iter().map(|s| Some(s.allocation)).collect(),
                colour: SERIES[0],
            },
            Columns {
                name: "Selection".to_owned(),
                values: segments.iter().map(|s| Some(s.selection)).collect(),
                colour: SERIES[1],
            },
            Columns {
                name: "Interaction".to_owned(),
                values: segments.iter().map(|s| Some(s.interaction)).collect(),
                colour: SERIES[2],
            },
        ],
        &|v| axis_pct(v, true),
        220.0,
    );
    let mut rows: Vec<Vec<String>> = segments
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
    let scheme = words(&a.scheme);
    let heading = {
        let mut c = scheme.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    };
    let linking = match at.linking {
        Linking::Carino => "Cariño's method",
        Linking::Menchero => "Menchero's method",
        _ => "smoothing",
    };
    let mut note = format!(
        "Brinson–Fachler, {} linked by {linking}.",
        plural(a.periods.len(), "period")
    );
    if let Some(twr) = a.time_weighted {
        let _ = write!(
            note,
            " Segment returns are measured over each period, so the linked total can differ \
             slightly from the time-weighted return over the same span ({}).",
            pct(Some(twr), 2, true)
        );
    }
    body.push_str(&card(
        &format!("Effects by {scheme}"),
        &(chart
            + &table(
                &[&heading, "Allocation", "Selection", "Interaction", "Total"],
                &rows,
                &numbers(5),
            )),
        Some(&note),
    ));
    section(
        "attribution",
        "Attribution",
        "Why the portfolio beat or trailed the benchmark",
        &body,
        p,
    )
}

fn scenarios(s: &ScenariosReport) -> String {
    let p = &s.provenance;
    let ccy = p.base_currency.to_string();
    let mut body = String::new();
    let bars: Vec<(String, f64)> = s
        .summary
        .iter()
        .map(|x| (x.name.clone(), x.change))
        .collect();
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
    let failed: Vec<String> = s
        .summary
        .iter()
        .filter_map(crate::text::stress_note)
        .chain(s.scenarios.iter().filter(|o| o.result.is_none()).map(
            |o| match o.problems.first() {
                Some(i) => format!("{} could not be run: {}", o.name, i.message),
                None => format!("{} could not be run.", o.name),
            },
        ))
        .collect();
    body.push_str(&card(
        &format!("Effect on today's portfolio · {} {ccy}", money(s.value)),
        &(hbars(&bars, &money, SERIES[0])
            + &table(
                &[
                    "Scenario",
                    &format!("Change ({ccy})"),
                    "Return",
                    "Hardest hit",
                    "",
                    "Volatility after",
                ],
                &rows,
                &[1, 2, 4, 5],
            )),
        (!failed.is_empty()).then(|| failed.join(" ")).as_deref(),
    ));
    let g = &s.changes;
    if !g.rows.is_empty() {
        let scale = g
            .values
            .iter()
            .flatten()
            .flatten()
            .fold(0.0f64, |a, v| a.max(v.abs()));
        let notes: Vec<String> = p
            .options
            .scenarios
            .iter()
            .filter_map(|named| match &named.scenario {
                ScenarioDefinition::Historical { window } => Some(format!(
                    "{}: the price and rate changes from {} to {}, on today's holdings.",
                    named.name,
                    date(window.start),
                    date(window.end)
                )),
                _ => None,
            })
            .collect();
        body.push_str(&card(
            &format!("Change in value by holding ({ccy})"),
            &heat_table(&g.rows, &g.columns, &g.values, scale, &money, false),
            (!notes.is_empty()).then(|| notes.join(" ")).as_deref(),
        ));
    }
    section(
        "scenarios",
        "Scenarios",
        "What shocks and past episodes would do to today's portfolio",
        &body,
        p,
    )
}

pub(crate) const CSS: &str = include_str!("../assets/report.css");

/// The colours of a report's page: the reader's light or dark setting (the default), or one of
/// the two always.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Theme {
    /// Light or dark as the reader's system or browser is set.
    #[default]
    Auto,
    /// Light, whatever the reader's setting.
    Light,
    /// Dark, whatever the reader's setting.
    Dark,
}

impl Theme {
    /// `page`, a report's HTML page from `to_html`, in this theme.
    ///
    /// ```
    /// # mod finalytics { pub mod reports { pub use finalytics_reports::*; } }
    /// use finalytics::reports::Theme;
    ///
    /// let page = "<!doctype html>\n<html lang=\"en\"><head></head></html>";
    /// assert!(Theme::Dark.apply(page).contains(r#"<html lang="en" data-theme="dark">"#));
    /// assert_eq!(Theme::Auto.apply(page), page);
    /// ```
    pub fn apply(self, page: &str) -> String {
        let name = match self {
            Theme::Auto => return page.to_owned(),
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        page.replacen(
            r#"<html lang="en">"#,
            &format!(r#"<html lang="en" data-theme="{name}">"#),
            1,
        )
    }
}

/// A benchmark's name for a chart's legend: "Benchmark" when it has none.
pub(crate) fn benchmark_label(name: &str) -> String {
    if name.is_empty() {
        "Benchmark".to_owned()
    } else {
        name.to_owned()
    }
}

impl PortfolioReport {
    /// The six reports as one self-contained HTML page, with the portfolio's `name` (or the
    /// report's own) as its title, if given: inline styles and SVG charts with hover details, no
    /// scripts or external resources, in the reader's light or dark theme, and laid out for
    /// printing, one report to a page.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let name = name.or(self.name.as_deref());
        let p = &self.overview.provenance;
        let ccy = p.base_currency.to_string();
        let title = name.unwrap_or("Portfolio report");
        let mut parts = vec![
            ("overview", "Overview", overview(&self.overview)),
            ("holdings", "Holdings", holdings(&self.holdings)),
            (
                "performance",
                "Performance",
                performance(
                    &self.performance,
                    &crate::text::account_names(&self.holdings),
                ),
            ),
            ("risk", "Risk", risk(&self.risk)),
        ];
        if let Some(a) = &self.attribution {
            parts.push(("attribution", "Attribution", attribution(a)));
        }
        if let Some(s) = &self.scenarios {
            parts.push(("scenarios", "Scenarios", scenarios(s)));
        }
        let nav: String = parts
            .iter()
            .map(|(anchor, label, _)| format!(r##"<a href="#{anchor}">{label}</a>"##))
            .collect();
        let sections: String = parts.into_iter().map(|(_, _, html)| html).collect();
        let s = &p.source;
        let benchmark = p.benchmark.as_ref().map_or_else(String::new, |b| {
            let mut chars = b.chars();
            let b: String = chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default();
            format!("<br>Benchmark: {}", esc(&b))
        });
        let brand = if name.is_some() {
            r#"<div class="brand">Portfolio report</div>"#
        } else {
            ""
        };
        let digest: String = s.digest.chars().take(12).collect();
        format!(
            r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="generator" content="Finalytics {version}"><title>{title}</title><style>{CSS}</style></head>
<body><div class="page">
<div class="top"><div><div class="logo"><img class="lg-light" alt="Finalytics" src="data:image/png;base64,{light}"><img class="lg-dark" alt="Finalytics" src="data:image/png;base64,{dark}"></div>
<h1>{title}</h1>{brand}</div>
<div class="meta">As of {as_of} · from {start}<br>{accounts} · {instruments} · {transactions} · {ccy}{benchmark}</div></div>
<nav>{nav}</nav>
{sections}
<div class="foot">Every figure is computed by Finalytics {version} from the portfolio's transactions and market data; the same inputs give the same report, identified by the input digest <code>{digest}</code>.</div>
</div></body></html>
"#,
            version = esc(&p.version),
            title = esc(title),
            light = base64(LOGO_LIGHT),
            dark = base64(LOGO_DARK),
            as_of = date(p.as_of),
            start = date(p.start),
            accounts = plural(s.accounts, "account"),
            instruments = plural(s.instruments, "instrument"),
            transactions = plural(s.transactions, "transaction"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_encoding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn axis_ticks_are_round_and_cover_the_range() {
        assert_eq!(ticks(0.0, 0.97, 4), [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(
            ticks(-0.017, 0.202, 5),
            [-0.05, 0.0, 0.05, 0.1, 0.15, 0.2, 0.25]
        );
        let t = ticks(-0.123, 0.0, 4);
        assert!(t[0] <= -0.123 && *t.last().unwrap_or(&1.0) >= 0.0);
        assert_eq!(ticks(5.0, 5.0, 4).len(), 2);
    }

    #[test]
    fn labels_overlap_neither_points_nor_each_other_and_stay_inside() {
        let points = [
            (100.0, 100.0),
            (150.0, 102.0),
            (400.0, 20.0),
            (790.0, 200.0),
        ];
        let mut labels = Labels::new((0.0, 0.0, 800.0, 240.0), &points, 6.0);
        let mut boxes = Vec::new();
        for (k, (x, y)) in points.iter().enumerate() {
            let text = format!("Asset number {k}");
            let (lx, ly, anchor) = labels.place(*x, *y, &text, 13.0).unwrap();
            let width = text.chars().count() as f64 * 13.0 * 0.56;
            let x0 = match anchor {
                "start" => lx,
                "end" => lx - width,
                _ => lx - width / 2.0,
            };
            assert!(x0 >= 0.0 && x0 + width <= 800.0, "{text} leaves the area");
            boxes.push((x0, ly - 10.4, x0 + width, ly + 2.6));
        }
        // The label at the right edge goes to the left of its point.
        assert!(boxes[3].2 <= 790.0);
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(a.2 <= b.0 || a.0 >= b.2 || a.3 <= b.1 || a.1 >= b.3);
            }
            for (x, y) in points {
                assert!(a.2 <= x - 6.0 || a.0 >= x + 6.0 || a.3 <= y - 6.0 || a.1 >= y + 6.0);
            }
        }
        // With no room left, a label is left out.
        let mut crowded = Labels::new((0.0, 0.0, 60.0, 20.0), &[(30.0, 10.0)], 6.0);
        assert_eq!(crowded.place(30.0, 10.0, "A long name", 13.0), None);
    }

    #[test]
    fn a_theme_sets_the_page_colours() {
        let page = "<!doctype html>\n<html lang=\"en\"><head></head><body></body></html>";
        assert!(
            Theme::Light
                .apply(page)
                .contains(r#"<html lang="en" data-theme="light">"#)
        );
        assert!(Theme::Dark.apply(page).contains(r#"data-theme="dark""#));
        assert_eq!(Theme::Auto.apply(page), page);
        assert!(CSS.contains(r#":root[data-theme="dark"]"#));
    }

    #[test]
    fn wide_charts_keep_a_minimum_width_on_narrow_screens() {
        assert!(svg_open(880.0, 200.0).starts_with(r#"<svg class="chart""#));
        assert!(!svg_open(300.0, 200.0).contains("chart"));
        assert!(CSS.contains("svg.chart{min-width:640px}"));
    }

    #[test]
    fn labels_on_a_fill_follow_its_series() {
        assert_eq!(on("var(--s3)"), "var(--on-s3)");
        assert_eq!(on("var(--muted-fill)"), "var(--on-muted-fill)");
        for k in 1..=8 {
            assert_eq!(CSS.matches(&format!("--on-s{k}:")).count(), 3, "s{k}");
        }
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(
            esc(r#"<a href="x">&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;"
        );
    }
}
