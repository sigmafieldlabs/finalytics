//! The screening report, on one screen of a universe: how the rules narrow the universe to the
//! matches, the matches in order, the securities that fail just one rule, where each rule's
//! threshold falls among the securities' values, the spread of every measure and what is out of
//! the ordinary in the data.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Date, ValidationReport};
use finalytics_risk::screening::{
    Coverage, FieldValue, FormulaResult, Measure, MeasureUnit, Op, Order, Rule, RuleResult,
    RuleValue, ScreenResults, SecurityScreen, Sort, Status,
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
    /// The names of the fields given, then of the formulas.
    pub fields: Vec<String>,
    /// The formulas, in order, with how many securities have no value of each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formulas: Vec<FormulaResult>,
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
    /// Its name in words, such as "1-year return" or "Sector".
    pub label: String,
    /// How a measure is shown; absent for a field.
    pub unit: Option<MeasureUnit>,
    /// Whether it is a field rather than a measure.
    pub field: bool,
}

/// One security of the matches, with its value in each of the report's columns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenMatch {
    /// Its place in the sort, from 1.
    pub rank: usize,
    /// The security.
    pub id: String,
    /// Its value in each column, in the order of the report's `columns`.
    pub values: Vec<Option<FieldValue>>,
    /// The places in `columns` of its values close to a threshold of a rule: within 10% of it
    /// or, for a threshold of zero, within 10% of the measure's typical size, the median of its
    /// size across the securities that are not stale.
    #[serde(default)]
    pub near_threshold: Vec<usize>,
}

/// A security that fails just one rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct NearMiss {
    /// The security.
    pub id: String,
    /// The place in the report's `rules` of the rule it fails.
    pub rule: usize,
    /// Its value in each column, in the order of the report's `columns`.
    pub values: Vec<Option<FieldValue>>,
}

/// One security's value on a threshold strip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct StripPoint {
    /// The security.
    pub id: String,
    /// Its value.
    pub value: f64,
    /// Whether it is one of the matches.
    pub matched: bool,
}

/// Where the securities' values of one measure, or of a numeric field, fall against the
/// thresholds of the rules that test it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ThresholdStrip {
    /// The measure's or the field's name.
    pub name: String,
    /// Its name in words.
    pub label: String,
    /// How a measure is shown; absent for a field.
    pub unit: Option<MeasureUnit>,
    /// The places in the report's `rules` of the rules that test it.
    pub rules: Vec<usize>,
    /// The value of every security that is not stale and has one, lowest first.
    pub points: Vec<StripPoint>,
}

/// What is out of the ordinary in the securities' prices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenExceptions {
    /// The securities measured on a last close before the date measured to, within
    /// `max_carry_days` of it.
    pub late: Vec<String>,
    /// The stale securities: listed, but not measured.
    pub stale: Vec<String>,
    /// The securities with too short a history for a rule or the sort.
    pub too_short: Vec<String>,
    /// The date of every security's first close, when they all share one.
    pub common_start: Option<Date>,
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
    /// Its name in words, such as "1-year return".
    pub label: String,
    /// Its group, such as "Returns", or "Fields".
    pub group: String,
    /// How a measure is shown; absent for a field.
    pub unit: Option<MeasureUnit>,
    /// How many securities have a value.
    pub count: usize,
    /// How many that are not stale have none.
    pub lacking: usize,
    /// How many of those without a value have too short a history for it; none for a field.
    #[serde(default)]
    pub lacking_history: usize,
    /// For a measure that is yes or no, how many securities have yes.
    #[serde(default)]
    pub yes: Option<usize>,
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
    /// The columns of the matches: the security's name and sector when given, the text fields
    /// the rules test, the measure or field sorted by and those the rules compare with numbers;
    /// without rules or a sort, a few of the main measures.
    pub columns: Vec<ScreenColumn>,
    /// Every match, in order.
    pub matches: Vec<ScreenMatch>,
    /// The first securities that pass every rule but are beyond the limit, in order: up to ten.
    #[serde(default)]
    pub also_passed: Vec<ScreenMatch>,
    /// Every security that fails just one rule, the closest to passing first: by how far its
    /// value is from the rule's threshold, as a share of the threshold (or, for a threshold of
    /// zero, of the measure's typical size). Those that fail a rule on text, or have no value
    /// for it, come last.
    #[serde(default)]
    pub near_misses: Vec<NearMiss>,
    /// For each measure or numeric field the rules compare with a number, every security's
    /// value against the rules' thresholds.
    #[serde(default)]
    pub strips: Vec<ThresholdStrip>,
    /// The spread of each measure across the universe.
    pub spread: Vec<MeasureSpread>,
    /// What is out of the ordinary in the securities' prices.
    #[serde(default)]
    pub exceptions: ScreenExceptions,
    /// The report's title, such as "Quality at a fair price": shown when it is printed or laid
    /// out as a page.
    #[serde(default)]
    pub name: Option<String>,
}

/// The measures the matches show when no rule compares with a number and nothing is sorted.
const HEADLINE: [Measure; 4] = [
    Measure::Return1y,
    Measure::Volatility,
    Measure::SharpeRatio,
    Measure::MaxDrawdown,
];

/// The fields the matches show, when given, beside the security.
const NAMED: [&str; 2] = ["name", "sector"];

/// The most securities beyond the limit that the report keeps.
const ALSO_PASSED: usize = 10;

/// How close to a threshold a value is marked, as a share of the threshold.
pub(crate) const NEAR: f64 = 0.10;

/// Whether a security meets a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    Lacking,
}

/// What `rule` finds of `value`, the value of what it tests.
fn verdict(rule: &Rule, value: Option<&FieldValue>) -> Verdict {
    let judged = |ok: bool| if ok { Verdict::Pass } else { Verdict::Fail };
    match (rule.op, value) {
        (Op::In | Op::NotIn, Some(FieldValue::Text(t))) => {
            let inside = rule.values.iter().flatten().any(|v| v == t);
            judged(inside == (rule.op == Op::In))
        }
        (_, Some(FieldValue::Number(x))) if !rule.op.is_text() => judged(passes(rule, *x)),
        _ => Verdict::Lacking,
    }
}

