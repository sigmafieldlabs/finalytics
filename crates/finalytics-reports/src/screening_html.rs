//! The screening report as a self-contained HTML page, in the style of the other reports:
//! inline styles, no scripts, light and dark themes. It reads from the top: what the screen
//! found, how the rules narrow the universe, the matches, the near misses, where each rule's
//! threshold falls among the securities, every measure and a line on the data.

use std::collections::HashSet;
use std::fmt::Write as _;

use finalytics_risk::screening::{FieldValue, Order};

use crate::backtest_html::{Frame, frame, section};
use crate::html::{esc, tiles};
use crate::screening::{
    CLOSE_NOTE, ScreenReport, ThresholdStrip, column_text, count, counted, limit_text, on_scale,
    value_text, vs_average,
};
use crate::text::{date, split_heading};

/// The screening page's styles, added to the reports' own.
const STYLE: &str = include_str!("../assets/screening.css");

/// The most near misses the page lists.
const PAGE_MISSES: usize = 100;

/// The width in pixels the dots of a strip are spread over, which sets how many rows they
/// need; the strip itself takes the width of the page.
const STRIP_WIDTH: f64 = 600.0;

/// A dot's width in pixels, and the space a row of dots takes.
const DOT: f64 = 9.0;
const ROW: f64 = DOT + 1.0;

/// The most rows of dots a strip draws; with more, it draws columns of counts instead.
const ROWS: usize = 12;

/// The columns of counts of a strip with too many securities for dots, and their height.
const BINS: usize = 60;
const BIN_HEIGHT: f64 = 56.0;

