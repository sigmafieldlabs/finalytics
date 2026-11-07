//! The screening report, on one screen of a universe: how many securities were measured, each
//! rule with how many met it, the matches in order, and the spread of each measure across the
//! universe, so that a reader sees where a threshold falls.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Date, ValidationReport};
use finalytics_risk::screening::{
    Coverage, FieldValue, Measure, MeasureUnit, Op, Order, Rule, RuleResult, RuleValue,
    ScreenResults, Sort, Status,
};

use crate::tables::{Cell, Column, ColumnKind, ReportTable};
use crate::text::{money_places, num, pct, round};

/// What a screen measured, beside its figures: what the report needs to describe it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenContext {
    /// The engine's version.
    pub version: String,
    /// A SHA-256 digest, in hexadecimal, of the options and of what the screen found for each
    /// security: the same inputs give the same report.
    pub digest: String,
    /// The date measured to.
    pub as_of: Date,
    /// The securities in the universe.
    pub securities: usize,
    /// The earliest close of any security.
    pub first_date: Option<Date>,
    /// The benchmark, if any.
    pub benchmark: Option<String>,
    /// The annual risk-free rate.
    pub risk_free_rate: f64,
    /// The most days a last close could be before the date measured to without being stale.
    pub max_carry_days: u32,
    /// The measures computed.
    pub measures: Vec<Measure>,
    /// The names of the fields given.
    pub fields: Vec<String>,
    /// How the matches are sorted.
    pub sort: Option<Sort>,
    /// The most matches kept.
    pub limit: Option<usize>,
    /// Problems that did not stop the screen.
    pub warnings: ValidationReport,
}

/// A column of the matches: a measure or a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenColumn {
    /// The measure's or the field's name, such as `"return_1y"` or `"sector"`.
    pub name: String,
    /// Its heading, such as "Return 1y".
    pub label: String,
    /// How a measure is shown; absent for a field.
    pub unit: Option<MeasureUnit>,
    /// Whether it is a field rather than a measure.
    pub field: bool,
}

/// One match, with its value in each of the report's columns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenMatch {
    /// Its place among the matches, from 1.
    pub rank: usize,
    /// The security.
    pub id: String,
    /// Its value in each column, in the order of the report's `columns`.
    pub values: Vec<Option<FieldValue>>,
}

/// The spread of one measure, or of a numeric field a rule tests, across the securities that
/// are not stale: the smallest and largest values and the quartiles, with the values the rules
/// compare it with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MeasureSpread {
    /// The measure's or the field's name.
    pub name: String,
    /// Its label, such as "Return 1y".
    pub label: String,
    /// Its group, such as "Returns", or "Fields".
    pub group: String,
    /// How a measure is shown; absent for a field.
    pub unit: Option<MeasureUnit>,
    /// How many securities have a value.
    pub count: usize,
    /// How many that are not stale have none.
    pub lacking: usize,
    /// The smallest value.
    pub min: Option<f64>,
    /// The lower quartile, interpolated linearly between values.
    pub lower_quartile: Option<f64>,
    /// The median.
    pub median: Option<f64>,
    /// The upper quartile.
    pub upper_quartile: Option<f64>,
    /// The largest value.
    pub max: Option<f64>,
    /// The values the rules compare it with.
    pub thresholds: Vec<f64>,
}

/// The screening report: what one screen of a universe found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenReport {
    /// What was measured and how.
    pub context: ScreenContext,
    /// How many securities were measured, passed and matched.
    pub coverage: Coverage,
    /// Each rule, with how many met it.
    pub rules: Vec<RuleResult>,
    /// The columns of the matches: the measure or field sorted by, those the rules test, and a
    /// few of the main measures.
    pub columns: Vec<ScreenColumn>,
    /// Every match, in order.
    pub matches: Vec<ScreenMatch>,
    /// The spread of each measure across the universe.
    pub spread: Vec<MeasureSpread>,
    /// The report's title, such as "Quality at a fair price": shown when it is printed or laid
    /// out as a page.
    #[serde(default)]
    pub name: Option<String>,
}