/// Whether the number `x` meets the numeric rule `rule`.
pub(crate) fn passes(rule: &Rule, x: f64) -> bool {
    match (rule.op, rule.value) {
        (Op::Below, Some(RuleValue::Number(v))) => x < v,
        (Op::AtMost, Some(RuleValue::Number(v))) => x <= v,
        (Op::Above, Some(RuleValue::Number(v))) => x > v,
        (Op::AtLeast, Some(RuleValue::Number(v))) => x >= v,
        (Op::Between, Some(RuleValue::Range([low, high]))) => x >= low && x <= high,
        _ => false,
    }
}

/// The numbers a rule compares with.
pub(crate) fn thresholds(rule: &Rule) -> Vec<f64> {
    match rule.value {
        Some(RuleValue::Number(x)) => vec![x],
        Some(RuleValue::Range([a, b])) => vec![a, b],
        None => Vec::new(),
    }
}

/// Whether the measure `name` compares the price with its moving average, and so is shown as
/// the difference: 1.08 as +8%.
pub(crate) fn vs_average(name: &str) -> bool {
    matches!(
        name,
        "price_to_sma_20" | "price_to_sma_50" | "price_to_sma_200"
    )
}

/// A value of `name` on the scale the report shows it: a price against its average as the
/// difference.
pub(crate) fn on_scale(name: &str, x: f64) -> f64 {
    if vs_average(name) { x - 1.0 } else { x }
}

/// How far `x` is from the threshold `t` of a rule on `name`, as a share of the threshold or,
/// for a threshold of zero, of `typical`.
pub(crate) fn gap(name: &str, x: f64, t: f64, typical: f64) -> f64 {
    let t = on_scale(name, t);
    let scale = if t.abs() > 1e-12 { t.abs() } else { typical };
    if scale > 0.0 {
        (on_scale(name, x) - t).abs() / scale
    } else {
        f64::INFINITY
    }
}

/// The typical size of values on `name`'s scale: the median of their sizes, the upper of the
/// two middle ones when there is an even number.
pub(crate) fn typical(name: &str, values: impl Iterator<Item = f64>) -> f64 {
    let mut sizes: Vec<f64> = values.map(|x| on_scale(name, x).abs()).collect();
    sizes.sort_by(f64::total_cmp);
    sizes.get(sizes.len() / 2).copied().unwrap_or(0.0)
}

/// The places from 0 to `n` in the order `cmp` gives, equal ones in place. Every sort of the
/// report goes through it, so that one sort is compiled for them all.
fn ordered(n: usize, cmp: &dyn Fn(usize, usize) -> Ordering) -> Vec<usize> {
    let mut at: Vec<usize> = (0..n).collect();
    at.sort_by(|a, b| cmp(*a, *b));
    at
}

