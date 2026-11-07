//! The screening report as text for printing, 80 columns wide: what the screen found, how the
//! rules narrow the universe, the top matches, the near misses, where each rule's threshold
//! falls among the securities and a line on the data.

use std::fmt;

use finalytics_risk::screening::{FieldValue, Order};

use crate::screening::{
    CLOSE_NOTE, ScreenColumn, ScreenReport, ThresholdStrip, column_text, count, limit_text,
    on_scale, value_text, vs_average,
};
use crate::text::{Page, WIDTH, date, fit, pad_left, pad_right, split_heading, width, wrap};

/// The matches the text lists; the page lists them all.
pub(crate) const TEXT_MATCHES: usize = 50;

/// The near misses the text lists.
const TEXT_MISSES: usize = 20;

/// The widest a security's identifier is shown.
const ID: usize = 14;

/// The narrowest a column of text, such as the sector, is shown.
const TEXT_COLUMN: usize = 8;

/// The widest a funnel step's words are shown on one line.
const STEP_WORDS: usize = 44;

/// The blocks of a strip's columns, from none to full.
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A note under a section, indented by two and wrapped to 76 characters.
fn note(page: &mut Page, text: &str) {
    for line in wrap(text, WIDTH - 4) {
        page.line(format!("  {line}"));
    }
}

/// A measure's heading in two short lines, so that the matches keep to 80 columns: what it is
/// and over what window.
fn short_heading(column: &ScreenColumn) -> (String, String) {
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    if column.field {
        return match split_heading(&column.label) {
            (one, Some(two)) => (one, two),
            (one, None) => (String::new(), one),
        };
    }
    match column.name.as_str() {
        "return_1w" => pair("Return", "1-week"),
        "return_1m" => pair("Return", "1-month"),
        "return_3m" => pair("Return", "3-month"),
        "return_6m" => pair("Return", "6-month"),
        "return_ytd" => pair("Return", "this year"),
        "return_1y" => pair("Return", "1-year"),
        "return_3y" => pair("Return/yr", "3-year"),
        "return_5y" => pair("Return/yr", "5-year"),
        "volatility" => pair("Volatility", "1-year"),
        "volatility_3y" => pair("Volatility", "3-year"),
        "downside_deviation" => pair("Downside", "volatility"),
        "max_drawdown" => pair("Drawdown", "1-year"),
        "max_drawdown_3y" => pair("Drawdown", "3-year"),
        "drawdown" => pair("Below", "its high"),
        "sharpe_ratio" => pair("Sharpe", "1-year"),
        "sharpe_ratio_3y" => pair("Sharpe", "3-year"),
        "sortino_ratio" => pair("Sortino", "ratio"),
        "calmar_ratio" => pair("Calmar", "ratio"),
        "value_at_risk" => pair("Value at", "risk"),
        "relative_return_6m" => pair("vs bench", "6-month"),
        "relative_return_1y" => pair("vs bench", "1-year"),
        "price_to_sma_20" => pair("vs 20-", "day avg"),
        "price_to_sma_50" => pair("vs 50-", "day avg"),
        "price_to_sma_200" => pair("vs 200-", "day avg"),
        "sma_50_above_200" => pair("50 above", "200-day"),
        "rsi_14" => pair("RSI", "14-day"),
        "macd_histogram" => pair("MACD", "histogram"),
        "from_52w_high" => pair("Below", "52w high"),
        "from_52w_low" => pair("Above", "52w low"),
        "volatility_20d" => pair("Volatility", "20-day"),
        "dividend_yield" => pair("Dividend", "yield"),
        "average_volume_20d" => pair("Volume", "20-day"),
        "volume_ratio" => pair("Volume vs", "20d avg"),
        "dollar_volume_20d" => pair("$ volume", "20-day"),
        "last_price" => pair("Last", "price"),
        _ => match split_heading(&column.label) {
            (one, Some(two)) => (one, two),
            (one, None) => (String::new(), one),
        },
    }
}

/// A sector in the text: the long names of the GICS sectors shortened.
fn short_sector(sector: &str) -> &str {
    match sector {
        "Consumer Staples" => "Staples",
        "Consumer Discretionary" => "Discretionary",
        "Information Technology" => "Technology",
        "Communication Services" => "Communication",
        other => other,
    }
}

/// A column of the matches as laid out: its heading on two lines, its cells with the marks
/// beside them, its width and its alignment.
struct Laid {
    label: String,
    heading: (String, String),
    cells: Vec<String>,
    marks: Vec<bool>,
    width: usize,
    right: bool,
}

