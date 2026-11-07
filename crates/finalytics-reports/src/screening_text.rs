//! The screening report as text for printing, 80 columns wide.

use std::fmt;

use finalytics_risk::screening::FieldValue;

use crate::screening::{ScreenReport, cell, count, rule_words, shown, sort_words};
use crate::text::{
    L, Page, R, WIDTH, date, fit, pad_left, pad_right, plural, split_heading, width, wrap,
};

/// The matches the text lists; the page lists them all.
pub(crate) const TEXT_MATCHES: usize = 50;

/// The widest a security's identifier is shown.
const ID: usize = 14;

/// Text beside a label column `lw` wide, wrapped, the label on the first line only.
fn labelled(page: &mut Page, label: &str, lw: usize, text: &str) {
    for (k, line) in wrap(text, WIDTH - 4 - lw).into_iter().enumerate() {
        let label = if k == 0 { label } else { "" };
        page.line(format!("  {}  {line}", pad_right(label, lw)));
    }
}

/// A column of the matches as laid out: its heading on one or two lines, its width and its
/// alignment.
struct Laid {
    heading: (String, Option<String>),
    width: usize,
    right: bool,
    cells: Vec<String>,
}

impl ScreenReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own): the coverage, the rules, the top 50 matches and the spread of each measure.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let cov = &self.coverage;
        let mut page = Page::new(None);

        // The header.
        page.rule('━');
        let left = "FINALYTICS  ·  SCREEN";
        let right = format!("as of {}", date(c.as_of));
        page.line(format!(
            "{left}{}",
            pad_left(&right, WIDTH.saturating_sub(width(left)))
        ));
        page.line(self.title(name));
        let mut parts = vec![plural(cov.matched, "match").replace("matchs", "matches")];
        parts.extend(self.parts());
        page.parts(parts.join(" · "));
        page.rule('━');

        // Coverage.
        page.section("Coverage");
        page.pairs(&[
            ("Securities".to_owned(), count(cov.securities)),
            ("Measured".to_owned(), count(cov.measured)),
            ("Passed every rule".to_owned(), count(cov.passed)),
            ("Too short".to_owned(), count(cov.too_short)),
            ("Stale".to_owned(), count(cov.stale)),
            ("Matches".to_owned(), count(cov.matched)),
        ]);
        page.blank();
        page.note(&self.coverage_note());

        // The rules.
        if !self.rules.is_empty() {
            page.section("Rules");
            let heads = ["Passed", "Lacking", "With those before"];
            let numbers: Vec<[String; 3]> = self
                .rules
                .iter()
                .map(|r| [count(r.passed), count(r.lacking), count(r.remaining)])
                .collect();
            let nw: Vec<usize> = (0..3)
                .map(|k| {
                    numbers
                        .iter()
                        .map(|n| width(&n[k]))
                        .chain([width(heads[k])])
                        .max()
                        .unwrap_or(0)
                })
                .collect();
            let tw = WIDTH - 2 - nw.iter().map(|w| w + 2).sum::<usize>();
            let words: Vec<Vec<String>> = self
                .rules
                .iter()
                .map(|r| wrap(&rule_words(&r.rule), tw))
                .collect();
            let rw = words
                .iter()
                .flatten()
                .map(|l| width(l))
                .max()
                .unwrap_or(0)
                .max(4)
                .min(tw);
            let row = |text: &str, n: [&str; 3]| {
                format!(
                    "  {}  {}  {}  {}",
                    pad_right(text, rw),
                    pad_left(n[0], nw[0]),
                    pad_left(n[1], nw[1]),
                    pad_left(n[2], nw[2])
                )
            };
            page.line(row("Rule", heads));
            for (lines, n) in words.iter().zip(&numbers) {
                for (k, line) in lines.iter().enumerate() {
                    if k == 0 {
                        page.line(row(line, [&n[0], &n[1], &n[2]]));
                    } else {
                        page.line(format!("  {line}"));
                    }
                }
            }
            page.blank();
            page.note(
                "Each count is of the securities that are not stale. A security without the \
                 value a rule tests lacks it, and fails the rule.",
            );
        }

        // The matches.
        let heading = match &c.sort {
            Some(s) => format!("Matches, {}", sort_words(s)),
            None => "Matches".to_owned(),
        };
        page.section(&heading);
        if self.matches.is_empty() {
            page.note(if self.rules.is_empty() {
                "No security was measured."
            } else {
                "No security meets every rule."
            });
        } else {
            self.matches_table(&mut page);
        }

        // The spread, without the measures no security has.
        let spread = self.spread_shown();
        let absent = self.spread_absent();
        if !spread.is_empty() || absent.is_some() {
            page.section("Spread across the universe");
        }
        if !spread.is_empty() {
            let tested: Vec<&str> = self.rules.iter().map(|r| r.rule.name()).collect();
            let rows: Vec<Vec<String>> = spread
                .iter()
                .map(|s| {
                    let label = if tested.contains(&s.name.as_str()) {
                        format!("{} *", s.label)
                    } else {
                        s.label.clone()
                    };
                    vec![
                        label,
                        count(s.count),
                        shown(s.min, s.unit),
                        shown(s.lower_quartile, s.unit),
                        shown(s.median, s.unit),
                        shown(s.upper_quartile, s.unit),
                        shown(s.max, s.unit),
                    ]
                })
                .collect();
            page.table(
                &[
                    "Measure", "Have", "Lowest", "25%", "Median", "75%", "Highest",
                ],
                &rows,
                Some(&[L, R, R, R, R, R, R]),
            );
            page.blank();
            let mut note = String::from(
                "Of the securities that are not stale; Have counts those with a value.",
            );
            if !tested.is_empty() {
                note.push_str(" * A rule tests it.");
            }
            page.note(&note);
        }
        if let Some(absent) = &absent {
            page.note(absent);
        }

        // How it was measured.
        page.section("How it was measured");
        let notes = self.method_notes();
        let lw = notes.iter().map(|(t, _)| width(t)).max().unwrap_or(0);
        for (term, text) in &notes {
            labelled(&mut page, term, lw, text);
        }
        page.blank();
        let issues = c.warnings.issues();
        page.line(format!(
            "  Finalytics {} · inputs {} · {}",
            c.version,
            c.digest.chars().take(12).collect::<String>(),
            plural(issues.len(), "warning")
        ));
        for issue in issues.iter().take(3) {
            page.line(format!("  ! {}", fit(&issue.message, WIDTH - 4)));
        }
        if issues.len() > 3 {
            page.line(format!("  ! and {} more", issues.len() - 3));
        }
        page.finish()
    }

    /// The top matches, with as many of the report's columns as fit the width.
    fn matches_table(&self, page: &mut Page) {
        let shown_matches = &self.matches[..self.matches.len().min(TEXT_MATCHES)];
        let ranks: Vec<String> = shown_matches.iter().map(|m| m.rank.to_string()).collect();
        let ids: Vec<String> = shown_matches.iter().map(|m| fit(&m.id, ID)).collect();
        let rank_w = ranks.iter().map(|r| width(r)).max().unwrap_or(1).max(1);
        let id_w = ids
            .iter()
            .map(|i| width(i))
            .max()
            .unwrap_or(0)
            .max(width("Security"));
        let mut used = 2 + rank_w + 2 + id_w;
        let mut laid: Vec<Laid> = Vec::new();
        let mut left_out: Vec<&str> = Vec::new();
        for (k, column) in self.columns.iter().enumerate() {
            let cells: Vec<String> = shown_matches
                .iter()
                .map(|m| {
                    let value = m.values.get(k).cloned().flatten();
                    fit(&cell(&value, column.unit), 20)
                })
                .collect();
            let right = !shown_matches
                .iter()
                .any(|m| matches!(m.values.get(k), Some(Some(FieldValue::Text(_)))));
            let cw = cells.iter().map(|c| width(c)).max().unwrap_or(0);
            let label = &column.label;
            let heading = if width(label) > cw {
                match split_heading(label) {
                    (one, Some(two)) => (one, Some(two)),
                    (one, None) => (one, None),
                }
            } else {
                (label.clone(), None)
            };
            let hw = width(&heading.0).max(heading.1.as_deref().map_or(0, width));
            let w = cw.max(hw);
            if left_out.is_empty() && used + 2 + w <= WIDTH {
                used += 2 + w;
                laid.push(Laid {
                    heading,
                    width: w,
                    right,
                    cells,
                });
            } else {
                left_out.push(label);
            }
        }
        let two = laid.iter().any(|l| l.heading.1.is_some());
        let line = |rank: &str, id: &str, cells: &dyn Fn(&Laid) -> String| {
            let mut text = format!("  {}  {}", pad_left(rank, rank_w), pad_right(id, id_w));
            for l in &laid {
                let value = cells(l);
                text.push_str("  ");
                text.push_str(&if l.right {
                    pad_left(&value, l.width)
                } else {
                    pad_right(&value, l.width)
                });
            }
            text
        };
        if two {
            page.line(line("", "", &|l| {
                if l.heading.1.is_some() {
                    l.heading.0.clone()
                } else {
                    String::new()
                }
            }));
            page.line(line("#", "Security", &|l| {
                l.heading.1.clone().unwrap_or_else(|| l.heading.0.clone())
            }));
        } else {
            page.line(line("#", "Security", &|l| l.heading.0.clone()));
        }
        for (k, (rank, id)) in ranks.iter().zip(&ids).enumerate() {
            page.line(line(rank, id, &|l| l.cells[k].clone()));
        }
        let mut notes = Vec::new();
        if self.matches.len() > TEXT_MATCHES {
            notes.push(format!(
                "The top {TEXT_MATCHES} of {} matches; the page lists them all.",
                count(self.matches.len())
            ));
        }
        if !left_out.is_empty() {
            notes.push(format!(
                "{} on the page, for want of width here.",
                left_out.join(", ")
            ));
        }
        if !notes.is_empty() {
            page.blank();
            page.note(&notes.join(" "));
        }
    }
}

impl fmt::Display for ScreenReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}