/// A section with its heading and one card.
fn block(anchor: &str, title: &str, subtitle: &str, body: &str) -> String {
    section(
        anchor,
        title,
        subtitle,
        &format!(r#"<div class="card">{body}</div>"#),
    )
}

/// A column's heading on two lines, with "average" shortened: "Price vs<br>200-day avg.". A
/// space inside parentheses does not break: "RSI<br>(14 days)".
fn heading(label: &str) -> String {
    let mut inside = false;
    let kept: String = label
        .replace(" average", " avg.")
        .chars()
        .map(|c| {
            match c {
                '(' => inside = true,
                ')' => inside = false,
                _ => {}
            }
            if c == ' ' && inside { '\u{a0}' } else { c }
        })
        .collect();
    match split_heading(&kept) {
        (one, Some(two)) => format!("{}<br>{}", esc(&one), esc(&two)),
        (one, None) => esc(&one),
    }
    .replace('\u{a0}', " ")
}

impl ScreenReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own):
    /// how the rules narrow the universe, every match, the near misses, where each threshold
    /// falls, every measure and the data, in the reader's light or dark theme.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let cov = &self.coverage;
        let title = self.title(name);
        let as_of = date(c.as_of);
        let meta = format!(
            "prices to {}<br>{} · {}",
            esc(&as_of),
            esc(&counted(c.securities, "security", "securities")),
            esc(&counted(cov.matched, "match", "matches"))
        );
        let mut body = format!(r#"<p class="lede">{}</p>"#, esc(&self.summary()));
        body.push_str(&tiles(&self.tiles()));
        if !self.rules.is_empty() {
            body.push_str(&block(
                "funnel",
                "How the rules narrow the list",
                "Each bar is how many securities are left after that rule and the ones above it.",
                &self.funnel_html(),
            ));
        }
        let sorted = match (&c.sort, self.sort_label()) {
            (Some(s), Some(label)) => format!(
                "Sorted by {label}, {}",
                match s.order {
                    Order::Descending => "highest first",
                    Order::Ascending => "lowest first",
                }
            ),
            _ => "In the universe's order".to_owned(),
        };
        body.push_str(&block("matches", "Matches", &sorted, &self.matches_html()));
        if !self.rules.is_empty() {
            body.push_str(&block(
                "near-misses",
                "Near misses",
                "Securities that fail just one rule, and by how much",
                &self.misses_html(),
            ));
        }
        let measures = self.measures_html();
        if self.strips.is_empty() {
            body.push_str(&block("measures", "Measures", "", &measures));
        } else {
            let mut strips = self.strips_html();
            strips.push_str(&measures);
            body.push_str(&block(
                "limits",
                "Where the limits fall",
                "Each dot is a security. The shaded side of the line passes the rule.",
                &strips,
            ));
        }
        body.push_str(&block(
            "data",
            "Data",
            "",
            &format!(r#"<p class="dline">{}</p>"#, esc(&self.data_line())),
        ));
        let source = format!(
            "the closes of {} up to {as_of}",
            counted(c.securities, "security", "securities")
        );
        frame(
            &Frame {
                title: &title,
                brand: Some("Screening report"),
                meta: &meta,
                nav: &[],
                version: &c.version,
                digest: &c.digest,
                source: &source,
                style: STYLE,
            },
            &body,
        )
    }

    /// The funnel: one bar for each step, as wide as the share of the universe left after it.
    fn funnel_html(&self) -> String {
        let total = self.coverage.securities.max(1) as f64;
        let steps = self.steps();
        let last = steps.len() - 1;
        let limited = self.coverage.matched < self.coverage.passed;
        let mut out = String::from(r#"<div class="fn">"#);
        for (k, step) in steps.iter().enumerate() {
            let class = match k {
                0 => "fn-row fn-first",
                _ if k == last && limited => "fn-row fn-last",
                _ => "fn-row",
            };
            let _ = write!(
                out,
                r#"<div class="{class}"><div class="fn-l"><span class="fn-w">{}</span>"#,
                esc(&step.words)
            );
            if let Some(alone) = step.alone {
                let mut extra = format!(
                    "{} of {} pass this rule on its own",
                    count(alone),
                    count(self.coverage.securities)
                );
                if let Some(lacking) = &step.lacking {
                    let _ = write!(extra, "; {lacking}");
                }
                let _ = write!(out, r#"<span class="fn-x">{}</span>"#, esc(&extra));
            }
            let _ = write!(
                out,
                r#"</div><div class="fn-b"><div class="fn-f" style="width:{:.1}%"></div><span class="fn-n">{}</span></div></div>"#,
                step.left as f64 / total * 100.0,
                count(step.left)
            );
        }
        out.push_str("</div>");
        out
    }

    /// Every match, with its name and sector when given, the measure sorted by and those the
    /// rules test, a mark beside a value close to a threshold, and the securities beyond the
    /// limit under the table.
    fn matches_html(&self) -> String {
        if self.matches.is_empty() {
            return format!(
                "<p>{}</p>",
                if self.rules.is_empty() {
                    "No security was measured."
                } else {
                    "No security passes every rule."
                }
            );
        }
        let sort = self.sort_column();
        let text: Vec<bool> = (0..self.columns.len())
            .map(|j| {
                self.matches
                    .iter()
                    .any(|m| matches!(m.values.get(j), Some(Some(FieldValue::Text(_)))))
            })
            .collect();
        // Numbers are right-aligned, and the measure sorted by stands out.
        let class = |j: usize| -> &'static str {
            match (text[j], Some(j) == sort) {
                (true, _) => "",
                (false, true) => r#" class="r em""#,
                (false, false) => r#" class="r""#,
            }
        };
        let mut out = String::from(
            r#"<div class="tw"><table class="mt"><thead><tr><th class="r">#</th><th>Ticker</th>"#,
        );
        for (j, column) in self.columns.iter().enumerate() {
            let head = if text[j] {
                esc(&column.label)
            } else {
                heading(&column.label)
            };
            let _ = write!(out, "<th{}>{head}</th>", class(j));
        }
        out.push_str("</tr></thead><tbody>");
        let mut marked = false;
        for m in &self.matches {
            let _ = write!(
                out,
                r#"<tr><td class="rk">{}</td><td class="id">{}</td>"#,
                m.rank,
                esc(&m.id)
            );
            for (j, (column, value)) in self.columns.iter().zip(&m.values).enumerate() {
                let mark = if m.near_threshold.contains(&j) {
                    marked = true;
                    r#"<span class="near" title="Close to the rule's limit">◆</span>"#
                } else {
                    ""
                };
                let class = if text[j] && column.name == "name" && column.field {
                    r#" class="nm""#
                } else {
                    class(j)
                };
                let _ = write!(
                    out,
                    "<td{class}>{mark}{}</td>",
                    esc(&column_text(column, value))
                );
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table></div>");
        if marked {
            let _ = write!(
                out,
                r#"<p class="note"><span class="near">◆</span> {}</p>"#,
                esc(CLOSE_NOTE)
            );
        }
        if let Some(also) = self.also_words() {
            let _ = write!(out, r#"<p class="note">{}</p>"#, esc(&also));
        }
        out
    }

    /// The near misses, the closest to passing first, in two columns.
    fn misses_html(&self) -> String {
        if self.near_misses.is_empty() {
            return r#"<p class="note">No security fails exactly one rule.</p>"#.to_owned();
        }
        let name = self.column_of("name", true);
        let mut out = String::from(r#"<ul class="nmiss">"#);
        for m in self.near_misses.iter().take(PAGE_MISSES) {
            let _ = write!(out, "<li><b>{}</b>", esc(&m.id));
            if let Some(Some(FieldValue::Text(n))) = name.and_then(|j| m.values.get(j)) {
                let _ = write!(out, r#" <span class="nm">{}</span>"#, esc(n));
            }
            let _ = write!(
                out,
                r#"<span class="nmx">{}</span></li>"#,
                esc(&self.miss_words(m))
            );
        }
        out.push_str("</ul>");
        if self.near_misses.len() > PAGE_MISSES {
            let _ = write!(
                out,
                r#"<p class="note">And {} more.</p>"#,
                count(self.near_misses.len() - PAGE_MISSES)
            );
        }
        out.push_str(r#"<p class="note">Closest to passing first.</p>"#);
        out
    }

    /// A strip for each measure the rules compare with a number, under a key.
    fn strips_html(&self) -> String {
        let matched: HashSet<&str> = self.matches.iter().map(|m| m.id.as_str()).collect();
        let mut out = String::from(
            r#"<div class="legend"><span class="key"><i class="kdm"></i>match</span><span class="key"><i class="kdp"></i>passes this rule</span><span class="key"><i class="kdf"></i>fails it</span><span class="key"><i class="kz"></i>passing side</span><span class="key"><i class="kt"></i>limit</span></div>"#,
        );
        for s in &self.strips {
            out.push_str(&self.strip_html(s, &matched));
        }
        out
    }

    /// One strip: a dot for each security on a line, close values in rows above and below
    /// one another, the rules' thresholds as lines and the side that passes shaded. Positions
    /// are in percent, so that the text keeps its size on any screen.
    fn strip_html(&self, s: &ThresholdStrip, matched: &HashSet<&str>) -> String {
        let Some(scale) = self.strip_scale(s) else {
            return String::new();
        };
        let x = |v: f64| scale.at(v) * 100.0;
        let points: Vec<(&str, f64, f64)> = s
            .points
            .iter()
            .map(|p| (p.id.as_str(), p.value, on_scale(&s.name, p.value)))
            .collect();
        // Each dot in the first row with room for it.
        let gap = ROW / STRIP_WIDTH * 100.0;
        let mut rows: Vec<f64> = Vec::new();
        let placed: Vec<usize> = points
            .iter()
            .map(|(_, _, v)| {
                let at = x(*v);
                match rows.iter().position(|last| at - last >= gap) {
                    Some(r) => {
                        rows[r] = at;
                        r
                    }
                    None => {
                        rows.push(at);
                        rows.len() - 1
                    }
                }
            })
            .collect();
        let dots = rows.len() <= ROWS;
        let height = if dots {
            rows.len().max(1) as f64 * ROW + 10.0
        } else {
            BIN_HEIGHT + 10.0
        };
        let class_of = |id: &str, raw: f64| {
            if matched.contains(id) {
                "dm"
            } else if self.strip_passes(s, raw) {
                "dp"
            } else {
                "df"
            }
        };
        let mut out = format!(r#"<div class="sv" style="height:{height:.0}px">"#);
        if let Some((a, b)) = scale.zone {
            let (a, b) = (x(a), x(b));
            let _ = write!(
                out,
                r#"<div class="pz" style="left:{a:.2}%;width:{:.2}%"></div>"#,
                b - a
            );
        }
        if dots {
            let mid = height / 2.0;
            for ((id, raw, v), row) in points.iter().zip(&placed) {
                // Rows alternate below and above the middle one.
                let off = row.div_ceil(2) as f64 * if row % 2 == 1 { 1.0 } else { -1.0 };
                let out_of_scale = *v < scale.lo || *v > scale.hi;
                let _ = write!(
                    out,
                    r#"<i class="d {}{}" style="left:{:.2}%;top:{:.1}px" title="{}: {}"></i>"#,
                    class_of(id, *raw),
                    if out_of_scale { " out" } else { "" },
                    x(*v),
                    mid + off * ROW,
                    esc(id),
                    esc(&value_text(&s.name, s.unit, Some(*raw)))
                );
            }
        } else {
            // Too many securities for dots: a column for each slice of the scale, as tall as
            // the securities in it, and a dot on the line for each match.
            let mut counts = vec![[0usize; 2]; BINS];
            for (_, raw, v) in &points {
                let b = ((scale.at(*v) * BINS as f64) as usize).min(BINS - 1);
                counts[b][usize::from(self.strip_passes(s, *raw))] += 1;
            }
            let top = counts.iter().map(|c| c[0] + c[1]).max().unwrap_or(1).max(1) as f64;
            let w = 100.0 / BINS as f64;
            for (b, c) in counts.iter().enumerate() {
                let mut base = height;
                for (class, n) in [("hp", c[1]), ("hf", c[0])] {
                    if n == 0 {
                        continue;
                    }
                    let h = n as f64 / top * BIN_HEIGHT;
                    base -= h;
                    let _ = write!(
                        out,
                        r#"<i class="hb {class}" style="left:{:.2}%;width:{:.2}%;top:{base:.1}px;height:{h:.1}px"></i>"#,
                        b as f64 * w,
                        w
                    );
                }
            }
            for (id, raw, v) in points.iter().filter(|p| matched.contains(p.0)) {
                let _ = write!(
                    out,
                    r#"<i class="d dm" style="left:{:.2}%;top:{:.1}px" title="{}: {}"></i>"#,
                    x(*v),
                    height - DOT / 2.0,
                    esc(id),
                    esc(&value_text(&s.name, s.unit, Some(*raw)))
                );
            }
        }
        // A value on the scale as the value it is.
        let raw = |v: f64| if vs_average(&s.name) { v + 1.0 } else { v };
        for t in &scale.limits {
            let _ = write!(
                out,
                r#"<div class="th" style="left:{:.2}%"><span>{}</span></div>"#,
                x(*t),
                esc(&limit_text(&s.name, s.unit, raw(*t)))
            );
        }
        out.push_str("</div>");
        let below = points.iter().filter(|p| p.2 < scale.lo).count();
        let above = points.iter().filter(|p| p.2 > scale.hi).count();
        let end = |v: f64| value_text(&s.name, s.unit, Some(raw(v)));
        let _ = write!(
            out,
            r#"<div class="sx"><span>{}{}</span><span>{}{}</span></div>"#,
            if below > 0 { "‹ " } else { "" },
            esc(&end(scale.lo)),
            esc(&end(scale.hi)),
            if above > 0 { " ›" } else { "" }
        );
        let passing = points.iter().filter(|p| self.strip_passes(s, p.1)).count();
        let mut sub = format!(
            "{} of {} pass",
            count(passing),
            count(self.coverage.securities)
        );
        let mut edges = Vec::new();
        if below > 0 {
            edges.push(format!("{} lower", count(below)));
        }
        if above > 0 {
            edges.push(format!("{} higher", count(above)));
        }
        if !edges.is_empty() {
            let _ = write!(sub, " · {} shown at the edge", edges.join(" and "));
        }
        format!(
            r#"<div class="st"><div class="st-h"><span class="st-n">{}</span><span class="st-c">{}</span></div>{out}</div>"#,
            esc(&self.strip_words(s)),
            esc(&sub)
        )
    }

    /// Every measure some security has, folded away: the lowest and highest values and the
    /// quartiles.
    fn measures_html(&self) -> String {
        let rows = self.measure_rows();
        let mut out = format!(
            r#"<details class="am"><summary>All measures ({})</summary><div class="tw"><table class="sp"><thead><tr><th>Measure</th><th class="r">Lowest</th><th class="r">25%</th><th class="r">Median</th><th class="r">75%</th><th class="r">Highest</th></tr></thead><tbody>"#,
            rows.len()
        );
        let mut group = "";
        for s in rows {
            if s.group != group {
                group = &s.group;
                let _ = write!(
                    out,
                    r#"<tr class="grp"><th colspan="6">{}</th></tr>"#,
                    esc(group)
                );
            }
            if let Some(yes) = s.yes {
                let _ = write!(
                    out,
                    r#"<tr><td>{}</td><td class="r" colspan="5">yes for {} of {}</td></tr>"#,
                    esc(&s.label),
                    count(yes),
                    count(s.count)
                );
                continue;
            }
            let _ = write!(out, "<tr><td>{}</td>", esc(&s.label));
            for q in [s.min, s.lower_quartile, s.median, s.upper_quartile, s.max] {
                let _ = write!(
                    out,
                    r#"<td class="r">{}</td>"#,
                    esc(&value_text(&s.name, s.unit, q))
                );
            }
            out.push_str("</tr>");
        }
        let _ = write!(
            out,
            r#"</tbody></table></div><p class="note">{}</p></details>"#,
            esc(&self.measures_note())
        );
        out
    }
}