/// The measures shown beside those the rules and the sort use, when computed.
const HEADLINE: [Measure; 6] = [
    Measure::Return1y,
    Measure::Volatility,
    Measure::SharpeRatio,
    Measure::MaxDrawdown,
    Measure::Rsi14,
    Measure::Beta,
];

/// The most columns the matches show besides the security, unless the rules and the sort use
/// more.
const COLUMNS: usize = 7;

/// The screening report on `results`. The `finalytics` crate assembles it.
#[doc(hidden)]
pub fn screen_report(results: &ScreenResults, version: &str) -> ScreenReport {
    let measure_column = |m: Measure| ScreenColumn {
        name: m.name().to_owned(),
        label: m.label().to_owned(),
        unit: Some(m.unit()),
        field: false,
    };
    let field_column = |name: &str| ScreenColumn {
        name: name.to_owned(),
        label: name.to_owned(),
        unit: None,
        field: true,
    };
    let mut columns: Vec<ScreenColumn> = Vec::new();
    let push = |column: ScreenColumn, columns: &mut Vec<ScreenColumn>| {
        if !columns.iter().any(|c| c.name == column.name) {
            columns.push(column);
        }
    };
    let column_of = |name: &str, field: bool| match (field, Measure::from_name(name)) {
        (false, Some(m)) => measure_column(m),
        _ => field_column(name),
    };
    if let Some(sort) = &results.sort {
        push(column_of(sort.name(), sort.field.is_some()), &mut columns);
    }
    for r in &results.rules {
        push(
            column_of(r.rule.name(), r.rule.field.is_some()),
            &mut columns,
        );
    }
    for m in HEADLINE {
        if columns.len() >= COLUMNS.max(1) {
            break;
        }
        if results.measures.contains(&m) {
            push(measure_column(m), &mut columns);
        }
    }

    let position = |m: &str| results.measures.iter().position(|x| x.name() == m);
    let matches: Vec<ScreenMatch> = results
        .matched()
        .into_iter()
        .map(|s| ScreenMatch {
            rank: s.rank.unwrap_or_default(),
            id: s.id.clone(),
            values: columns
                .iter()
                .map(|c| {
                    if c.field {
                        s.fields.get(&c.name).cloned()
                    } else {
                        position(&c.name)
                            .and_then(|k| s.values[k])
                            .map(FieldValue::Number)
                    }
                })
                .collect(),
        })
        .collect();

    // The spread of each measure, and of each numeric field a rule tests, over the securities
    // that are not stale.
    let live: Vec<usize> = (0..results.securities.len())
        .filter(|i| results.securities[*i].status != Status::Stale)
        .collect();
    let thresholds = |name: &str| -> Vec<f64> {
        let mut found: Vec<f64> = Vec::new();
        for r in results.rules.iter().filter(|r| r.rule.name() == name) {
            match r.rule.value {
                Some(RuleValue::Number(x)) => found.push(x),
                Some(RuleValue::Range([a, b])) => found.extend([a, b]),
                None => {}
            }
        }
        found
    };
    let spread_of = |name: String,
                     label: String,
                     group: String,
                     unit: Option<MeasureUnit>,
                     values: Vec<Option<f64>>|
     -> MeasureSpread {
        let mut have: Vec<f64> = values.iter().flatten().copied().collect();
        have.sort_by(f64::total_cmp);
        let q = |p: f64| (!have.is_empty()).then(|| quantile(&have, p));
        MeasureSpread {
            thresholds: thresholds(&name),
            name,
            label,
            group,
            unit,
            count: have.len(),
            lacking: values.len() - have.len(),
            min: have.first().copied(),
            lower_quartile: q(0.25),
            median: q(0.5),
            upper_quartile: q(0.75),
            max: have.last().copied(),
        }
    };
    let mut spread: Vec<MeasureSpread> = results
        .measures
        .iter()
        .enumerate()
        .map(|(k, m)| {
            spread_of(
                m.name().to_owned(),
                m.label().to_owned(),
                m.group().to_owned(),
                Some(m.unit()),
                live.iter()
                    .map(|i| results.securities[*i].values[k])
                    .collect(),
            )
        })
        .collect();
    for r in &results.rules {
        let Some(name) = r.rule.field.as_deref() else {
            continue;
        };
        if r.rule.op.is_text() || spread.iter().any(|s| s.name == name) {
            continue;
        }
        let values = live
            .iter()
            .map(|i| match results.securities[*i].fields.get(name) {
                Some(FieldValue::Number(x)) => Some(*x),
                _ => None,
            })
            .collect();
        spread.push(spread_of(
            name.to_owned(),
            name.to_owned(),
            "Fields".to_owned(),
            None,
            values,
        ));
    }

    ScreenReport {
        context: ScreenContext {
            version: version.to_owned(),
            digest: digest(results),
            as_of: results.as_of,
            securities: results.securities.len(),
            first_date: results.securities.iter().filter_map(|s| s.first_date).min(),
            benchmark: results.benchmark.clone(),
            risk_free_rate: results.risk_free_rate,
            max_carry_days: results.max_carry_days,
            measures: results.measures.clone(),
            fields: results.fields.clone(),
            sort: results.sort.clone(),
            limit: results.limit,
            warnings: results.warnings.clone(),
        },
        coverage: results.coverage.clone(),
        rules: results.rules.clone(),
        columns,
        matches,
        spread,
        name: None,
    }
}