/// The order of the sort on the values `a` and `b`, those without a value last.
fn sorted(order: Order, a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => match order {
            Order::Ascending => x.total_cmp(&y),
            Order::Descending => y.total_cmp(&x),
        },
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// A field's name in words, starting with a capital: `pe_ratio` as "Pe ratio".
pub(crate) fn field_label(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

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
        label: field_label(name),
        unit: None,
        field: true,
    };
    let column_of = |name: &str, field: bool| match (field, Measure::from_name(name)) {
        (false, Some(m)) => measure_column(m),
        _ => field_column(name),
    };
    let mut columns: Vec<ScreenColumn> = Vec::new();
    let push = |column: ScreenColumn, columns: &mut Vec<ScreenColumn>| {
        if !columns
            .iter()
            .any(|c| c.name == column.name && c.field == column.field)
        {
            columns.push(column);
        }
    };
    for name in NAMED {
        if results.fields.iter().any(|f| f == name) {
            push(field_column(name), &mut columns);
        }
    }
    for r in results.rules.iter().filter(|r| r.rule.op.is_text()) {
        push(field_column(r.rule.name()), &mut columns);
    }
    if let Some(sort) = &results.sort {
        push(column_of(sort.name(), sort.field.is_some()), &mut columns);
    }
    for r in results.rules.iter().filter(|r| !r.rule.op.is_text()) {
        push(
            column_of(r.rule.name(), r.rule.field.is_some()),
            &mut columns,
        );
    }
    let numeric = results.sort.is_some() || results.rules.iter().any(|r| !r.rule.op.is_text());
    if !numeric {
        for m in HEADLINE {
            if results.measures.contains(&m) {
                push(measure_column(m), &mut columns);
            }
        }
    }

    let position = |m: &str| results.measures.iter().position(|x| x.name() == m);
    let value_of = |s: &SecurityScreen, name: &str, field: bool| -> Option<FieldValue> {
        match (field, position(name)) {
            (false, Some(k)) => s.values[k].map(FieldValue::Number),
            _ => s.fields.get(name).cloned(),
        }
    };
    let rule_value = |s: &SecurityScreen, rule: &Rule| -> Option<FieldValue> {
        value_of(s, rule.name(), rule.field.is_some())
    };
    let row = |s: &SecurityScreen| -> Vec<Option<FieldValue>> {
        columns
            .iter()
            .map(|c| value_of(s, &c.name, c.field))
            .collect()
    };
    let live: Vec<&SecurityScreen> = results
        .securities
        .iter()
        .filter(|s| s.status != Status::Stale)
        .collect();

    // A strip for each measure or numeric field the rules compare with a number, in the order
    // of the rules.
    let mut strips: Vec<ThresholdStrip> = Vec::new();
    for (k, r) in results.rules.iter().enumerate() {
        if r.rule.op.is_text() {
            continue;
        }
        let (name, field) = (r.rule.name(), r.rule.field.is_some());
        if let Some(strip) = strips.iter_mut().find(|s| s.name == name) {
            strip.rules.push(k);
            continue;
        }
        let found: Vec<(&SecurityScreen, f64)> = live
            .iter()
            .filter_map(|s| match value_of(s, name, field) {
                Some(FieldValue::Number(x)) if x.is_finite() => Some((*s, x)),
                _ => None,
            })
            .collect();
        let points = ordered(found.len(), &|a, b| found[a].1.total_cmp(&found[b].1))
            .into_iter()
            .map(|k| StripPoint {
                id: found[k].0.id.clone(),
                value: found[k].1,
                matched: found[k].0.matched,
            })
            .collect();
        let column = column_of(name, field);
        strips.push(ThresholdStrip {
            name: name.to_owned(),
            label: column.label,
            unit: column.unit,
            rules: vec![k],
            points,
        });
    }
    let typical_of = |name: &str| -> f64 {
        strips
            .iter()
            .find(|s| s.name == name)
            .map_or(0.0, |s| typical(name, s.points.iter().map(|p| p.value)))
    };
    let typicals: Vec<f64> = results
        .rules
        .iter()
        .map(|r| typical_of(r.rule.name()))
        .collect();
    // How far a value is from passing a numeric rule: from its nearest threshold.
    let distance = |k: usize, x: f64| -> f64 {
        let rule = &results.rules[k].rule;
        thresholds(rule)
            .into_iter()
            .map(|t| gap(rule.name(), x, t, typicals[k]))
            .fold(f64::INFINITY, f64::min)
    };
    let near_threshold = |values: &[Option<FieldValue>]| -> Vec<usize> {
        columns
            .iter()
            .zip(values)
            .enumerate()
            .filter(|(_, (c, v))| {
                let Some(FieldValue::Number(x)) = v else {
                    return false;
                };
                results.rules.iter().enumerate().any(|(k, r)| {
                    !r.rule.op.is_text()
                        && r.rule.name() == c.name
                        && r.rule.field.is_some() == c.field
                        && distance(k, *x) <= NEAR
                })
            })
            .map(|(j, _)| j)
            .collect()
    };
    let shown_row = |s: &SecurityScreen, rank: usize| -> ScreenMatch {
        let values = row(s);
        ScreenMatch {
            rank,
            id: s.id.clone(),
            near_threshold: near_threshold(&values),
            values,
        }
    };
    let matches: Vec<ScreenMatch> = results
        .matched()
        .into_iter()
        .map(|s| shown_row(s, s.rank.unwrap_or_default()))
        .collect();

    // The securities beyond the limit, in the sort's order.
    let beyond: Vec<&SecurityScreen> = results
        .securities
        .iter()
        .filter(|s| s.passed && !s.matched)
        .collect();
    let at = match &results.sort {
        Some(sort) => {
            let key = |s: &SecurityScreen| match value_of(s, sort.name(), sort.field.is_some()) {
                Some(FieldValue::Number(x)) => Some(x),
                _ => None,
            };
            ordered(beyond.len(), &|a, b| {
                sorted(sort.order, key(beyond[a]), key(beyond[b]))
            })
        }
        None => (0..beyond.len()).collect(),
    };
    let also_passed = at
        .into_iter()
        .map(|k| beyond[k])
        .take(ALSO_PASSED)
        .enumerate()
        .map(|(k, s)| shown_row(s, matches.len() + k + 1))
        .collect();

    // The securities that fail just one rule, the closest to passing first.
    let near: Vec<(f64, NearMiss)> = live
        .iter()
        .filter_map(|s| {
            let mut failed = results.rules.iter().enumerate().filter(|(_, r)| {
                verdict(&r.rule, rule_value(s, &r.rule).as_ref()) != Verdict::Pass
            });
            let (k, r) = failed.next()?;
            if failed.next().is_some() {
                return None;
            }
            let far = match rule_value(s, &r.rule) {
                Some(FieldValue::Number(x)) if !r.rule.op.is_text() => distance(k, x),
                _ => f64::INFINITY,
            };
            Some((
                far,
                NearMiss {
                    id: s.id.clone(),
                    rule: k,
                    values: row(s),
                },
            ))
        })
        .collect();
    let near_misses = ordered(near.len(), &|a, b| {
        near[a]
            .0
            .total_cmp(&near[b].0)
            .then_with(|| near[a].1.id.cmp(&near[b].1.id))
    })
    .into_iter()
    .map(|k| near[k].1.clone())
    .collect();

    // The spread of each measure, and of each numeric field a rule tests, over the securities
    // that are not stale.
    let spread_thresholds = |name: &str| -> Vec<f64> {
        results
            .rules
            .iter()
            .filter(|r| r.rule.name() == name)
            .flat_map(|r| thresholds(&r.rule))
            .collect()
    };
    let spread_of = |name: String,
                     label: String,
                     group: String,
                     unit: Option<MeasureUnit>,
                     values: Vec<Option<f64>>,
                     lacking_history: usize|
     -> MeasureSpread {
        let mut have: Vec<f64> = values.iter().flatten().copied().collect();
        have.sort_by(f64::total_cmp);
        let q = |p: f64| (!have.is_empty()).then(|| quantile(&have, p));
        MeasureSpread {
            thresholds: spread_thresholds(&name),
            name,
            label,
            group,
            unit,
            count: have.len(),
            lacking: values.len() - have.len(),
            lacking_history,
            yes: (unit == Some(MeasureUnit::Flag))
                .then(|| have.iter().filter(|x| **x != 0.0).count()),
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
                live.iter().map(|s| s.values[k]).collect(),
                results
                    .coverage
                    .lacking_history
                    .get(k)
                    .copied()
                    .unwrap_or(0),
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
            .map(|s| match s.fields.get(name) {
                Some(FieldValue::Number(x)) => Some(*x),
                _ => None,
            })
            .collect();
        spread.push(spread_of(
            name.to_owned(),
            field_label(name),
            "Fields".to_owned(),
            None,
            values,
            0,
        ));
    }

    // What is out of the ordinary in the prices.
    let ids = |keep: &dyn Fn(&SecurityScreen) -> bool| -> Vec<String> {
        results
            .securities
            .iter()
            .filter(|s| keep(s))
            .map(|s| s.id.clone())
            .collect()
    };
    let starts: Vec<Option<Date>> = results.securities.iter().map(|s| s.first_date).collect();
    let exceptions = ScreenExceptions {
        late: ids(&|s| s.status != Status::Stale && s.last_date.is_some_and(|d| d < results.as_of)),
        stale: ids(&|s| s.status == Status::Stale),
        too_short: ids(&|s| s.status == Status::TooShort),
        common_start: match starts.first() {
            Some(Some(d)) if starts.iter().all(|s| *s == Some(*d)) => Some(*d),
            _ => None,
        },
    };

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
            formulas: results.formulas.clone(),
            sort: results.sort.clone(),
            limit: results.limit,
            warnings: results.warnings.clone(),
        },
        coverage: results.coverage.clone(),
        rules: results.rules.clone(),
        columns,
        matches,
        also_passed,
        near_misses,
        strips,
        spread,
        exceptions,
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

/// The name in words of what a rule or sort tests: a measure's label, or a field's name.
pub(crate) fn label_of(name: &str, field: bool) -> String {
    match (field, Measure::from_name(name)) {
        (false, Some(m)) => m.label().to_owned(),
        _ => field_label(name),
    }
}

/// Words joined as a list: "a, b or c"; with a semicolon between them when one has a comma.
pub(crate) fn either(words: &[String], last: &str) -> String {
    if words.iter().any(|w| w.contains(',')) {
        return words.join("; ");
    }
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., final_word] => format!("{} {last} {final_word}", rest.join(", ")),
    }
}

