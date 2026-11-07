//! The screening report as a self-contained HTML page, in the style of the other reports:
//! inline styles and SVG, no scripts, light and dark themes.

use std::fmt::Write as _;

use finalytics_risk::screening::{FieldValue, MeasureUnit};

use crate::backtest_html::{Frame, frame, notes_table, section};
use crate::html::{card, esc, tiles};
use crate::screening::{
    MeasureSpread, ScreenReport, cell, count, label_of, rule_words, shown, sort_words,
};
use crate::text::date;

/// The screening page's styles, added to the reports' own.
const STYLE: &str = include_str!("../assets/screening.css");

/// A box plot of one measure's spread, with a mark at each value the rules compare it with.
fn box_plot(s: &MeasureSpread) -> String {
    let (Some(min), Some(q1), Some(median), Some(q3), Some(max)) =
        (s.min, s.lower_quartile, s.median, s.upper_quartile, s.max)
    else {
        return String::new();
    };
    if s.unit == Some(MeasureUnit::Flag) {
        return String::new();
    }
    let shown_thresholds: Vec<f64> = s
        .thresholds
        .iter()
        .copied()
        .filter(|t| t.is_finite())
        .collect();
    let lo = shown_thresholds.iter().copied().fold(min, f64::min);
    let hi = shown_thresholds.iter().copied().fold(max, f64::max);
    let span = if hi > lo { hi - lo } else { 1.0 };
    let (w, pad) = (180.0, 4.0);
    let x = |v: f64| pad + (v - lo) / span * (w - 2.0 * pad);
    let mut svg = format!(
        r#"<svg viewBox="0 0 {w} 22" role="img" aria-label="From {} to {}, median {}">"#,
        esc(&shown(Some(min), s.unit)),
        esc(&shown(Some(max), s.unit)),
        esc(&shown(Some(median), s.unit))
    );
    if s.unit == Some(MeasureUnit::Fraction) && lo < 0.0 && hi > 0.0 {
        let _ = write!(
            svg,
            r#"<line x1="{z:.1}" x2="{z:.1}" y1="2" y2="20" class="grid"/>"#,
            z = x(0.0)
        );
    }
    let _ = write!(
        svg,
        r#"<line x1="{:.1}" x2="{:.1}" y1="11" y2="11" style="stroke:var(--axis);stroke-width:1.5"/><rect x="{:.1}" y="5" width="{:.1}" height="12" rx="2" style="fill:var(--s1);opacity:.35"/><line x1="{m:.1}" x2="{m:.1}" y1="4" y2="18" style="stroke:var(--s1);stroke-width:2"/>"#,
        x(min),
        x(max),
        x(q1),
        (x(q3) - x(q1)).max(1.0),
        m = x(median)
    );
    for t in shown_thresholds {
        let _ = write!(
            svg,
            r#"<line x1="{v:.1}" x2="{v:.1}" y1="1" y2="21" style="stroke:var(--s2);stroke-width:2"><title>A rule's threshold: {}</title></line>"#,
            esc(&shown(Some(t), s.unit)),
            v = x(t)
        );
    }
    svg.push_str("</svg>");
    svg
}