impl ScreenReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own): how the rules narrow the universe, the top 50 matches, the near misses, where each
    /// threshold falls and a line on the data.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let mut page = Page::new(None);

        // The header, the summary and the tiles.
        page.rule('━');
        let left = "FINALYTICS  ·  SCREEN";
        let right = format!("prices to {}", date(c.as_of));
        page.line(format!(
            "{left}{}",
            pad_left(&right, WIDTH.saturating_sub(width(left)))
        ));
        page.line(self.title(name));
        page.rule('━');
        page.blank();
        for line in wrap(&self.summary(), WIDTH) {
            page.line(line);
        }
        page.blank();
        let tiles: Vec<String> = self
            .tiles()
            .into_iter()
            .map(|(label, value, _)| format!("{label} {value}"))
            .collect();
        let spaced = tiles.join("   ·   ");
        if width(&spaced) <= WIDTH - 2 {
            page.line(format!("  {spaced}"));
        } else {
            note(&mut page, &tiles.join(" · "));
        }

        if !self.rules.is_empty() {
            self.funnel_text(&mut page);
        }

        // The matches.
        let heading = match (&c.sort, self.sort_label()) {
            (Some(s), Some(label)) if s.order == Order::Ascending => {
                format!("Matches, by {label}, lowest first")
            }
            (Some(_), Some(label)) => format!("Matches, by {label}"),
            _ => "Matches".to_owned(),
        };
        page.section(&heading);
        if self.matches.is_empty() {
            note(
                &mut page,
                if self.rules.is_empty() {
                    "No security was measured."
                } else {
                    "No security passes every rule."
                },
            );
        } else {
            self.matches_text(&mut page);
        }

        if !self.rules.is_empty() {
            self.misses_text(&mut page);
        }
        if !self.strips.is_empty() {
            page.section("Where the limits fall");
            note(
                &mut page,
                "Each column of the strip counts the securities with a value there: the taller \
                 the mark, the more. ┃ is the limit; ━ marks the side that passes.",
            );
            for s in &self.strips {
                self.strip_text(&mut page, s);
            }
        }

        // The data.
        page.section("Data");
        note(&mut page, &self.data_line());
        page.blank();
        page.line(format!(
            "  Finalytics {} · inputs {}",
            c.version,
            c.digest.chars().take(12).collect::<String>()
        ));
        page.finish()
    }

    /// The funnel: a bar for each step, as long as the share of the universe left after it,
    /// with how many pass each rule on its own under it.
    fn funnel_text(&self, page: &mut Page) {
        page.section("How the rules narrow the list");
        let total = self.coverage.securities.max(1) as f64;
        let steps = self.steps();
        let nw = steps
            .iter()
            .map(|s| width(&count(s.left)))
            .max()
            .unwrap_or(0)
            .max(5);
        let lw = steps
            .iter()
            .map(|s| width(&s.words))
            .max()
            .unwrap_or(0)
            .min(STEP_WORDS);
        let bar_w = WIDTH - 2 - lw - 2 - nw;
        for step in &steps {
            let n =
                ((step.left as f64 / total * bar_w as f64).round_ties_even() as usize).min(bar_w);
            let words = wrap(&step.words, lw);
            for (k, line) in words.iter().enumerate() {
                if k + 1 < words.len() {
                    page.line(format!("  {line}"));
                } else {
                    page.line(format!(
                        "  {}  {}{}{}",
                        pad_right(line, lw),
                        "█".repeat(n),
                        " ".repeat(bar_w - n),
                        pad_left(&count(step.left), nw)
                    ));
                }
            }
            if let Some(alone) = step.alone {
                let extra = format!("{} pass this rule on its own", count(alone));
                for line in wrap(&extra, WIDTH - 4 - lw) {
                    page.line(format!("  {}  {line}", " ".repeat(lw)));
                }
            }
        }
        page.blank();
        note(
            page,
            "Each bar is how many are left after that rule and the ones above it.",
        );
    }

    /// The top matches: the security, its sector and the other text fields the rules test as
    /// far as room allows, the measure sorted by and those the rules test, a mark beside a
    /// value close to a threshold.
    fn matches_text(&self, page: &mut Page) {
        let shown = &self.matches[..self.matches.len().min(TEXT_MATCHES)];
        let ranks: Vec<String> = shown.iter().map(|m| m.rank.to_string()).collect();
        let ids: Vec<String> = shown.iter().map(|m| fit(&m.id, ID)).collect();
        let rank_w = ranks.iter().map(|r| width(r)).max().unwrap_or(1).max(1);
        let id_w = ids
            .iter()
            .map(|i| width(i))
            .max()
            .unwrap_or(0)
            .max(width("Ticker"));
        let text: Vec<bool> = (0..self.columns.len())
            .map(|j| {
                shown
                    .iter()
                    .any(|m| matches!(m.values.get(j), Some(Some(FieldValue::Text(_)))))
            })
            .collect();
        let laid_out = |j: usize| -> Laid {
            let column = &self.columns[j];
            let cells: Vec<String> = shown
                .iter()
                .map(|m| {
                    let value = m.values.get(j).cloned().flatten();
                    let cell = column_text(column, &value);
                    if column.name == "sector" && column.field {
                        short_sector(&cell).to_owned()
                    } else {
                        cell
                    }
                })
                .collect();
            let heading = if text[j] {
                (String::new(), column.label.clone())
            } else {
                short_heading(column)
            };
            let width = cells
                .iter()
                .map(|c| width(c))
                .chain([width(&heading.0), width(&heading.1)])
                .max()
                .unwrap_or(0);
            Laid {
                label: column.label.clone(),
                heading,
                marks: shown
                    .iter()
                    .map(|m| m.near_threshold.contains(&j))
                    .collect(),
                cells,
                width,
                right: !text[j],
            }
        };
        // Room for the columns after the rank and the security; a mark beside a value in the
        // last column takes one more character.
        let mut room = WIDTH - 2 - rank_w - 2 - id_w;
        let mut left_out: Vec<String> = Vec::new();
        let mut numbers: Vec<Laid> = Vec::new();
        for j in (0..self.columns.len()).filter(|j| !text[*j]) {
            let l = laid_out(j);
            if left_out.is_empty() && l.width + 2 <= room {
                room -= l.width + 2;
                numbers.push(l);
            } else {
                left_out.push(self.columns[j].label.clone());
            }
        }
        if numbers.last().is_some_and(|l| l.marks.iter().any(|m| *m)) {
            if room == 0
                && let Some(l) = numbers.pop()
            {
                room += l.width + 2;
                left_out.insert(0, l.label);
            }
            room = room.saturating_sub(1);
        }
        let mut words: Vec<Laid> = Vec::new();
        for j in (0..self.columns.len()).filter(|j| text[*j] && self.columns[*j].name != "name") {
            let mut l = laid_out(j);
            if room < TEXT_COLUMN + 2 {
                left_out.push(self.columns[j].label.clone());
                continue;
            }
            l.width = l.width.min(room - 2);
            room -= l.width + 2;
            words.push(l);
        }
        let laid: Vec<Laid> = words.into_iter().chain(numbers).collect();
        let line = |rank: &str, id: &str, cell: &dyn Fn(&Laid) -> String| -> String {
            let mut text = format!("  {}  {}", pad_left(rank, rank_w), pad_right(id, id_w));
            for l in &laid {
                let value = fit(&cell(l), l.width);
                text.push_str("  ");
                text.push_str(&if l.right {
                    pad_left(&value, l.width)
                } else {
                    pad_right(&value, l.width)
                });
            }
            text
        };
        page.line(line("", "", &|l| l.heading.0.clone()));
        page.line(line("#", "Ticker", &|l| l.heading.1.clone()));
        for (k, (rank, id)) in ranks.iter().zip(&ids).enumerate() {
            // A mark sits in the space just after its value.
            let mut chars: Vec<char> = line(rank, id, &|l| l.cells[k].clone()).chars().collect();
            let mut at = 2 + rank_w + 2 + id_w;
            for l in &laid {
                at += 2 + l.width;
                if l.marks[k] {
                    if chars.len() <= at {
                        chars.resize(at + 1, ' ');
                    }
                    chars[at] = '◆';
                }
            }
            page.line(chars.into_iter().collect::<String>());
        }
        let mut notes = Vec::new();
        if laid.iter().any(|l| l.marks.iter().any(|m| *m)) {
            notes.push(format!("◆ {CLOSE_NOTE}"));
        }
        if let Some(also) = self.also_words() {
            notes.push(also);
        }
        if self.matches.len() > TEXT_MATCHES {
            notes.push(format!(
                "The top {TEXT_MATCHES} of {} matches; the page lists them all.",
                count(self.matches.len())
            ));
        }
        if !left_out.is_empty() {
            notes.push(format!(
                "Not shown here for lack of room: {}; the page shows them.",
                left_out.join(", ")
            ));
        }
        if !notes.is_empty() {
            page.blank();
            for text in notes {
                note(page, &text);
            }
        }
    }

    /// The securities that fail just one rule, the closest to passing first.
    fn misses_text(&self, page: &mut Page) {
        page.section("Near misses: securities that fail just one rule");
        if self.near_misses.is_empty() {
            note(page, "No security fails exactly one rule.");
            return;
        }
        let shown = &self.near_misses[..self.near_misses.len().min(TEXT_MISSES)];
        let iw = shown
            .iter()
            .map(|m| width(&fit(&m.id, ID)))
            .max()
            .unwrap_or(0);
        for m in shown {
            for (k, line) in wrap(&self.miss_words(m), WIDTH - 4 - iw)
                .into_iter()
                .enumerate()
            {
                let id = if k == 0 {
                    fit(&m.id, ID)
                } else {
                    String::new()
                };
                page.line(format!("  {}  {line}", pad_right(&id, iw)));
            }
        }
        page.blank();
        if self.near_misses.len() > shown.len() {
            note(
                page,
                &format!(
                    "And {} more. Closest to passing first.",
                    count(self.near_misses.len() - shown.len())
                ),
            );
        } else {
            note(page, "Closest to passing first.");
        }
    }

    /// One strip: a column of blocks for each slice of the scale, as tall as the securities in
    /// it, over a line that marks each threshold and the side that passes.
    fn strip_text(&self, page: &mut Page, s: &ThresholdStrip) {
        let Some(scale) = self.strip_scale(s) else {
            return;
        };
        let sw = WIDTH - 4;
        let col = |v: f64| ((scale.at(v) * (sw - 1) as f64 + 0.5) as usize).min(sw - 1);
        let mut counts = vec![0usize; sw];
        for p in &s.points {
            counts[col(on_scale(&s.name, p.value))] += 1;
        }
        let top = counts.iter().copied().max().unwrap_or(1).max(1) as f64;
        let bars: String = counts
            .iter()
            .map(|n| match n {
                0 => BLOCKS[0],
                n => BLOCKS[((*n as f64 / top * 8.0).round_ties_even() as usize).clamp(1, 8)],
            })
            .collect();
        let limits: Vec<usize> = scale.limits.iter().map(|t| col(*t)).collect();
        let zone = scale
            .zone
            .map(|(a, b)| (col(a.max(scale.lo)), col(b.min(scale.hi))));
        let axis: String = (0..sw)
            .map(|i| {
                if limits.contains(&i) {
                    '┃'
                } else if zone.is_some_and(|(a, b)| a <= i && i <= b) {
                    '━'
                } else {
                    '─'
                }
            })
            .collect();
        // The thresholds under their marks, and the ends of the scale where there is room.
        let raw = |v: f64| if vs_average(&s.name) { v + 1.0 } else { v };
        let mut labels: Vec<char> = vec![' '; sw];
        for (c, t) in limits.iter().zip(&scale.limits) {
            let label: Vec<char> = limit_text(&s.name, s.unit, raw(*t)).chars().collect();
            let start = c
                .saturating_sub(label.len() / 2)
                .min(sw.saturating_sub(label.len()));
            for (k, ch) in label.into_iter().enumerate() {
                if let Some(slot) = labels.get_mut(start + k) {
                    *slot = ch;
                }
            }
        }
        let end =
            |v: f64| -> Vec<char> { value_text(&s.name, s.unit, Some(raw(v))).chars().collect() };
        let (lo, hi) = (end(scale.lo), end(scale.hi));
        if lo.len() < sw && labels[..=lo.len()].iter().all(|c| *c == ' ') {
            labels[..lo.len()].copy_from_slice(&lo);
        }
        if hi.len() < sw && labels[sw - hi.len() - 1..].iter().all(|c| *c == ' ') {
            labels[sw - hi.len()..].copy_from_slice(&hi);
        }
        let passing = s
            .points
            .iter()
            .filter(|p| self.strip_passes(s, p.value))
            .count();
        let tally = format!(
            "{} of {} pass",
            count(passing),
            count(self.coverage.securities)
        );
        page.blank();
        let words = wrap(&self.strip_words(s), WIDTH - 3 - width(&tally));
        for (k, line) in words.iter().enumerate() {
            if k == 0 {
                page.line(format!(
                    "  {line}{}",
                    pad_left(&tally, WIDTH - 2 - width(line))
                ));
            } else {
                page.line(format!("  {line}"));
            }
        }
        page.line(format!("  {bars}"));
        page.line(format!("  {axis}"));
        page.line(format!("  {}", labels.into_iter().collect::<String>()));
    }
}

impl fmt::Display for ScreenReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}