/// The quantile of sorted `values` at `p`, interpolated linearly between order statistics.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * p;
    let below = position.floor();
    let k = below as usize;
    match sorted.get(k + 1) {
        Some(next) => sorted[k] + (position - below) * (next - sorted[k]),
        None => sorted[k],
    }
}

/// A SHA-256 digest of what the screen found, in hexadecimal.
fn digest(r: &ScreenResults) -> String {
    let mut hash = Sha256::new();
    let settings = serde_json::json!({
        "as_of": r.as_of,
        "max_carry_days": r.max_carry_days,
        "benchmark": r.benchmark,
        "risk_free_rate": r.risk_free_rate,
        "measures": r.measures,
        "rules": r.rules.iter().map(|x| &x.rule).collect::<Vec<_>>(),
        "sort": r.sort,
        "limit": r.limit,
    });
    hash.update(settings.to_string().as_bytes());
    for s in &r.securities {
        hash.update(s.id.as_bytes());
        hash.update([s.status as u8]);
        for d in [s.first_date, s.last_date] {
            hash.update(d.map_or(i32::MIN, |d| d.days_since_epoch()).to_le_bytes());
        }
        for v in &s.values {
            hash.update(v.unwrap_or(f64::NAN).to_le_bytes());
        }
        for (name, value) in &s.fields {
            hash.update(name.as_bytes());
            hash.update(value.to_string().as_bytes());
        }
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ── Words and numbers ────────────────────────────────────────────────────────────────────────

/// A large count in three figures, such as "12.3M"; a smaller one in full, such as "4,512".
pub(crate) fn compact(x: f64) -> String {
    let size = x.abs();
    if size < 10_000.0 {
        return money_places(x, if x.fract() == 0.0 { 0 } else { 1 });
    }
    let (scale, suffix) = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "k")]
        .into_iter()
        .find(|(s, _)| size >= *s)
        .unwrap_or((1.0, ""));
    let v = x / scale;
    let places = if v.abs() >= 100.0 {
        0
    } else if v.abs() >= 10.0 {
        1
    } else {
        2
    };
    format!("{}{suffix}", num(Some(v), places))
}