/// Whether the measure `name` is a fall, negative or zero, for which a value above a threshold
/// is better than it.
fn is_fall(name: &str) -> bool {
    matches!(
        name,
        "max_drawdown" | "max_drawdown_3y" | "drawdown" | "from_52w_high"
    )
}

/// A value of `name` as the report shows it: a price against its average as the difference,
/// such as "+7.8%".
pub(crate) fn value_text(name: &str, unit: Option<MeasureUnit>, x: Option<f64>) -> String {
    match x {
        Some(x) if x.is_finite() && vs_average(name) => pct(Some(x - 1.0), 1, true),
        _ => shown(x, unit),
    }
}

/// A cell of the matches as the report shows it.
pub(crate) fn column_text(column: &ScreenColumn, value: &Option<FieldValue>) -> String {
    match value {
        None => "–".to_owned(),
        Some(FieldValue::Text(t)) => t.clone(),
        Some(FieldValue::Number(x)) if column.field => shown(Some(*x), None),
        Some(FieldValue::Number(x)) => value_text(&column.name, column.unit, Some(*x)),
    }
}

/// A threshold of a rule on `name` in words: "30%", "1.5", "30"; for a price against its
/// average, "its average" or the difference, such as "+5%".
pub(crate) fn limit_text(name: &str, unit: Option<MeasureUnit>, t: f64) -> String {
    if !vs_average(name) {
        return threshold(t, unit);
    }
    let d = round((t - 1.0) * 100.0, 4);
    if d == 0.0 {
        "its average".to_owned()
    } else {
        format!(
            "{}{}%",
            if d > 0.0 { "+" } else { "" },
            trimmed(num(Some(d), 4))
        )
    }
}

/// How a rule compares, before its threshold `t`: "above 30%" or, for a fall, "better than
/// −30%".
fn comparison(op: Op, t: &str, fall: bool) -> String {
    match (op, fall) {
        (Op::Above, false) => format!("above {t}"),
        (Op::AtLeast, false) => format!("at least {t}"),
        (Op::Below, false) => format!("below {t}"),
        (Op::AtMost, false) => format!("at most {t}"),
        (Op::Above, true) => format!("better than {t}"),
        (Op::AtLeast, true) => format!("{t} or better"),
        (Op::Below, true) => format!("worse than {t}"),
        (Op::AtMost, true) => format!("{t} or worse"),
        _ => t.to_owned(),
    }
}

/// For a rule that compares a price with its average itself, how: "above", "at or below".
fn against_average(rule: &Rule) -> Option<&'static str> {
    match rule.value {
        Some(RuleValue::Number(x)) if rule.field.is_none() && vs_average(rule.name()) => {
            if round((x - 1.0) * 100.0, 4) != 0.0 {
                return None;
            }
            match rule.op {
                Op::Above => Some("above"),
                Op::AtLeast => Some("at or above"),
                Op::Below => Some("below"),
                Op::AtMost => Some("at or below"),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A rule in words, such as "1-year return above 0%", "Price above its 200-day average" or
/// "Sector is not Utilities".
pub(crate) fn rule_words(rule: &Rule) -> String {
    let name = rule.name();
    let label = label_of(name, rule.field.is_some());
    let unit = unit_of(rule);
    if rule.op.is_text() {
        let values = either(rule.values.as_deref().unwrap_or_default(), "or");
        let not = if rule.op == Op::NotIn { "not " } else { "" };
        return format!("{label} is {not}{values}");
    }
    if let Some(how) = against_average(rule) {
        let average = label.trim_start_matches("Price vs ");
        return format!("Price {how} its {average}");
    }
    match rule.value {
        Some(RuleValue::Range([a, b])) => format!(
            "{label} between {} and {}",
            limit_text(name, unit, a),
            limit_text(name, unit, b)
        ),
        Some(RuleValue::Number(x)) => format!(
            "{label} {}",
            comparison(rule.op, &limit_text(name, unit, x), is_fall(name))
        ),
        None => label,
    }
}

/// What a numeric rule asks for, after a value that fails it: "needs 30 to 70", "needs above
/// its average".
pub(crate) fn needs_words(rule: &Rule) -> String {
    let name = rule.name();
    let unit = unit_of(rule);
    if let Some(how) = against_average(rule) {
        return format!("needs {how} its average");
    }
    match rule.value {
        Some(RuleValue::Range([a, b])) => format!(
            "needs {} to {}",
            limit_text(name, unit, a),
            limit_text(name, unit, b)
        ),
        Some(RuleValue::Number(x)) => format!(
            "needs {}",
            comparison(rule.op, &limit_text(name, unit, x), is_fall(name))
        ),
        None => String::new(),
    }
}

/// "a" or "an" before `word`.
fn article(word: &str) -> &'static str {
    match word.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U') => "an",
        _ => "a",
    }
}

/// `n` and the noun for it: "1 security", "56 securities".
pub(crate) fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", count(n), if n == 1 { one } else { many })
}

