//! The planning report as text for printing, 80 columns wide.

use std::fmt;

use finalytics_plan::Mix;

use crate::planning::{
    IMPORTANT, Kind, NOT_ADVICE, PlanReport, a_chance, left_or_out, levers, levers_heading,
    levers_note, method_notes, outcome_money, pc, pc1, steps, steps_note, stress_labels,
    stress_meaning, stress_rows,
};
use crate::text::{L, Page, R, WIDTH, bar, fit, pad_left, pad_right, width, wrap};

/// Text beside a label column `lw` wide, wrapped, the label on the first line only.
fn labelled(page: &mut Page, label: &str, lw: usize, text: &str) {
    for (k, line) in wrap(text, WIDTH - 4 - lw).into_iter().enumerate() {
        let label = if k == 0 { label } else { "" };
        page.line(format!("  {}  {line}", pad_right(label, lw)));
    }
}

impl PlanReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own, or the goal's).
    pub fn to_text(&self, name: Option<&str>) -> String {
        let f = &self.figures;
        let c = &self.context;
        let m = self.money();
        let wanted = self.wanted();
        let end = self.end_long();
        let mut page = Page::new(None);

        // The header.
        page.rule('━');
        let left = "FINALYTICS  ·  PLANNING";
        let right = format!(
            "{} to {}",
            f.start.short_name(),
            self.last_month().short_name()
        );
        page.line(format!(
            "{left}{}",
            pad_left(&right, WIDTH.saturating_sub(width(left)))
        ));
        page.line(self.title(name));
        let goal = match self.kind() {
            Kind::Target { amount, by } => format!("{} by {}", m.whole(amount), by.long_name()),
            Kind::Payments {
                amount,
                every,
                start,
                end,
            } => format!(
                "{} a {}, {} to {}",
                m.whole(amount),
                every.noun(),
                start.short_name(),
                end.short_name()
            ),
            Kind::Projection => format!("Projection to {end}"),
        };
        let mix = c.mix.name().map_or_else(String::new, |n| format!("{n} · "));
        page.parts(format!(
            "{goal} · {mix}{} scenarios · today's money",
            crate::text::money(f.simulations as f64)
        ));
        page.rule('━');

        // Where the plan stands.
        page.section("Where you stand");
        let e = f.end_value;
        match (self.kind(), f.status, f.outcome.chance) {
            (Kind::Target { amount, by }, Some(status), Some(chance)) => {
                page.note(&format!(
                    "{}: {} of reaching {} by {}.",
                    status.label(),
                    a_chance(chance),
                    m.whole(amount),
                    by.long_name()
                ));
                let mut sub = format!("You want at least {}.", pc(wanted));
                if let Some(s) = f.shortfall {
                    sub.push_str(&format!(
                        " If you fall short, it is usually by about {}.",
                        m.about(s)
                    ));
                }
                page.note(&sub);
            }
            (
                Kind::Payments {
                    amount,
                    every,
                    start,
                    end,
                },
                Some(status),
                Some(chance),
            ) => {
                page.note(&format!(
                    "{}: {} of paying {} a {} in full from {} to {}. You want at least {}.",
                    status.label(),
                    a_chance(chance),
                    m.whole(amount),
                    every.noun(),
                    start.long_name(),
                    end.long_name(),
                    pc(wanted)
                ));
                if let Some(p) = &f.payments
                    && let (Some(when), Some(share)) = (p.runs_short, p.paid_when_short)
                {
                    page.note(&format!(
                        "When the money runs short, it is typically in {}; even then, about {} \
                         of the money planned is paid.",
                        when.long_name(),
                        pc(share)
                    ));
                }
            }
            _ => {
                let mut text = format!(
                    "In an average market, you would have about {} by {end}.",
                    m.about(e.p50)
                );
                if let Some(put) = self.put_in().filter(|p| *p > 0.0) {
                    text.push_str(&format!(" What you put in comes to {}.", m.whole(put)));
                }
                page.note(&text);
            }
        }
        page.blank();
        let payments = matches!(self.kind(), Kind::Payments { .. });
        page.line(if payments {
            format!("  By {end}")
        } else {
            format!("  By {end} you would have")
        });
        let p = f.payments.as_ref();
        let values = if payments {
            [
                left_or_out(m, e.p90, p.and_then(|p| p.runs_out_strong)),
                left_or_out(m, e.p50, p.and_then(|p| p.runs_out_average)),
                left_or_out(m, e.p10, p.and_then(|p| p.runs_out_weak)),
            ]
        } else {
            [m.about(e.p90), m.about(e.p50), m.about(e.p10)]
        };
        let labels = [
            "Strong market (1 in 10 does better)",
            "Average market",
            "Weak market (1 in 10 does worse)",
        ];
        let vw = values.iter().map(|v| width(v)).max().unwrap_or(0);
        for (label, value) in labels.iter().zip(&values) {
            page.line(format!(
                "    {}  {}",
                pad_right(label, 38),
                pad_left(value, vw)
            ));
        }

        // The plan.
        page.section("Your plan");
        let steps = steps(self);
        let ww = steps.iter().map(|s| width(&s.1)).max().unwrap_or(0).min(34);
        let aw = steps.iter().map(|s| width(&s.4)).max().unwrap_or(0);
        let what_w = WIDTH.saturating_sub(2 + ww + 2 + 2 + aw);
        for (_, when, what, detail, amount) in &steps {
            page.line(format!(
                "  {}  {}  {}",
                pad_right(&fit(when, ww), ww),
                pad_right(&fit(what, what_w), what_w),
                pad_left(amount, aw)
            ));
            for line in wrap(detail, what_w + 2 + aw) {
                page.line(format!("  {}  {line}", " ".repeat(ww)));
            }
        }
        page.blank();
        page.note(&steps_note(self));

        // The changes.
        if !matches!(self.kind(), Kind::Projection) && f.changes.is_some() {
            let up = f.changes.as_ref().is_some_and(|c| c.to_reach);
            let all = levers(self);
            let heading = if all.is_empty() {
                levers_heading(self).to_owned()
            } else if up {
                format!(
                    "{}: any one of these would give {}",
                    levers_heading(self),
                    a_chance(wanted)
                )
            } else {
                format!(
                    "{}: any one of these would still keep {}",
                    levers_heading(self),
                    a_chance(wanted)
                )
            };
            page.section(&heading);
            let lw = all.iter().map(|l| width(&l.0)).max().unwrap_or(0);
            let cw = all.iter().map(|l| width(&l.1)).max().unwrap_or(0);
            let dw = WIDTH.saturating_sub(2 + lw + 2 + cw + 2);
            for (label, change, detail) in &all {
                for (k, line) in wrap(detail, dw).into_iter().enumerate() {
                    if k == 0 {
                        page.line(format!(
                            "  {}  {}  {line}",
                            pad_right(label, lw),
                            pad_right(change, cw)
                        ));
                    } else {
                        page.line(format!("  {}  {line}", " ".repeat(lw + 2 + cw)));
                    }
                }
            }
            let note = levers_note(self);
            if !note.is_empty() {
                page.blank();
                page.note(&note);
            }
        }

        // When markets do worse.
        page.section("What if markets do worse?");
        let rows = stress_rows(self);
        let projection = matches!(self.kind(), Kind::Projection);
        let labels = stress_labels(self);
        let table: Vec<Vec<String>> = rows
            .iter()
            .zip(labels)
            .map(|((_, _, o), label)| {
                let mut row = vec![label.to_owned()];
                if !projection {
                    row.push(pc(o.chance.unwrap_or(0.0)));
                }
                row.push(outcome_money(self, o));
                row
            })
            .collect();
        let money_head = format!("Average market, {}", self.last_month().short_name());
        if projection {
            page.table(&["", &money_head], &table, Some(&[L, R]));
        } else {
            page.table(&["", "Chance", &money_head], &table, Some(&[L, R, R]));
        }
        if let Some((_, description, _)) = rows.get(2) {
            page.blank();
            page.note(&format!(
                "The crash: {}.",
                crate::backtest_html::lower_first(description)
            ));
        }
        if let Some(meaning) = stress_meaning(self) {
            page.blank();
            page.note(&meaning);
        }

        // The mix.
        page.section(&match c.mix.name() {
            Some(name) => format!("Your investment mix: {name}"),
            None => "Your investment mix".to_owned(),
        });
        let mut parts: Vec<&(String, f64)> = f.weights.iter().filter(|(_, w)| *w > 0.0).collect();
        parts.sort_by(|a, b| b.1.total_cmp(&a.1));
        let nw = parts
            .iter()
            .map(|(a, _)| width(a).min(24))
            .max()
            .unwrap_or(0);
        for (asset, w) in parts {
            page.line(format!(
                "  {}  {}  {}",
                pad_right(&fit(asset, 24), nw),
                bar(*w, 30),
                pad_left(&pc(*w), 4)
            ));
        }
        page.blank();
        match &c.mix {
            Mix::GlidePath { points, .. } => {
                let assets: Vec<&String> = c
                    .assets
                    .iter()
                    .filter(|a| points.values().any(|w| w.get(*a).is_some_and(|v| *v > 0.0)))
                    .collect();
                page.note(
                    "The weights move along a glide path, a straight line from each point to \
                     the next, and are rebalanced every month.",
                );
                let mut head: Vec<&str> = vec!["From"];
                head.extend(assets.iter().map(|a| a.as_str()));
                let rows: Vec<Vec<String>> = points
                    .iter()
                    .map(|(month, w)| {
                        let mut row = vec![month.short_name()];
                        row.extend(assets.iter().map(|a| pc(w.get(*a).copied().unwrap_or(0.0))));
                        row
                    })
                    .collect();
                page.table(&head, &rows, None);
            }
            _ => page.note("Rebalanced back to these weights every month."),
        }
        page.blank();
        page.pairs(&[
            (
                "Typical return a year".to_owned(),
                pc1(f.risk.typical_return),
            ),
            (
                "Worst year in 20".to_owned(),
                f.risk.worst_year.map_or_else(|| "–".to_owned(), pc),
            ),
            ("Largest fall".to_owned(), pc(-f.risk.largest_fall.abs())),
            ("Volatility".to_owned(), pc1(f.risk.volatility)),
        ]);
        if !f.alternatives.is_empty() {
            page.blank();
            let rows: Vec<Vec<String>> = self
                .mixes()
                .iter()
                .map(|(x, yours)| {
                    let name = if *yours && c.mix.name().is_some() {
                        format!("{} (yours)", x.name)
                    } else {
                        x.name.to_owned()
                    };
                    let (weak, average) = if payments {
                        (
                            left_or_out(m, x.end.p10, x.runs_out_weak),
                            left_or_out(m, x.end.p50, x.outcome.runs_out),
                        )
                    } else {
                        (m.about(x.end.p10), m.about(x.end.p50))
                    };
                    let mut row = vec![name];
                    if !projection {
                        row.push(pc(x.outcome.chance.unwrap_or(0.0)));
                    }
                    row.extend([
                        weak,
                        average,
                        x.risk.worst_year.map_or_else(|| "–".to_owned(), pc),
                    ]);
                    row
                })
                .collect();
            if projection {
                page.table(
                    &["Mix", "Weak market", "Average market", "Worst year"],
                    &rows,
                    None,
                );
            } else {
                page.table(
                    &[
                        "Mix",
                        "Chance",
                        "Weak market",
                        "Average market",
                        "Worst year",
                    ],
                    &rows,
                    None,
                );
            }
        }

        // How it was worked out.
        page.section("How we worked it out");
        let notes = method_notes(self, false);
        for (k, (term, text)) in notes.iter().enumerate() {
            if k > 0 {
                page.blank();
            }
            page.line(format!("  {term}"));
            for line in wrap(text, WIDTH - 4) {
                page.line(format!("    {line}"));
            }
        }
        page.blank();
        labelled(&mut page, "Important", 10, IMPORTANT);
        page.blank();
        labelled(&mut page, "Not advice", 10, NOT_ADVICE);
        page.blank();
        page.line(format!(
            "  Finalytics {} · inputs {}",
            c.version,
            c.digest.chars().take(12).collect::<String>()
        ));
        page.finish()
    }
}

impl fmt::Display for PlanReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}