/// A field's number: in three figures when large, otherwise with up to two decimals.
fn field_number(x: f64) -> String {
    if x.abs() >= 10_000.0 {
        compact(x)
    } else {
        trimmed(num(Some(x), 2))
    }
}

/// `text` without trailing zeros after a decimal point.
fn trimmed(text: String) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

/// A value as the report shows it.
pub(crate) fn shown(x: Option<f64>, unit: Option<MeasureUnit>) -> String {
    let Some(x) = x.filter(|x| x.is_finite()) else {
        return "–".to_owned();
    };
    match unit {
        Some(MeasureUnit::Fraction) => pct(Some(x), 1, false),
        Some(MeasureUnit::Ratio) => num(Some(x), 2),
        Some(MeasureUnit::Price) => money_places(x, 2),
        Some(MeasureUnit::Index) => num(Some(x), 1),
        Some(MeasureUnit::Count) => compact(x),
        Some(MeasureUnit::Flag) => (if x != 0.0 { "yes" } else { "no" }).to_owned(),
        _ => field_number(x),
    }
}

/// A cell of the matches as the report shows it.
pub(crate) fn cell(value: &Option<FieldValue>, unit: Option<MeasureUnit>) -> String {
    match value {
        None => "–".to_owned(),
        Some(FieldValue::Number(x)) => shown(Some(*x), unit),
        Some(FieldValue::Text(t)) => t.clone(),
    }
}

/// A rule's number as written in its words: "10%", "1.5", "30".
fn threshold(x: f64, unit: Option<MeasureUnit>) -> String {
    match unit {
        Some(MeasureUnit::Fraction) => {
            let v = round(x * 100.0, 4);
            format!("{}%", trimmed(num(Some(v), 4)))
        }
        Some(MeasureUnit::Count) if x.abs() >= 10_000.0 => compact(x),
        _ => trimmed(num(Some(x), 6)),
    }
}

/// The unit of the measure `name`, if it is one.
fn unit_of(rule: &Rule) -> Option<MeasureUnit> {
    rule.measure
        .as_deref()
        .and_then(Measure::from_name)
        .map(Measure::unit)
}

/// The label of what a rule or sort tests: a measure's label, or a field's name.
pub(crate) fn label_of(name: &str, field: bool) -> String {
    match (field, Measure::from_name(name)) {
        (false, Some(m)) => m.label().to_owned(),
        _ => name.to_owned(),
    }
}

/// Words joined as a list: "a, b or c".
fn either(words: &[String], last: &str) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., final_word] => format!("{} {last} {final_word}", rest.join(", ")),
    }
}

/// A rule in words, such as "Return 1y above 10%" or "sector is Energy or Utilities".
pub(crate) fn rule_words(rule: &Rule) -> String {
    let name = label_of(rule.name(), rule.field.is_some());
    let unit = unit_of(rule);
    let one = || match rule.value {
        Some(RuleValue::Number(x)) => threshold(x, unit),
        _ => "–".to_owned(),
    };
    match rule.op {
        Op::Below => format!("{name} below {}", one()),
        Op::AtMost => format!("{name} at most {}", one()),
        Op::Above => format!("{name} above {}", one()),
        Op::AtLeast => format!("{name} at least {}", one()),
        Op::Between => match rule.value {
            Some(RuleValue::Range([a, b])) => {
                format!(
                    "{name} from {} to {}",
                    threshold(a, unit),
                    threshold(b, unit)
                )
            }
            _ => format!("{name} in a range"),
        },
        Op::In => format!(
            "{name} is {}",
            either(rule.values.as_deref().unwrap_or_default(), "or")
        ),
        Op::NotIn => format!(
            "{name} is not {}",
            either(rule.values.as_deref().unwrap_or_default(), "or")
        ),
        _ => name,
    }
}

/// How the matches are sorted, in words, such as "by Sharpe ratio, highest first".
pub(crate) fn sort_words(sort: &Sort) -> String {
    let order = match sort.order {
        Order::Descending => "highest first",
        Order::Ascending => "lowest first",
    };
    format!(
        "by {}, {order}",
        lower(&label_of(sort.name(), sort.field.is_some()))
    )
}