impl ScreenReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own):
    /// the coverage, the rules, every match and the spread of each measure, in the reader's
    /// light or dark theme.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let cov = &self.coverage;
        let title = self.title(name);
        let as_of = date(c.as_of);
        let meta = format!(
            "as of {}<br>{} · {}",
            esc(&as_of),
            esc(&format!("{} securities", count(c.securities))),
            esc(&format!(
                "{} {}",
                count(cov.matched),
                if cov.matched == 1 { "match" } else { "matches" }
            ))
        );
        let mut nav: Vec<(&str, &str)> = vec![("summary", "Summary")];
        if !self.rules.is_empty() {
            nav.push(("rules", "Rules"));
        }
        nav.extend([
            ("matches", "Matches"),
            ("spread", "Spread"),
            ("method", "How it was measured"),
        ]);
        let mut body = String::new();

        // The summary.
        let measured_sub =
            (cov.too_short > 0).then(|| format!("{} too short", count(cov.too_short)));
        let passed_sub = Some(if self.rules.is_empty() {
            "no rules".to_owned()
        } else {
            format!("every one of {}", self.rules.len())
        });
        let matched_sub = match (&c.sort, c.limit) {
            (Some(s), Some(l)) => Some(format!("top {} {}", count(l), sort_words(s))),
            (Some(s), None) => Some(sort_words(s)),
            (None, Some(l)) => Some(format!("the first {}", count(l))),
            (None, None) => None,
        };
        let items = vec![
            (
                "Securities".to_owned(),
                count(cov.securities),
                c.first_date.map(|d| format!("closes from {}", date(d))),
            ),
            (
                "Measured".to_owned(),
                count(cov.measured + cov.too_short),
                measured_sub,
            ),
            (
                "Stale".to_owned(),
                count(cov.stale),
                Some(format!("no close in the {} days", c.max_carry_days)),
            ),
            ("Passed".to_owned(), count(cov.passed), passed_sub),
            ("Matches".to_owned(), count(cov.matched), matched_sub),
        ];
        let mut summary = tiles(&items);
        let parts = self.parts().join(" · ");
        let _ = write!(
            summary,
            r#"<p class="note">{}</p><p class="note">{}</p>"#,
            esc(&self.coverage_note()),
            esc(&parts)
        );
        body.push_str(&section(
            "summary",
            "Summary",
            &format!("Each security on its own history, to {as_of}"),
            &summary,
        ));

        // The rules.
        if !self.rules.is_empty() {
            let base = (cov.measured + cov.too_short).max(1) as f64;
            let mut table = String::from(
                r#"<div class="tw"><table><thead><tr><th>Rule</th><th class="r">Passed</th><th class="r">Lacking</th><th class="r">With those before</th><th></th></tr></thead><tbody>"#,
            );
            for r in &self.rules {
                let share = r.remaining as f64 / base * 100.0;
                let _ = write!(
                    table,
                    r#"<tr><td>{}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td><div class="fw" title="{} of the {} measured"><div class="fb" style="width:{share:.1}%"></div></div></td></tr>"#,
                    esc(&rule_words(&r.rule)),
                    count(r.passed),
                    count(r.lacking),
                    count(r.remaining),
                    count(r.remaining),
                    count(cov.measured + cov.too_short),
                );
            }
            table.push_str("</tbody></table></div>");
            body.push_str(&section(
                "rules",
                "Rules",
                "Every match meets them all",
                &card(
                    "How many securities meet each rule",
                    &table,
                    Some(
                        "Counts are of the securities that are not stale. A security without \
                         the value a rule tests lacks it, and fails the rule. The bar is how \
                         many meet the rule and every rule above it.",
                    ),
                ),
            ));
        }

        // The matches.
        let mut matches = String::new();
        if self.matches.is_empty() {
            let _ = write!(
                matches,
                r#"<p>{}</p>"#,
                if self.rules.is_empty() {
                    "No security was measured."
                } else {
                    "No security meets every rule."
                }
            );
        } else {
            matches.push_str(
                r#"<div class="tw"><table class="mt"><thead><tr><th class="r">#</th><th>Security</th>"#,
            );
            let right: Vec<bool> = (0..self.columns.len())
                .map(|k| {
                    !self
                        .matches
                        .iter()
                        .any(|m| matches!(m.values.get(k), Some(Some(FieldValue::Text(_)))))
                })
                .collect();
            for (column, r) in self.columns.iter().zip(&right) {
                let what = if column.field {
                    "A field given with the securities".to_owned()
                } else {
                    column.name.clone()
                };
                let _ = write!(
                    matches,
                    r#"<th{} title="{}">{}</th>"#,
                    if *r { r#" class="r""# } else { "" },
                    esc(&what),
                    esc(&column.label)
                );
            }
            matches.push_str("</tr></thead><tbody>");
            for m in &self.matches {
                let _ = write!(
                    matches,
                    r#"<tr><td class="rk">{}</td><td class="id">{}</td>"#,
                    m.rank,
                    esc(&m.id)
                );
                for ((column, value), r) in self.columns.iter().zip(&m.values).zip(&right) {
                    let _ = write!(
                        matches,
                        r#"<td{}>{}</td>"#,
                        if *r { r#" class="r""# } else { "" },
                        esc(&cell(value, column.unit))
                    );
                }
                matches.push_str("</tr>");
            }
            matches.push_str("</tbody></table></div>");
        }
        let matches_title = match &c.sort {
            Some(s) => format!("Sorted {}", sort_words(s)),
            None => "In the universe's order".to_owned(),
        };
        let shown_columns = self
            .columns
            .iter()
            .map(|k| label_of(&k.name, k.field))
            .collect::<Vec<_>>()
            .join(", ");
        body.push_str(&section(
            "matches",
            "Matches",
            &format!(
                "{} of {} securities",
                count(cov.matched),
                count(c.securities)
            ),
            &card(
                &matches_title,
                &matches,
                (!self.columns.is_empty())
                    .then(|| format!("Shown: {shown_columns}. The results hold every measure."))
                    .as_deref(),
            ),
        ));

        // The spread.
        let mut spread = String::from(
            r#"<div class="tw"><table class="sp"><thead><tr><th>Measure</th><th class="r">Have</th><th class="r">Lacking</th><th class="r">Lowest</th><th class="r">25%</th><th class="r">Median</th><th class="r">75%</th><th class="r">Highest</th><th>Spread</th></tr></thead><tbody>"#,
        );
        let mut group = "";
        for s in &self.spread {
            if s.group != group {
                group = &s.group;
                let _ = write!(
                    spread,
                    r#"<tr class="grp"><th colspan="9">{}</th></tr>"#,
                    esc(group)
                );
            }
            let label = if s.thresholds.is_empty() {
                esc(&s.label)
            } else {
                format!(
                    r#"{} <span class="rl" title="A rule tests it">•</span>"#,
                    esc(&s.label)
                )
            };
            let _ = write!(
                spread,
                r#"<tr><td title="{}">{label}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td class="r">{}</td><td class="bx">{}</td></tr>"#,
                esc(&s.name),
                count(s.count),
                count(s.lacking),
                esc(&shown(s.min, s.unit)),
                esc(&shown(s.lower_quartile, s.unit)),
                esc(&shown(s.median, s.unit)),
                esc(&shown(s.upper_quartile, s.unit)),
                esc(&shown(s.max, s.unit)),
                box_plot(s)
            );
        }
        spread.push_str("</tbody></table></div>");
        body.push_str(&section(
            "spread",
            "Spread across the universe",
            "Where each measure falls, so that a threshold can be set where it bites",
            &card(
                "Each measure over the securities that are not stale",
                &spread,
                Some(
                    "The box runs from the lower to the upper quartile, with the median as a \
                     line and the whiskers to the lowest and highest values; a mark in the \
                     second colour is a value a rule compares with.",
                ),
            ),
        ));

        // How it was measured.
        let notes = self.method_notes();
        let rows: Vec<(&str, String)> =
            notes.iter().map(|(t, x)| (t.as_str(), x.clone())).collect();
        let mut method = notes_table(&rows);
        let issues = c.warnings.issues();
        if !issues.is_empty() {
            method.push_str(r#"<h3 style="margin-top:16px">Warnings</h3><ul class="warn">"#);
            for issue in issues {
                let _ = write!(method, "<li>{}</li>", esc(&issue.message));
            }
            method.push_str("</ul>");
        }
        body.push_str(&section(
            "method",
            "How it was measured",
            "",
            &card("Definitions", &method, None),
        ));

        let source = format!(
            "the closes of {} securities up to {as_of}",
            count(c.securities)
        );
        frame(
            &Frame {
                title: &title,
                brand: Some("Screening report"),
                meta: &meta,
                nav: &nav,
                version: &c.version,
                digest: &c.digest,
                source: &source,
                style: STYLE,
            },
            &body,
        )
    }
}