/// A measure's label inside a sentence: "Beta" as "beta", but "Sortino ratio" and "RSI (14
/// days)" as they are, and anything else that is not a measure's label as it is.
pub(crate) fn lower(label: &str) -> String {
    const NAMES: [&str; 5] = ["Sharpe", "Sortino", "Calmar", "RSI", "MACD"];
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

/// A step of the funnel: what it keeps, how many securities are left after it and, for a rule,
/// how many pass it on its own and how many have no value for it.
pub(crate) struct Step {
    pub(crate) words: String,
    pub(crate) left: usize,
    pub(crate) alone: Option<usize>,
    pub(crate) lacking: Option<String>,
}

/// The scale of a threshold strip, in the units the report shows the values in.
pub(crate) struct Scale {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
    /// The thresholds of the strip's rules.
    pub(crate) limits: Vec<f64>,
    /// From where to where the values pass every rule of the strip; none when no value can.
    pub(crate) zone: Option<(f64, f64)>,
}

impl Scale {
    /// Where `x` falls between the ends of the scale, from 0 to 1, held to the ends.
    pub(crate) fn at(&self, x: f64) -> f64 {
        (x.clamp(self.lo, self.hi) - self.lo) / (self.hi - self.lo)
    }
}

/// The note under the matches on the mark beside a value close to a threshold.
pub(crate) const CLOSE_NOTE: &str = "close to a rule's limit: within 10% of it (for a limit of \
                                     0, within 10% of a typical value).";

/// The measures the table of every measure leaves out: they describe the data rather than the
/// securities.
const DATA_MEASURES: [&str; 3] = ["observations", "last_price", "stale"];

/// The most identifiers a sentence names.
const NAMED_IDS: usize = 8;

/// Identifiers in a sentence: the first eight, and how many more.
fn id_list(ids: &[String]) -> String {
    let shown = ids
        .iter()
        .take(NAMED_IDS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if ids.len() > NAMED_IDS {
        format!("{shown} and {} more", ids.len() - NAMED_IDS)
    } else {
        shown
    }
}

/// A sentence from words: its first letter a capital, with a full stop.
fn sentence(words: &str) -> String {
    let words = words.trim().trim_end_matches('.');
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => format!("{}{}.", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

impl ScreenReport {
    /// What the screen kept, as a headline: "10 matches among 56 securities".
    pub(crate) fn headline(&self) -> String {
        let m = self.coverage.matched;
        let matches = if m == 0 {
            "No matches".to_owned()
        } else {
            counted(m, "match", "matches")
        };
        format!(
            "{matches} among {}",
            counted(self.context.securities, "security", "securities")
        )
    }

    /// The title: the report's name, or its headline.
    pub(crate) fn title(&self, name: Option<&str>) -> String {
        name.or(self.name.as_deref())
            .map_or_else(|| self.headline(), str::to_owned)
    }

    /// The securities that are not stale.
    pub(crate) fn live(&self) -> usize {
        self.coverage.securities - self.coverage.stale
    }

    /// What the matches are sorted by, inside a sentence: "3-year Sharpe ratio".
    pub(crate) fn sort_label(&self) -> Option<String> {
        self.context
            .sort
            .as_ref()
            .map(|s| match s.field.as_deref() {
                Some(f) => f.replace('_', " "),
                None => lower(&label_of(s.name(), false)),
            })
    }

    /// The place in `columns` of the measure or field `name`.
    pub(crate) fn column_of(&self, name: &str, field: bool) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name == name && c.field == field)
    }

    /// The place in `columns` of what the matches are sorted by.
    pub(crate) fn sort_column(&self) -> Option<usize> {
        let s = self.context.sort.as_ref()?;
        self.column_of(s.name(), s.field.is_some())
    }

    /// Two sentences on what the screen found: "13 of 56 securities pass all 6 rules. The 10
    /// with the highest 3-year Sharpe ratio are shown."
    pub(crate) fn summary(&self) -> String {
        let c = &self.coverage;
        let securities = counted(c.securities, "security", "securities");
        let mut text = match self.rules.len() {
            0 if c.stale == 0 => format!("With no rules, all {securities} pass."),
            0 => format!(
                "With no rules, the {} with recent prices pass.",
                counted(c.passed, "security", "securities")
            ),
            n => {
                let rules = match n {
                    1 => "the rule".to_owned(),
                    2 => "both rules".to_owned(),
                    _ => format!("all {n} rules"),
                };
                match c.passed {
                    0 => format!("None of the {securities} passes {rules}."),
                    1 => format!("1 of {securities} passes {rules}."),
                    p => format!("{} of {securities} pass {rules}.", count(p)),
                }
            }
        };
        if c.matched < c.passed {
            let m = c.matched;
            let kept = match (&self.context.sort, self.sort_label()) {
                (Some(s), Some(label)) => {
                    let how = match s.order {
                        Order::Descending => "highest",
                        Order::Ascending => "lowest",
                    };
                    if m == 1 {
                        format!("The one with the {how} {label} is shown.")
                    } else {
                        format!("The {} with the {how} {label} are shown.", count(m))
                    }
                }
                _ if m == 1 => "The first is shown.".to_owned(),
                _ => format!("The first {} are shown.", count(m)),
            };
            text.push(' ');
            text.push_str(&kept);
        }
        text
    }

    /// The tiles under the summary: a label, a value and a line under it.
    pub(crate) fn tiles(&self) -> Vec<(String, String, Option<String>)> {
        let c = &self.coverage;
        let x = &self.context;
        vec![
            (
                "Securities".to_owned(),
                count(c.securities),
                x.first_date
                    .map(|d| format!("prices from {}", crate::text::date(d))),
            ),
            (
                "Matches".to_owned(),
                count(c.matched),
                Some(if self.rules.is_empty() {
                    format!("of {} measured", count(c.passed))
                } else {
                    format!("of {} that pass every rule", count(c.passed))
                }),
            ),
            (
                "Rules".to_owned(),
                count(self.rules.len()),
                self.sort_label().map(|s| format!("sorted by {s}")),
            ),
            (
                "Prices to".to_owned(),
                crate::text::date(x.as_of),
                Some("the date measured to".to_owned()),
            ),
        ]
    }

    /// The steps of the funnel: every security, those with recent prices when some are stale,
    /// those left after each rule and, when the limit leaves some out, the matches kept.
    pub(crate) fn steps(&self) -> Vec<Step> {
        let c = &self.coverage;
        let step = |words: String, left: usize| Step {
            words,
            left,
            alone: None,
            lacking: None,
        };
        let mut steps = vec![step("All securities".to_owned(), c.securities)];
        if c.stale > 0 {
            steps.push(step(
                format!(
                    "With a price in the {} days to {}",
                    self.context.max_carry_days,
                    crate::text::date(self.context.as_of)
                ),
                self.live(),
            ));
        }
        for r in &self.rules {
            let lacking = (r.lacking > 0).then(|| match r.rule.field.as_deref() {
                Some(f) => format!(
                    "{} {} no {}",
                    count(r.lacking),
                    if r.lacking == 1 { "has" } else { "have" },
                    f.replace('_', " ")
                ),
                None => format!("{} without a value", count(r.lacking)),
            });
            steps.push(Step {
                words: rule_words(&r.rule),
                left: r.remaining,
                alone: Some(r.passed),
                lacking,
            });
        }
        if c.matched < c.passed {
            let words = match (&self.context.sort, self.sort_label()) {
                (Some(s), Some(label)) if s.order == Order::Ascending => {
                    format!("Top {} by {label}, lowest first", count(c.matched))
                }
                (Some(_), Some(label)) => format!("Top {} by {label}", count(c.matched)),
                _ => format!("The first {}", count(c.matched)),
            };
            steps.push(step(words, c.matched));
        }
        steps
    }

    /// What a near miss fails, in words: "RSI (14 days) 70.1, needs 30 to 70".
    pub(crate) fn miss_words(&self, m: &NearMiss) -> String {
        let Some(rule) = self.rules.get(m.rule).map(|r| &r.rule) else {
            return String::new();
        };
        let (name, field) = (rule.name(), rule.field.is_some());
        let value = self
            .column_of(name, field)
            .and_then(|j| m.values.get(j).cloned().flatten());
        let words = name.replace('_', " ");
        match value {
            Some(FieldValue::Text(x)) if rule.op == Op::NotIn => {
                format!("In {x}, {} {words} the rules leave out", article(&words))
            }
            Some(FieldValue::Text(x)) => {
                format!(
                    "In {x}, {} {words} the rules do not include",
                    article(&words)
                )
            }
            Some(FieldValue::Number(x)) if !rule.op.is_text() => {
                let unit = unit_of(rule);
                format!(
                    "{} {}, {}",
                    label_of(name, field),
                    if field {
                        shown(Some(x), None)
                    } else {
                        value_text(name, unit, Some(x))
                    },
                    needs_words(rule)
                )
            }
            _ if field => format!("No {words} given"),
            _ => format!("{}: no value", label_of(name, field)),
        }
    }

    /// The securities that pass every rule beyond the limit, in a sentence, with their value of
    /// what the matches are sorted by: "Also pass every rule, outside the top 10: EQIX (0.51)."
    pub(crate) fn also_words(&self) -> Option<String> {
        if self.also_passed.is_empty() {
            return None;
        }
        let sort = self.sort_column();
        let names: Vec<String> = self
            .also_passed
            .iter()
            .map(|m| match sort {
                Some(j) => format!("{} ({})", m.id, column_text(&self.columns[j], &m.values[j])),
                None => m.id.clone(),
            })
            .collect();
        let more = self.coverage.passed - self.coverage.matched - self.also_passed.len();
        let mut text = format!(
            "Also pass every rule, {} {}: {}",
            if sort.is_some() {
                "outside the top"
            } else {
                "beyond the first"
            },
            count(self.coverage.matched),
            names.join(", ")
        );
        if more > 0 {
            text.push_str(&format!(" and {} more", count(more)));
        }
        text.push('.');
        Some(text)
    }

    /// A strip's rules in words, joined by "and".
    pub(crate) fn strip_words(&self, s: &ThresholdStrip) -> String {
        s.rules
            .iter()
            .filter_map(|k| self.rules.get(*k))
            .map(|r| rule_words(&r.rule))
            .collect::<Vec<_>>()
            .join(" and ")
    }

    /// Whether the value `x` passes every rule of the strip.
    pub(crate) fn strip_passes(&self, s: &ThresholdStrip, x: f64) -> bool {
        s.rules
            .iter()
            .filter_map(|k| self.rules.get(*k))
            .all(|r| passes(&r.rule, x))
    }

    /// The scale a strip is drawn on: from about the second to the ninety-eighth percentile of
    /// its values, widened to take in every threshold, with a little room at each end. Values
    /// beyond it are drawn at its ends.
    pub(crate) fn strip_scale(&self, s: &ThresholdStrip) -> Option<Scale> {
        let xs: Vec<f64> = s
            .points
            .iter()
            .map(|p| on_scale(&s.name, p.value))
            .collect();
        let n = xs.len();
        if n == 0 {
            return None;
        }
        let at = |share: f64| xs[((n as f64 * share) as usize).min(n - 1)];
        let mut limits: Vec<f64> = Vec::new();
        let mut zone = Some((f64::NEG_INFINITY, f64::INFINITY));
        for r in s.rules.iter().filter_map(|k| self.rules.get(*k)) {
            let ts: Vec<f64> = thresholds(&r.rule)
                .into_iter()
                .map(|t| on_scale(&s.name, t))
                .collect();
            let (from, to) = match (r.rule.op, ts.as_slice()) {
                (Op::Above | Op::AtLeast, [t]) => (*t, f64::INFINITY),
                (Op::Below | Op::AtMost, [t]) => (f64::NEG_INFINITY, *t),
                (Op::Between, [a, b]) => (*a, *b),
                _ => (f64::NEG_INFINITY, f64::INFINITY),
            };
            zone = zone
                .map(|(a, b)| (a.max(from), b.min(to)))
                .filter(|(a, b)| a <= b);
            limits.extend(ts);
        }
        let lo = limits.iter().copied().fold(at(0.02), f64::min);
        let hi = limits.iter().copied().fold(at(0.98), f64::max);
        let pad = if hi > lo {
            (hi - lo) * 0.04
        } else if lo != 0.0 {
            lo.abs() * 0.1
        } else {
            1.0
        };
        Some(Scale {
            lo: lo - pad,
            hi: hi + pad,
            limits,
            zone,
        })
    }

    /// The rows of the table of every measure: those some security has, without the measures
    /// of the data.
    pub(crate) fn measure_rows(&self) -> Vec<&MeasureSpread> {
        self.spread
            .iter()
            .filter(|s| s.count > 0 && !DATA_MEASURES.contains(&s.name.as_str()))
            .collect()
    }

    /// The note under the table of every measure: what the columns are, the measures some
    /// securities lack and those none has.
    pub(crate) fn measures_note(&self) -> String {
        let mut text = "Over the securities that are not stale. A quarter of them are below the \
                        25% column and a quarter above the 75% column."
            .to_owned();
        let rows = self.measure_rows();
        let lacking: Vec<String> = rows
            .iter()
            .filter(|s| s.lacking > 0)
            .map(|s| {
                format!(
                    "{}: {} of {} have a value",
                    s.label,
                    count(s.count),
                    count(s.count + s.lacking)
                )
            })
            .collect();
        if !lacking.is_empty() {
            let shown = lacking.len().min(NAMED_IDS);
            text.push(' ');
            text.push_str(&lacking[..shown].join("; "));
            if lacking.len() > shown {
                text.push_str(&format!("; and {} more measures", lacking.len() - shown));
            }
            text.push('.');
        }
        let absent = |history: bool| -> Vec<String> {
            self.spread
                .iter()
                .filter(|s| s.count == 0 && s.lacking > 0)
                .filter(|s| (s.lacking_history == s.lacking) == history)
                .filter(|s| !DATA_MEASURES.contains(&s.name.as_str()))
                .map(|s| lower(&s.label))
                .collect()
        };
        let short = absent(true);
        if !short.is_empty() {
            text.push_str(&format!(
                " Not computed, as no history is long enough: {}.",
                either(&short, "and")
            ));
        }
        let other = absent(false);
        if !other.is_empty() {
            text.push_str(&format!(
                " No security has a value of {}.",
                either(&other, "or")
            ));
        }
        text
    }

    /// What the relative and risk-adjusted measures are taken against, in a line: "Benchmark:
    /// S&P 500 · risk-free rate: 4% a year". The benchmark appears when the screen has one; the
    /// rate always does, as the Sharpe and Sortino ratios use it even when it is the default 0%.
    pub(crate) fn basis_line(&self) -> Option<String> {
        let c = &self.context;
        let mut parts = Vec::new();
        if let Some(b) = &c.benchmark {
            parts.push(format!("benchmark: {b}"));
        }
        if c.risk_free_rate.is_finite() {
            parts.push(format!(
                "risk-free rate: {} a year",
                threshold(c.risk_free_rate, Some(MeasureUnit::Fraction))
            ));
        }
        let mut line = parts.join(" · ");
        if let Some(first) = line.get(..1) {
            line = first.to_uppercase() + &line[1..];
        }
        (!line.is_empty()).then(|| line + ".")
    }

    /// One line on the data that names only what is out of the ordinary: late, stale and short
    /// histories, and the screen's warnings.
    pub(crate) fn data_line(&self) -> String {
        let c = &self.coverage;
        let x = &self.exceptions;
        let as_of = crate::text::date(self.context.as_of);
        let days = self.context.max_carry_days;
        let mut parts: Vec<String> = Vec::new();
        if x.late.is_empty() && x.stale.is_empty() && x.too_short.is_empty() {
            parts.push(format!(
                "{} {} prices to {as_of}. None is stale or too short.",
                if c.securities == 1 { "The" } else { "All" },
                counted(c.securities, "security has", "securities have")
            ));
        } else {
            parts.push(format!(
                "{} of {} {}.",
                count(c.measured),
                counted(c.securities, "security has", "securities have"),
                if self.rules.is_empty() && self.context.sort.is_none() {
                    "at least two prices"
                } else {
                    "the prices every rule needs"
                }
            ));
            let one = |ids: &[String], single: &str, many: &str| -> String {
                format!(
                    "{} {}",
                    count(ids.len()),
                    if ids.len() == 1 { single } else { many }
                )
            };
            if !x.late.is_empty() {
                parts.push(format!(
                    "{} before {as_of}, within {days} days: {}.",
                    one(&x.late, "has its last price", "have their last price"),
                    id_list(&x.late)
                ));
            }
            if !x.stale.is_empty() {
                parts.push(format!(
                    "{} in the {days} days to {as_of} and {}: {}.",
                    one(&x.stale, "has no price", "have no price"),
                    if x.stale.len() == 1 {
                        "is left out"
                    } else {
                        "are left out"
                    },
                    id_list(&x.stale)
                ));
            }
            if !x.too_short.is_empty() {
                parts.push(format!(
                    "{}: {}.",
                    one(
                        &x.too_short,
                        "has too little history for a rule",
                        "have too little history for a rule"
                    ),
                    id_list(&x.too_short)
                ));
            }
        }
        if let Some(d) = x.common_start {
            parts.push(format!("Every history starts on {}.", crate::text::date(d)));
        }
        // Each formula as written, with the securities that have no value of it.
        let formulas: Vec<String> = self
            .context
            .formulas
            .iter()
            .map(|f| match f.lacking {
                0 => format!("{} = {}", f.name, f.formula),
                n => format!(
                    "{} = {} ({} without a value)",
                    f.name,
                    f.formula,
                    counted(n, "security", "securities")
                ),
            })
            .collect();
        if !formulas.is_empty() {
            parts.push(format!(
                "{}: {}.",
                if formulas.len() == 1 {
                    "Formula"
                } else {
                    "Formulas"
                },
                formulas.join("; ")
            ));
        }
        for issue in self.context.warnings.issues() {
            parts.push(sentence(&issue.message));
        }
        parts.join(" ")
    }
}

impl ScreenReport {
    /// The report as labelled tables for export: the coverage, the rules, the matches, the near
    /// misses and the spread of every measure.
    pub fn tables(&self) -> Vec<ReportTable> {
        let col = |key: &str, label: &str, kind: ColumnKind| Column {
            key: key.to_owned(),
            label: label.to_owned(),
            kind,
        };
        let table = crate::tables::table;
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
        // The columns of the matches, as the matches and the near misses hold them.
        let kinds: Vec<ColumnKind> = self
            .columns
            .iter()
            .enumerate()
            .map(|(j, k)| {
                let text = self
                    .matches
                    .iter()
                    .map(|m| &m.values)
                    .chain(self.near_misses.iter().map(|m| &m.values))
                    .any(|v| matches!(v.get(j), Some(Some(FieldValue::Text(_)))));
                match (k.field, text) {
                    (true, true) => ColumnKind::Text,
                    (true, false) => ColumnKind::Number,
                    (false, _) => kind_of(k.unit),
                }
            })
            .collect();
        let with_columns = |mut columns: Vec<Column>| {
            for (k, kind) in self.columns.iter().zip(&kinds) {
                columns.push(col(&k.name, &k.label, *kind));
            }
            columns
        };
        let cells = |values: &[Option<FieldValue>]| -> Vec<Cell> {
            values
                .iter()
                .map(|v| match v {
                    Some(FieldValue::Number(x)) => number(Some(*x)),
                    Some(FieldValue::Text(t)) => Cell::Text(t.clone()),
                    None => Cell::Missing,
                })
                .collect()
        };
        out.push(table(
            "matches",
            "Matches",
            with_columns(vec![
                col("rank", "Rank", ColumnKind::Integer),
                col("id", "Security", ColumnKind::Text),
            ]),
            self.matches
                .iter()
                .map(|m| {
                    let mut row = vec![whole(m.rank), Cell::Text(m.id.clone())];
                    row.extend(cells(&m.values));
                    row
                })
                .collect(),
        ));
        out.push(table(
            "near_misses",
            "Near misses",
            with_columns(vec![
                col("id", "Security", ColumnKind::Text),
                col("rule", "Rule failed", ColumnKind::Text),
            ]),
            self.near_misses
                .iter()
                .map(|m| {
                    let rule = self
                        .rules
                        .get(m.rule)
                        .map_or_else(String::new, |r| rule_words(&r.rule));
                    let mut row = vec![Cell::Text(m.id.clone()), Cell::Text(rule)];
                    row.extend(cells(&m.values));
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
        if !self.context.formulas.is_empty() {
            out.push(table(
                "formulas",
                "Formulas",
                vec![
                    col("name", "Name", ColumnKind::Text),
                    col("formula", "Formula", ColumnKind::Text),
                    col("lacking", "Without a value", ColumnKind::Integer),
                ],
                self.context
                    .formulas
                    .iter()
                    .map(|f| {
                        vec![
                            Cell::Text(f.name.clone()),
                            Cell::Text(f.formula.clone()),
                            whole(f.lacking),
                        ]
                    })
                    .collect(),
            ));
        }
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
        assert_eq!(rule_words(&rule), "RSI (14 days) between 30 and 70");
        assert_eq!(needs_words(&rule), "needs 30 to 70");
        let rule = Rule::one_of("sector", ["Energy", "Utilities", "Materials"]);
        assert_eq!(
            rule_words(&rule),
            "Sector is Energy, Utilities or Materials"
        );
        assert_eq!(
            rule_words(&Rule::above("return_1y", 0.0)),
            "1-year return above 0%"
        );
        // A price against its average is shown as the difference.
        let rule = Rule::above("price_to_sma_200", 1.0);
        assert_eq!(rule_words(&rule), "Price above its 200-day average");
        assert_eq!(needs_words(&rule), "needs above its average");
        assert_eq!(
            rule_words(&Rule::at_least("price_to_sma_50", 1.05)),
            "Price vs 50-day average at least +5%"
        );
        assert_eq!(
            value_text("price_to_sma_200", Some(MeasureUnit::Ratio), Some(0.983)),
            "−1.7%"
        );
        // A fall is better above its threshold.
        let rule = Rule::above("max_drawdown_3y", -0.3);
        assert_eq!(rule_words(&rule), "3-year max drawdown better than −30%");
        assert_eq!(needs_words(&rule), "needs better than −30%");
        assert_eq!(
            rule_words(&Rule::at_most("pe_ratio", 25.0)),
            "Pe ratio at most 25"
        );
        assert_eq!(
            either(
                &["3-year return, per year".to_owned(), "Beta".to_owned()],
                "and"
            ),
            "3-year return, per year; Beta"
        );
        // Close to a threshold: within 10% of it, or of the typical size for a threshold of 0.
        assert!(gap("rsi_14", 68.9, 70.0, 0.0) <= NEAR);
        assert!(gap("rsi_14", 62.7, 70.0, 0.0) > NEAR);
        assert!(gap("price_to_sma_200", 1.004, 1.0, 0.05) <= NEAR);
        assert_eq!(typical("return_1y", [-0.3, 0.1, 0.2].into_iter()), 0.2);
    }
}