/// A label inside a sentence: "Return 1y" as "return 1y", but "Sharpe ratio", "RSI 14" and a
/// field's name as they are.
pub(crate) fn lower(label: &str) -> String {
    const NAMES: [&str; 6] = ["Sharpe", "Sortino", "Calmar", "RSI", "MACD", "SMA"];
    if NAMES.iter().any(|n| label.starts_with(n)) || Measure::all().all(|m| m.label() != label) {
        label.to_owned()
    } else {
        crate::backtest_html::lower_first(label)
    }
}

/// Numbers with thousands separators.
pub(crate) fn count(n: usize) -> String {
    money_places(n as f64, 0)
}

impl ScreenReport {
    /// The title: the report's name, or what the screen kept.
    pub(crate) fn title(&self, name: Option<&str>) -> String {
        match name.or(self.name.as_deref()) {
            Some(n) => n.to_owned(),
            None => format!(
                "Screen of {} {}",
                count(self.context.securities),
                if self.context.securities == 1 {
                    "security"
                } else {
                    "securities"
                }
            ),
        }
    }

    /// What was screened, in parts joined by " · ".
    pub(crate) fn parts(&self) -> Vec<String> {
        let c = &self.context;
        let mut parts = Vec::new();
        if let Some(b) = &c.benchmark {
            parts.push(format!("against {b}"));
        }
        parts.push(format!(
            "risk-free {}",
            threshold(c.risk_free_rate, Some(MeasureUnit::Fraction))
        ));
        parts.push(match self.rules.len() {
            0 => "no rules".to_owned(),
            1 => "1 rule".to_owned(),
            n => format!("{n} rules"),
        });
        if let Some(s) = &c.sort {
            parts.push(format!("sorted {}", sort_words(s)));
        }
        if let Some(l) = c.limit {
            parts.push(format!("top {}", count(l)));
        }
        parts
    }

    /// What is too short and what is stale, in a sentence or two.
    pub(crate) fn coverage_note(&self) -> String {
        let c = &self.context;
        let needs = if self.rules.is_empty() && c.sort.is_none() {
            "fewer than two closes"
        } else {
            "fewer than two closes, or less history than a rule or the sort needs"
        };
        format!(
            "Too short: {needs}. Stale: no close in the {} days to {}; listed, but not measured.",
            c.max_carry_days,
            crate::text::date(c.as_of)
        )
    }

    /// The notes on how every figure was measured: a term and its text.
    pub(crate) fn method_notes(&self) -> Vec<(String, String)> {
        let c = &self.context;
        let as_of = crate::text::date(c.as_of);
        let has = |m: Measure| c.measures.contains(&m);
        let mut notes = vec![
            (
                "As of".to_owned(),
                format!(
                    "Each security is measured on its own closes up to {as_of}, with no \
                     alignment between them; one whose last close is more than {} days \
                     earlier is stale, and listed but not measured.",
                    c.max_carry_days
                ),
            ),
            (
                "Windows".to_owned(),
                format!(
                    "Returns and risk are over trailing calendar windows ending on {as_of}, each \
                     from the last close on or before its first day: a week, one, three and six \
                     months, the year to date and one, three and five years. A window longer \
                     than a security's history gives no value. Returns over three and five years \
                     are a year."
                ),
            ),
            (
                "Risk".to_owned(),
                format!(
                    "Volatility, downside deviation and the Sharpe and Sortino ratios are of \
                     daily returns, annualised with 252 days a year, against a risk-free rate of \
                     {} a year; drawdowns are falls from the highest close. Value at risk is the \
                     loss of the worst day in twenty over the last year, from history.",
                    pct(Some(c.risk_free_rate), 1, false)
                ),
            ),
        ];
        notes.push((
            "Benchmark".to_owned(),
            match &c.benchmark {
                Some(b) => format!(
                    "{b}: beta and correlation are of the daily returns between the dates both \
                     have a close, over the last year; a relative return is the return less \
                     {b}'s over the same window."
                ),
                None => "None given, so the measures against a benchmark are left out.".to_owned(),
            },
        ));
        notes.push((
            "Technicals".to_owned(),
            "Moving averages of the last 20, 50 and 200 closes; RSI of the last 14 changes with \
             Wilder's smoothing; MACD of exponential averages of 12, 26 and 9 closes, each \
             started at the first; the 52-week range of the last year's closes."
                .to_owned(),
        ));
        let mut income = Vec::new();
        if has(Measure::DividendYield) {
            income.push(
                "the dividend yield is the last year's dividends over the last close".to_owned(),
            );
        }
        if has(Measure::AverageVolume20d)
            || has(Measure::VolumeRatio)
            || has(Measure::DollarVolume20d)
        {
            income.push("volumes are over the last 20 days".to_owned());
        }
        if !income.is_empty() {
            let mut text = income.join("; ");
            if let Some(first) = text.get(..1) {
                text = first.to_uppercase() + &text[1..];
            }
            notes.push(("Income and volume".to_owned(), format!("{text}.")));
        }
        notes.push((
            "Rules".to_owned(),
            if self.rules.is_empty() {
                "None: every security that is not stale is kept.".to_owned()
            } else {
                "A match meets every rule; a security without the value a rule tests fails it. \
                 Each rule's count is of the securities that are not stale."
                    .to_owned()
            },
        ));
        notes
    }

    /// The report as labelled tables for export: the coverage, the rules, the matches and the
    /// spread of each measure.
    pub fn tables(&self) -> Vec<ReportTable> {
        let col = |key: &str, label: &str, kind: ColumnKind| Column {
            key: key.to_owned(),
            label: label.to_owned(),
            kind,
        };
        let table =
            |name: &str, title: &str, columns: Vec<Column>, rows: Vec<Vec<Cell>>| ReportTable {
                name: name.to_owned(),
                title: title.to_owned(),
                columns,
                rows,
            };
        let number = |x: Option<f64>| {
            x.filter(|x| x.is_finite())
                .map_or(Cell::Missing, Cell::Number)
        };
        let whole = |n: usize| Cell::Number(n as f64);
        let kind_of = |unit: Option<MeasureUnit>| match unit {
            Some(MeasureUnit::Fraction) => ColumnKind::Fraction,
            Some(MeasureUnit::Count) => ColumnKind::Number,
            Some(MeasureUnit::Flag) => ColumnKind::Integer,
            _ => ColumnKind::Number,
        };
        let c = &self.coverage;
        let mut out = vec![table(
            "coverage",
            "Coverage",
            vec![
                col("securities", "Securities", ColumnKind::Integer),
                col("measured", "Measured", ColumnKind::Integer),
                col("too_short", "Too short", ColumnKind::Integer),
                col("stale", "Stale", ColumnKind::Integer),
                col("passed", "Passed every rule", ColumnKind::Integer),
                col("matched", "Matches", ColumnKind::Integer),
            ],
            vec![vec![
                whole(c.securities),
                whole(c.measured),
                whole(c.too_short),
                whole(c.stale),
                whole(c.passed),
                whole(c.matched),
            ]],
        )];
        out.push(table(
            "rules",
            "Rules",
            vec![
                col("rule", "Rule", ColumnKind::Text),
                col("passed", "Passed", ColumnKind::Integer),
                col("lacking", "Lacking", ColumnKind::Integer),
                col("remaining", "With those before", ColumnKind::Integer),
            ],
            self.rules
                .iter()
                .map(|r| {
                    vec![
                        Cell::Text(rule_words(&r.rule)),
                        whole(r.passed),
                        whole(r.lacking),
                        whole(r.remaining),
                    ]
                })
                .collect(),
        ));
        let mut columns = vec![
            col("rank", "Rank", ColumnKind::Integer),
            col("id", "Security", ColumnKind::Text),
        ];
        for (j, k) in self.columns.iter().enumerate() {
            let kind = if k.field {
                let text = self
                    .matches
                    .iter()
                    .any(|m| matches!(m.values.get(j), Some(Some(FieldValue::Text(_)))));
                if text {
                    ColumnKind::Text
                } else {
                    ColumnKind::Number
                }
            } else {
                kind_of(k.unit)
            };
            columns.push(col(&k.name, &k.label, kind));
        }
        out.push(table(
            "matches",
            "Matches",
            columns,
            self.matches
                .iter()
                .map(|m| {
                    let mut row = vec![whole(m.rank), Cell::Text(m.id.clone())];
                    row.extend(m.values.iter().map(|v| match v {
                        Some(FieldValue::Number(x)) => number(Some(*x)),
                        Some(FieldValue::Text(t)) => Cell::Text(t.clone()),
                        None => Cell::Missing,
                    }));
                    row
                })
                .collect(),
        ));
        out.push(table(
            "spread",
            "Spread across the universe",
            vec![
                col("measure", "Measure", ColumnKind::Text),
                col("group", "Group", ColumnKind::Text),
                col("count", "With a value", ColumnKind::Integer),
                col("lacking", "Without", ColumnKind::Integer),
                col("min", "Lowest", ColumnKind::Number),
                col("lower_quartile", "Lower quartile", ColumnKind::Number),
                col("median", "Median", ColumnKind::Number),
                col("upper_quartile", "Upper quartile", ColumnKind::Number),
                col("max", "Highest", ColumnKind::Number),
            ],
            self.spread
                .iter()
                .map(|s| {
                    vec![
                        Cell::Text(s.name.clone()),
                        Cell::Text(s.group.clone()),
                        whole(s.count),
                        whole(s.lacking),
                        number(s.min),
                        number(s.lower_quartile),
                        number(s.median),
                        number(s.upper_quartile),
                        number(s.max),
                    ]
                })
                .collect(),
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_people_write_them() {
        assert_eq!(compact(4_512.0), "4,512");
        assert_eq!(compact(12_345_678.0), "12.3M");
        assert_eq!(compact(987_654.0), "988k");
        assert_eq!(compact(2.5e9), "2.50B");
        assert_eq!(threshold(0.1, Some(MeasureUnit::Fraction)), "10%");
        assert_eq!(threshold(0.125, Some(MeasureUnit::Fraction)), "12.5%");
        assert_eq!(threshold(-0.2, Some(MeasureUnit::Fraction)), "−20%");
        assert_eq!(threshold(1.5, Some(MeasureUnit::Ratio)), "1.5");
        assert_eq!(threshold(30.0, Some(MeasureUnit::Index)), "30");
        assert_eq!(shown(Some(-0.0523), Some(MeasureUnit::Fraction)), "−5.2%");
        assert_eq!(shown(Some(1.0), Some(MeasureUnit::Flag)), "yes");
        assert_eq!(shown(None, Some(MeasureUnit::Ratio)), "–");
        assert_eq!(shown(Some(18.4), None), "18.4");
        let rule = Rule::between("rsi_14", 30.0, 70.0);
        assert_eq!(rule_words(&rule), "RSI 14 from 30 to 70");
        let rule = Rule::one_of("sector", ["Energy", "Utilities", "Materials"]);
        assert_eq!(
            rule_words(&rule),
            "sector is Energy, Utilities or Materials"
        );
        assert_eq!(
            rule_words(&Rule::above("return_1y", 0.0)),
            "Return 1y above 0%"
        );
        assert_eq!(
            sort_words(&Sort::by("sharpe_ratio", Order::Descending)),
            "by Sharpe ratio, highest first"
        );
    }
}
