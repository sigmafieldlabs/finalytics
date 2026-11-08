//! Screening: measures and rules on many securities, each on its own history.
//!
//! A screen measures every security of a universe on its own dates, with no alignment between
//! them: trailing returns, risk, measures against a benchmark, technical indicators, income,
//! volume and the state of its data. Rules on the measures and on fields given with the
//! securities, such as a sector or a price-to-earnings ratio, keep the securities that meet all
//! of them, sorted and limited as asked.

mod formula;
mod measures;

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Deserializer, Serialize};

use finalytics_core::{Date, Error, Result, ValidationReport};

use measures::{DATES, DAYS_A_YEAR, Period, Prices, Setting};
pub use measures::{Measure, MeasureUnit};

/// The most problems reported with a universe's prices: one mistake tends to repeat.
const MOST_PROBLEMS: usize = 50;

/// Names a field may not take: those of the measures, and of the columns every security has.
const RESERVED: [&str; 6] = ["id", "status", "passed", "matched", "rank", "fields"];

// ── The securities ───────────────────────────────────────────────────────────────────────────

/// A value given with a security that the library does not compute: a number, such as a
/// price-to-earnings ratio, or text, such as a sector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum FieldValue {
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
}

impl From<f64> for FieldValue {
    fn from(value: f64) -> Self {
        FieldValue::Number(value)
    }
}

impl From<i32> for FieldValue {
    fn from(value: i32) -> Self {
        FieldValue::Number(f64::from(value))
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        FieldValue::Text(value.to_owned())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        FieldValue::Text(value)
    }
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::Number(x) => write!(f, "{x}"),
            FieldValue::Text(t) => f.write_str(t),
        }
    }
}

/// Numbers that may be written as `null` in JSON, which reads as NaN: no value.
fn nullable_floats<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<f64>, D::Error> {
    let values: Vec<Option<f64>> = Deserialize::deserialize(d)?;
    Ok(values.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect())
}

fn optional_nullable_floats<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Vec<f64>>, D::Error> {
    let values: Option<Vec<Option<f64>>> = Deserialize::deserialize(d)?;
    Ok(values.map(|v| v.into_iter().map(|x| x.unwrap_or(f64::NAN)).collect()))
}

/// One security's prices on its own dates: a close on each date, and its volume, dividends and
/// fields if known.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod screening { pub use finalytics_risk::screening::*; } }
/// use finalytics::Date;
/// use finalytics::screening::Series;
///
/// let dates = vec![Date::from_ymd(2026, 10, 29)?, Date::from_ymd(2026, 10, 30)?];
/// let series = Series::new("AAPL", dates, vec![201.5, 203.1])
///     .with_volume(vec![5.1e7, 4.8e7])
///     .with_field("sector", "Technology")
///     .with_field("pe_ratio", 31.2);
/// assert_eq!(series.fields.len(), 2);
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "SecuritySeries"))]
#[non_exhaustive]
pub struct Series {
    /// The security's identifier, such as `"AAPL"`.
    pub id: String,
    /// The dates of its closes, in increasing order.
    pub dates: Vec<Date>,
    /// Its close on each date. Adjusted closes, which include dividends and splits, give total
    /// returns. A close that is not a number (NaN, or `null` in JSON) is no close: that date is
    /// left out.
    #[serde(deserialize_with = "nullable_floats")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<Option<f64>>"))]
    pub close: Vec<f64>,
    /// The volume traded on each date, if known; NaN (`null`) where not.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_nullable_floats"
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<Option<f64>>>"))]
    pub volume: Option<Vec<f64>>,
    /// The dividend per share paid on each date, its ex-date, and zero on the others, if known;
    /// NaN (`null`) where not known, which counts as none paid unless every one is unknown.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_nullable_floats"
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<Option<f64>>>"))]
    pub dividends: Option<Vec<f64>>,
    /// Values the library does not compute, by name, such as `{"sector": "Technology",
    /// "pe_ratio": 18.4}`: numbers or text, for rules and results.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, FieldValue>,
}

impl Series {
    /// A security's closes on its dates, without volumes, dividends or fields.
    pub fn new(id: impl Into<String>, dates: Vec<Date>, close: Vec<f64>) -> Series {
        Series {
            id: id.into(),
            dates,
            close,
            volume: None,
            dividends: None,
            fields: BTreeMap::new(),
        }
    }

    /// The same series with the volume traded on each date.
    pub fn with_volume(mut self, volume: Vec<f64>) -> Series {
        self.volume = Some(volume);
        self
    }

    /// The same series with the dividend per share paid on each date, zero on the others.
    pub fn with_dividends(mut self, dividends: Vec<f64>) -> Series {
        self.dividends = Some(dividends);
        self
    }

    /// The same series with a field: a number or text, such as `("pe_ratio", 18.4)` or
    /// `("sector", "Technology")`.
    pub fn with_field(mut self, name: &str, value: impl Into<FieldValue>) -> Series {
        self.fields.insert(name.to_owned(), value.into());
        self
    }
}

/// Many securities' prices as flat columns: every security's rows one after another, with the
/// row each starts at. The bindings pass large universes this way.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Columns {
    /// Each security's identifier.
    pub ids: Vec<String>,
    /// The row each security's prices start at: the first is 0, and each is at least the one
    /// before.
    pub starts: Vec<usize>,
    /// Each row's date, as days since 1970-01-01.
    pub days: Vec<i32>,
    /// Each row's close; NaN where there is none.
    pub close: Vec<f64>,
    /// Each row's volume, if known; NaN where not.
    pub volume: Option<Vec<f64>>,
    /// Each row's dividend per share, if known: zero where none is paid, NaN where not known. A
    /// security whose dividends are all unknown has none given.
    pub dividends: Option<Vec<f64>>,
}

impl Columns {
    /// Columns of `ids`, starting at `starts`, with `days` and `close` for each row.
    pub fn new(ids: Vec<String>, starts: Vec<usize>, days: Vec<i32>, close: Vec<f64>) -> Columns {
        Columns {
            ids,
            starts,
            days,
            close,
            volume: None,
            dividends: None,
        }
    }
}

/// The values of one field for every security, in their order.
#[derive(Debug, Clone, PartialEq)]
enum FieldData {
    Numbers(Vec<Option<f64>>),
    Text(Vec<Option<String>>),
}

#[derive(Debug, Clone, PartialEq)]
struct FieldColumn {
    name: String,
    data: FieldData,
}

impl FieldColumn {
    fn get(&self, i: usize) -> Option<FieldValue> {
        match &self.data {
            FieldData::Numbers(v) => v[i].map(FieldValue::Number),
            FieldData::Text(v) => v[i].clone().map(FieldValue::Text),
        }
    }

    fn number(&self, i: usize) -> Option<f64> {
        match &self.data {
            FieldData::Numbers(v) => v[i],
            FieldData::Text(_) => None,
        }
    }

    fn text(&self, i: usize) -> Option<&str> {
        match &self.data {
            FieldData::Numbers(_) => None,
            FieldData::Text(v) => v[i].as_deref(),
        }
    }

    fn is_text(&self) -> bool {
        matches!(self.data, FieldData::Text(_))
    }
}

/// Many securities, each with its own dates and closes, and their volumes, dividends and fields
/// where given, checked: the data a screen measures.
#[derive(Debug, Clone, PartialEq)]
pub struct Securities {
    ids: Vec<String>,
    /// The first row of each security, and the number of rows after the last.
    offsets: Vec<usize>,
    days: Vec<i32>,
    close: Vec<f64>,
    volume: Option<Vec<f64>>,
    dividends: Option<Vec<f64>>,
    has_dividends: Vec<bool>,
    fields: Vec<FieldColumn>,
    positions: HashMap<String, usize>,
    warnings: ValidationReport,
}

/// Problems found checking prices, at most [`MOST_PROBLEMS`] of them.
struct Problems {
    report: ValidationReport,
    errors: usize,
}

impl Problems {
    fn new() -> Problems {
        Problems {
            report: ValidationReport::new(),
            errors: 0,
        }
    }

    fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        if self.errors < MOST_PROBLEMS {
            self.report.error(path, message);
        }
        self.errors += 1;
    }

    fn finish(mut self) -> ValidationReport {
        if self.errors > MOST_PROBLEMS {
            self.report.error(
                "series",
                format!(
                    "and {} more problems like these",
                    self.errors - MOST_PROBLEMS
                ),
            );
        }
        self.report
    }
}

fn day_text(day: i32) -> String {
    Date::from_days_since_epoch(day).map_or_else(|_| format!("day {day}"), |d| d.to_string())
}

impl Securities {
    /// The securities of `series`, in their order, checked: identifiers given once each,
    /// increasing dates, closes above zero (a NaN close is no close, and its date is left out,
    /// with a warning), volumes and dividends of zero or more with one per date.
    pub fn from_series(series: Vec<Series>) -> Result<Securities> {
        let mut columns = Columns::default();
        let rows: usize = series.iter().map(|s| s.dates.len()).sum();
        columns.days.reserve(rows);
        columns.close.reserve(rows);
        let any_volume = series.iter().any(|s| s.volume.is_some());
        let any_dividends = series.iter().any(|s| s.dividends.is_some());
        let mut volume = any_volume.then(|| Vec::with_capacity(rows));
        let mut dividends = any_dividends.then(|| Vec::with_capacity(rows));
        let mut has_dividends = Vec::with_capacity(series.len());
        let mut problems = Problems::new();
        let mut fields: Vec<(String, BTreeMap<String, FieldValue>)> = Vec::new();
        for (i, s) in series.into_iter().enumerate() {
            let n = s.dates.len();
            let mut lengths = |name: &str, len: Option<usize>| {
                if let Some(len) = len.filter(|len| *len != n) {
                    problems.error(
                        format!("series[{i}].{name}"),
                        format!("{}: {len} values for {n} dates", s.id),
                    );
                }
            };
            lengths("close", Some(s.close.len()));
            lengths("volume", s.volume.as_ref().map(Vec::len));
            lengths("dividends", s.dividends.as_ref().map(Vec::len));
            columns.starts.push(columns.days.len());
            columns
                .days
                .extend(s.dates.iter().map(|d| d.days_since_epoch()));
            let fill = |values: Option<&Vec<f64>>, out: &mut Vec<f64>, missing: f64| {
                for k in 0..n {
                    out.push(values.and_then(|v| v.get(k)).copied().unwrap_or(missing));
                }
            };
            fill(Some(&s.close), &mut columns.close, f64::NAN);
            if let Some(out) = volume.as_mut() {
                fill(s.volume.as_ref(), out, f64::NAN);
            }
            if let Some(out) = dividends.as_mut() {
                fill(s.dividends.as_ref(), out, f64::NAN);
            }
            has_dividends.push(s.dividends.is_some());
            if !s.fields.is_empty() {
                fields.push((s.id.clone(), s.fields));
            }
            columns.ids.push(s.id);
        }
        columns.volume = volume;
        columns.dividends = dividends;
        let report = problems.finish();
        report.into_result(())?;
        let mut securities = Securities::checked(columns, Some(has_dividends))?;
        if !fields.is_empty() {
            securities = securities.with_fields(fields.into_iter().collect())?;
        }
        Ok(securities)
    }

    /// The securities of `columns`, checked as [`Securities::from_series`] checks them.
    pub fn from_columns(columns: Columns) -> Result<Securities> {
        Securities::checked(columns, None)
    }

    fn checked(mut c: Columns, has_dividends: Option<Vec<bool>>) -> Result<Securities> {
        let mut problems = Problems::new();
        let rows = c.days.len();
        if c.ids.is_empty() {
            problems.error("series", "give at least one security");
        }
        if c.starts.len() != c.ids.len() {
            problems.error(
                "starts",
                format!(
                    "{} starts for {} securities: give the first row of each",
                    c.starts.len(),
                    c.ids.len()
                ),
            );
        }
        for (name, len) in [
            ("close", Some(c.close.len())),
            ("volume", c.volume.as_ref().map(Vec::len)),
            ("dividends", c.dividends.as_ref().map(Vec::len)),
        ] {
            if let Some(len) = len.filter(|len| *len != rows) {
                problems.error(name, format!("{len} values for {rows} dates"));
            }
        }
        if c.starts.first().is_some_and(|s| *s != 0) {
            problems.error("starts[0]", "the first security starts at row 0");
        }
        for (k, pair) in c.starts.windows(2).enumerate() {
            if pair[1] < pair[0] {
                problems.error(
                    format!("starts[{}]", k + 1),
                    format!("{} is before the start before it, {}", pair[1], pair[0]),
                );
            }
        }
        if c.starts.last().is_some_and(|s| *s > rows) {
            problems.error("starts", format!("a start is after the last row, {rows}"));
        }
        let report = problems.finish();
        report.into_result(())?;

        let mut offsets = c.starts.clone();
        offsets.push(rows);
        let mut problems = Problems::new();
        let mut positions = HashMap::with_capacity(c.ids.len());
        for (i, id) in c.ids.iter().enumerate() {
            if id.trim().is_empty() {
                problems.error(format!("series[{i}].id"), "is empty");
            } else if positions.insert(id.clone(), i).is_some() {
                problems.error(format!("series[{i}].id"), format!("{id} is given twice"));
            }
        }
        let mut warnings = ValidationReport::new();
        let mut dropped = vec![0usize; c.ids.len()];
        for (i, id) in c.ids.iter().enumerate() {
            let rows = offsets[i]..offsets[i + 1];
            let path = |name: &str, j: usize| format!("series[{i}].{name}[{}]", j - offsets[i]);
            for j in rows.clone() {
                if j > offsets[i] && c.days[j] <= c.days[j - 1] {
                    problems.error(
                        path("dates", j),
                        format!(
                            "{id}: {} does not follow {}",
                            day_text(c.days[j]),
                            day_text(c.days[j - 1])
                        ),
                    );
                    break;
                }
            }
            for j in rows.clone() {
                if Date::from_days_since_epoch(c.days[j]).is_err() {
                    problems.error(path("dates", j), format!("{id}: is not a date"));
                }
                let x = c.close[j];
                if x.is_nan() {
                    dropped[i] += 1;
                } else if !(x.is_finite() && x > 0.0) {
                    problems.error(
                        path("close", j),
                        format!(
                            "{id} on {}: {x} is not a price above zero",
                            day_text(c.days[j])
                        ),
                    );
                }
                if let Some(v) = c.volume.as_ref().map(|v| v[j])
                    && !v.is_nan()
                    && !(v.is_finite() && v >= 0.0)
                {
                    problems.error(
                        path("volume", j),
                        format!(
                            "{id} on {}: {v} is not a volume of zero or more",
                            day_text(c.days[j])
                        ),
                    );
                }
                if let Some(d) = c.dividends.as_ref().map(|d| d[j])
                    && !d.is_nan()
                    && !(d.is_finite() && d >= 0.0)
                {
                    problems.error(
                        path("dividends", j),
                        format!(
                            "{id} on {}: {d} is not a dividend of zero or more",
                            day_text(c.days[j])
                        ),
                    );
                }
            }
        }
        let report = problems.finish();
        report.into_result(())?;

        // A security has dividends when it was given them and at least one is a number; then a
        // dividend that is not a number is none paid.
        let mut has_dividends = has_dividends.unwrap_or_else(|| vec![true; c.ids.len()]);
        match c.dividends.as_mut() {
            Some(d) => {
                for (i, has) in has_dividends.iter_mut().enumerate() {
                    let rows = &mut d[offsets[i]..offsets[i + 1]];
                    *has &= rows.iter().any(|x| !x.is_nan());
                    for x in rows.iter_mut().filter(|x| x.is_nan()) {
                        *x = 0.0;
                    }
                }
            }
            None => has_dividends.iter_mut().for_each(|h| *h = false),
        }
        // A NaN close is no close: its rows are left out.
        let total: usize = dropped.iter().sum();
        if total > 0 {
            let keep: Vec<bool> = c.close.iter().map(|x| !x.is_nan()).collect();
            let retain = |values: &mut Vec<f64>| {
                let mut k = 0;
                values.retain(|_| {
                    k += 1;
                    keep[k - 1]
                });
            };
            let mut k = 0;
            c.days.retain(|_| {
                k += 1;
                keep[k - 1]
            });
            retain(&mut c.close);
            if let Some(v) = c.volume.as_mut() {
                retain(v);
            }
            if let Some(d) = c.dividends.as_mut() {
                retain(d);
            }
            let mut start = 0;
            for (i, removed) in dropped.iter().enumerate() {
                let len = offsets[i + 1] - offsets[i] - removed;
                offsets[i] = start;
                start += len;
            }
            if let Some(last) = offsets.last_mut() {
                *last = start;
            }
            let with: Vec<&str> = c
                .ids
                .iter()
                .zip(&dropped)
                .filter(|(_, d)| **d > 0)
                .map(|(id, _)| id.as_str())
                .collect();
            warnings.warning(
                "series",
                format!(
                    "{total} closes are not numbers, and their dates are left out: {}",
                    list(&with)
                ),
            );
        }
        let empty: Vec<&str> = c
            .ids
            .iter()
            .enumerate()
            .filter(|(i, _)| offsets[i + 1] == offsets[*i])
            .map(|(_, id)| id.as_str())
            .collect();
        if !empty.is_empty() {
            warnings.warning(
                "series",
                format!("{} have no closes: {}", empty.len(), list(&empty)),
            );
        }
        Ok(Securities {
            ids: c.ids,
            offsets,
            days: c.days,
            close: c.close,
            volume: c.volume,
            dividends: c.dividends,
            has_dividends,
            fields: Vec::new(),
            positions,
            warnings,
        })
    }

    /// The same securities with `fields` added, by security and then by name, such as
    /// `{"AAPL": {"sector": "Technology", "pe_ratio": 31.2}}`. A field is numbers or text for
    /// every security; a NaN number is no value. A security the universe does not hold is left
    /// out, with a warning.
    pub fn with_fields(
        mut self,
        fields: BTreeMap<String, BTreeMap<String, FieldValue>>,
    ) -> Result<Securities> {
        let n = self.ids.len();
        let mut report = ValidationReport::new();
        let mut unknown = Vec::new();
        let mut named: BTreeMap<&str, ()> = BTreeMap::new();
        for (id, values) in &fields {
            let Some(&i) = self.positions.get(id) else {
                unknown.push(id.as_str());
                continue;
            };
            for (name, value) in values {
                let path = format!("fields.{id}.{name}");
                if let Some(problem) = field_name_problem(name) {
                    if named.insert(name, ()).is_none() {
                        report.error(&path, problem);
                    }
                    continue;
                }
                let k = match self.fields.iter().position(|f| &f.name == name) {
                    Some(k) => k,
                    None => {
                        let data = match value {
                            FieldValue::Number(_) => FieldData::Numbers(vec![None; n]),
                            FieldValue::Text(_) => FieldData::Text(vec![None; n]),
                        };
                        self.fields.push(FieldColumn {
                            name: name.clone(),
                            data,
                        });
                        self.fields.len() - 1
                    }
                };
                match (&mut self.fields[k].data, value) {
                    (FieldData::Numbers(v), FieldValue::Number(x)) => {
                        if x.is_nan() {
                            v[i] = None;
                        } else if x.is_finite() {
                            v[i] = Some(*x);
                        } else {
                            report.error(&path, format!("{x} is not a finite number"));
                        }
                    }
                    (FieldData::Text(v), FieldValue::Text(t)) => v[i] = Some(t.clone()),
                    (FieldData::Numbers(_), FieldValue::Text(t)) => report.error(
                        &path,
                        format!("{t:?} is text, but {name} is a number for other securities"),
                    ),
                    (FieldData::Text(_), FieldValue::Number(x)) => report.error(
                        &path,
                        format!("{x} is a number, but {name} is text for other securities"),
                    ),
                }
            }
        }
        if !unknown.is_empty() {
            report.warning(
                "fields",
                format!(
                    "the universe has no prices for {}, so their fields are left out",
                    list(&unknown)
                ),
            );
        }
        let has_errors = report.has_errors();
        if has_errors {
            return Err(Error::Validation(report));
        }
        self.warnings.merge("", report);
        Ok(self)
    }

    /// The same securities with the fields named in `names` in that order, after any others,
    /// such as the order of a file's columns.
    pub fn with_field_order<S: AsRef<str>>(mut self, names: &[S]) -> Securities {
        self.fields.sort_by_key(|f| {
            names
                .iter()
                .position(|n| n.as_ref() == f.name)
                .map_or((0, 0), |k| (1, k))
        });
        self
    }

    /// The number of securities.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether there are no securities.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Each security's identifier, in order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The position of the security `id`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.positions.get(id).copied()
    }

    /// The dates of the `i`-th security's closes.
    pub fn dates(&self, i: usize) -> Vec<Date> {
        self.prices(i)
            .days
            .iter()
            .filter_map(|d| Date::from_days_since_epoch(*d).ok())
            .collect()
    }

    /// The `i`-th security's closes.
    pub fn close(&self, i: usize) -> &[f64] {
        self.prices(i).close
    }

    /// The names of the fields given, in the order they were first given.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    /// The `i`-th security's fields, by name.
    pub fn fields(&self, i: usize) -> BTreeMap<String, FieldValue> {
        self.fields
            .iter()
            .filter_map(|f| f.get(i).map(|v| (f.name.clone(), v)))
            .collect()
    }

    /// Whether any security has volumes.
    pub fn has_volume(&self) -> bool {
        self.volume.is_some()
    }

    /// Whether any security has dividends.
    pub fn has_dividends(&self) -> bool {
        self.has_dividends.iter().any(|d| *d)
    }

    /// The latest date of any security's closes.
    pub fn latest(&self) -> Option<Date> {
        (0..self.len())
            .filter_map(|i| self.prices(i).days.last().copied())
            .max()
            .and_then(|d| Date::from_days_since_epoch(d).ok())
    }

    /// The earliest date of any security's closes.
    pub fn earliest(&self) -> Option<Date> {
        (0..self.len())
            .filter_map(|i| self.prices(i).days.first().copied())
            .min()
            .and_then(|d| Date::from_days_since_epoch(d).ok())
    }

    /// Problems met reading the securities that did not stop them being used, such as closes
    /// that are not numbers.
    pub fn warnings(&self) -> &ValidationReport {
        &self.warnings
    }

    fn prices(&self, i: usize) -> Prices<'_> {
        let rows = self.offsets[i]..self.offsets[i + 1];
        Prices {
            days: &self.days[rows.clone()],
            close: &self.close[rows.clone()],
            volume: self.volume.as_ref().map(|v| &v[rows.clone()]),
            dividends: self
                .dividends
                .as_ref()
                .filter(|_| self.has_dividends[i])
                .map(|d| &d[rows]),
        }
    }

    fn field(&self, name: &str) -> Option<&FieldColumn> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// Why `name` cannot name a field, if it cannot.
fn field_name_problem(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        Some("a field needs a name".to_owned())
    } else if Measure::from_name(name).is_some() || DATES.contains(&name) {
        Some(format!(
            "{name} is the name of a measure the screen computes: give the field another name"
        ))
    } else if RESERVED.contains(&name) {
        Some(format!(
            "{name} names a column of every result: give the field another name"
        ))
    } else {
        None
    }
}

/// Names in a list for a message: the first five, and how many more.
fn list(names: &[&str]) -> String {
    let shown = names.iter().take(5).copied().collect::<Vec<_>>().join(", ");
    match names.len() {
        n if n > 5 => format!("{shown} and {} more", n - 5),
        _ => shown,
    }
}

// ── Options ──────────────────────────────────────────────────────────────────────────────────

/// How a rule compares a security's value: with a number, a range or a list of text values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Op {
    /// Below the value.
    #[serde(rename = "<")]
    Below,
    /// At most the value.
    #[serde(rename = "<=")]
    AtMost,
    /// Above the value.
    #[serde(rename = ">")]
    Above,
    /// At least the value.
    #[serde(rename = ">=")]
    AtLeast,
    /// From the first value of `[low, high]` to the second, both included.
    #[serde(rename = "between")]
    Between,
    /// One of the text values.
    #[serde(rename = "in")]
    In,
    /// None of the text values.
    #[serde(rename = "not_in")]
    NotIn,
}

impl Op {
    /// The comparison as written in rules, such as `">="`.
    pub fn symbol(self) -> &'static str {
        match self {
            Op::Below => "<",
            Op::AtMost => "<=",
            Op::Above => ">",
            Op::AtLeast => ">=",
            Op::Between => "between",
            Op::In => "in",
            Op::NotIn => "not_in",
        }
    }

    /// Whether the comparison is of text.
    pub fn is_text(self) -> bool {
        matches!(self, Op::In | Op::NotIn)
    }
}

/// The number a numeric rule compares with, or the `[low, high]` range of `between`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuleValue {
    /// One number.
    Number(f64),
    /// A range: the lowest and the highest values.
    Range([f64; 2]),
}

/// A rule every match must meet: a measure or a numeric field compared with a number (`<`,
/// `<=`, `>`, `>=`) or a range (`between`), or a text field that is (`in`) or is not (`not_in`)
/// one of a list of values. A security without the value a rule tests fails it.
///
/// ```
/// # mod finalytics { pub mod screening { pub use finalytics_risk::screening::*; } }
/// use finalytics::screening::{Op, Rule};
///
/// let rule = Rule::above("return_1y", 0.10);
/// assert_eq!(rule.measure.as_deref(), Some("return_1y"));
/// assert_eq!(rule.op, Op::Above);
/// let rule = Rule::one_of("sector", ["Technology", "Energy"]);
/// assert_eq!(rule.field.as_deref(), Some("sector"));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Rule {
    /// The measure the rule tests, such as `"return_1y"`; or give `field`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Measure>"))]
    pub measure: Option<String>,
    /// The field the rule tests, such as `"sector"` or `"pe_ratio"`; or give `measure`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// How the value is compared.
    pub op: Op,
    /// The number compared with, or `[low, high]` for `between`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RuleValue>,
    /// The text values of `in` and `not_in`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

impl Rule {
    /// A numeric rule on `name`: a measure if it names one, otherwise a field.
    fn numeric(name: &str, op: Op, value: RuleValue) -> Rule {
        let (measure, field) = target(name);
        Rule {
            measure,
            field,
            op,
            value: Some(value),
            values: None,
        }
    }

    fn text<S: Into<String>>(name: &str, op: Op, values: impl IntoIterator<Item = S>) -> Rule {
        Rule {
            measure: None,
            field: Some(name.to_owned()),
            op,
            value: None,
            values: Some(values.into_iter().map(Into::into).collect()),
        }
    }

    /// `name`, a measure or a numeric field, above `value`.
    pub fn above(name: &str, value: f64) -> Rule {
        Rule::numeric(name, Op::Above, RuleValue::Number(value))
    }

    /// `name` at least `value`.
    pub fn at_least(name: &str, value: f64) -> Rule {
        Rule::numeric(name, Op::AtLeast, RuleValue::Number(value))
    }

    /// `name` below `value`.
    pub fn below(name: &str, value: f64) -> Rule {
        Rule::numeric(name, Op::Below, RuleValue::Number(value))
    }

    /// `name` at most `value`.
    pub fn at_most(name: &str, value: f64) -> Rule {
        Rule::numeric(name, Op::AtMost, RuleValue::Number(value))
    }

    /// `name` from `low` to `high`, both included.
    pub fn between(name: &str, low: f64, high: f64) -> Rule {
        Rule::numeric(name, Op::Between, RuleValue::Range([low, high]))
    }

    /// The text field `name` one of `values`.
    pub fn one_of<S: Into<String>>(name: &str, values: impl IntoIterator<Item = S>) -> Rule {
        Rule::text(name, Op::In, values)
    }

    /// The text field `name` none of `values`.
    pub fn none_of<S: Into<String>>(name: &str, values: impl IntoIterator<Item = S>) -> Rule {
        Rule::text(name, Op::NotIn, values)
    }

    /// The name of the measure or field the rule tests.
    pub fn name(&self) -> &str {
        self.measure
            .as_deref()
            .or(self.field.as_deref())
            .unwrap_or_default()
    }
}

/// `name` as the measure it names, or else as a field.
fn target(name: &str) -> (Option<String>, Option<String>) {
    if Measure::from_name(name).is_some() || DATES.contains(&name) {
        (Some(name.to_owned()), None)
    } else {
        (None, Some(name.to_owned()))
    }
}

/// Which way matches are sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// The smallest first.
    Ascending,
    /// The largest first.
    Descending,
}

/// How matches are sorted: by a measure or a numeric field, one way or the other. Securities
/// without a value come last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Sort {
    /// The measure sorted by; or give `field`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Measure>"))]
    pub measure: Option<String>,
    /// The numeric field sorted by; or give `measure`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// The largest first (`descending`) or the smallest first (`ascending`).
    pub order: Order,
}

impl Sort {
    /// Sorted by `name`, a measure if it names one and otherwise a numeric field, in `order`.
    pub fn by(name: &str, order: Order) -> Sort {
        let (measure, field) = target(name);
        Sort {
            measure,
            field,
            order,
        }
    }

    /// The name of the measure or field sorted by.
    pub fn name(&self) -> &str {
        self.measure
            .as_deref()
            .or(self.field.as_deref())
            .unwrap_or_default()
    }
}

/// Formulas by name, in the order given, such as `risk_adjusted = return_1y / volatility`: each
/// is a numeric field of every security, computed from the measures, the numeric fields and the
/// formulas before it. In JSON, an object from names to formulas, whose keys keep their order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Formulas(pub Vec<(String, String)>);

impl Formulas {
    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for Formulas {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, formula) in &self.0 {
            map.serialize_entry(name, formula)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Formulas {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct InOrder;
        impl<'de> serde::de::Visitor<'de> for InOrder {
            type Value = Formulas;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(
                    "formulas by name, such as {\"risk_adjusted\": \"return_1y / volatility\"}",
                )
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Formulas, A::Error> {
                let mut out: Vec<(String, String)> = Vec::new();
                while let Some((name, formula)) = map.next_entry::<String, String>()? {
                    if out.iter().any(|(n, _)| *n == name) {
                        return Err(serde::de::Error::custom(format!("{name} is given twice")));
                    }
                    out.push((name, formula));
                }
                Ok(Formulas(out))
            }
        }
        deserializer.deserialize_map(InOrder)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Formulas {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Formulas".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        generator.subschema_for::<BTreeMap<String, String>>()
    }
}

/// What a screen measures and keeps. Everything is optional: by default every measure the
/// universe allows, on its latest date, with no rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct ScreenOptions {
    /// The date measured to: the latest date in the universe by default.
    pub as_of: Option<Date>,
    /// The most days a security's last close may be before `as_of`; a security whose last close
    /// is older is stale: listed, but not measured. 5 by default.
    pub max_carry_days: u32,
    /// The measures to compute, by name, such as `["return_1y", "rsi_14"]`: by default every
    /// measure the universe allows (those against a benchmark need one, and those of volumes
    /// and dividends need them). The measures the rules and the sort use are computed too.
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<Measure>>"))]
    pub measures: Option<Vec<String>>,
    /// The rules a security must all meet to match.
    pub rules: Vec<Rule>,
    /// How the matches are sorted; in the universe's order by default.
    pub sort: Option<Sort>,
    /// The most matches to keep, after sorting.
    pub limit: Option<usize>,
    /// Formulas by name, in order, such as `{"risk_adjusted": "return_1y / volatility"}`: each
    /// a numeric field of every security, which rules, the sort and the results take as they
    /// take a field. A formula is numbers, the names of measures, numeric fields and earlier
    /// formulas, `+ - * /`, unary minus, parentheses, and `abs(x)`, `sqrt(x)`, `log(x)`
    /// (natural), `min(a, b, ...)` and `max(a, b, ...)`. A security has no value where an input
    /// has none, where the formula divides by zero, takes the square root of a negative number
    /// or the logarithm of one at most zero, or where the result is not finite.
    #[serde(skip_serializing_if = "Formulas::is_empty")]
    #[cfg_attr(feature = "schema", schemars(with = "BTreeMap<String, String>"))]
    pub formulas: Formulas,
}

impl Default for ScreenOptions {
    fn default() -> Self {
        ScreenOptions {
            as_of: None,
            max_carry_days: 5,
            measures: None,
            rules: Vec::new(),
            sort: None,
            limit: None,
            formulas: Formulas::default(),
        }
    }
}

// ── Results ──────────────────────────────────────────────────────────────────────────────────

/// Whether a security was measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "ScreenStatus"))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// Measured, with the history every rule and the sort need.
    Measured,
    /// Measured, but with fewer than two closes up to the date measured to, or without the
    /// history a rule or the sort needs: it fails the rules it cannot be judged on.
    TooShort,
    /// Its last close is too long before the date measured to: listed, but not measured.
    Stale,
}

/// One security's row of a screen: its measures, its fields and whether it matched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct SecurityScreen {
    /// The security.
    pub id: String,
    /// Whether it was measured.
    pub status: Status,
    /// Whether it meets every rule.
    pub passed: bool,
    /// Whether it is one of the matches kept: it passed, and is within the limit.
    pub matched: bool,
    /// Its place among the matches kept, from 1.
    pub rank: Option<usize>,
    /// The date of its first close.
    pub first_date: Option<Date>,
    /// The date of its last close on or before the date measured to.
    pub last_date: Option<Date>,
    /// Its value of each measure of the results' `measures`, in their order; absent where it
    /// has none.
    pub values: Vec<Option<f64>>,
    /// Its fields, by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, FieldValue>,
}

/// How many securities were measured and how many matched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Coverage {
    /// The securities in the universe.
    pub securities: usize,
    /// Those measured with the history the rules and the sort need.
    pub measured: usize,
    /// Those measured with too short a history.
    pub too_short: usize,
    /// Those whose last close is too old: listed, but not measured.
    pub stale: usize,
    /// Those that meet every rule.
    pub passed: usize,
    /// The matches kept: those that passed, within the limit.
    pub matched: usize,
    /// For each measure of the results' `measures`, how many securities that are not stale
    /// have no value of it.
    pub lacking: Vec<usize>,
    /// For each measure, how many of those without a value have too short a history for it,
    /// such as two years of closes for the 3-year return.
    #[serde(default)]
    pub lacking_history: Vec<usize>,
}

/// A formula, with how many securities have no value of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FormulaResult {
    /// Its name.
    pub name: String,
    /// The formula, as given.
    pub formula: String,
    /// How many securities that are not stale have no value of it.
    pub lacking: usize,
}

/// A rule, with how many securities met it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RuleResult {
    /// The rule.
    pub rule: Rule,
    /// How many securities that are not stale meet it.
    pub passed: usize,
    /// How many that are not stale have no value of what it tests, and so fail it.
    pub lacking: usize,
    /// How many meet it and every rule before it.
    pub remaining: usize,
}

/// The raw results of a screen: one row per security, in the universe's order, with each
/// measure's value, its fields and whether it matched; the matches in order; each rule with how
/// many met it; and how many were measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ScreenResults {
    /// The date measured to.
    pub as_of: Date,
    /// The most days a last close could be before `as_of` without being stale.
    pub max_carry_days: u32,
    /// The benchmark, if any.
    pub benchmark: Option<String>,
    /// The annual risk-free rate of the Sharpe and Sortino ratios.
    pub risk_free_rate: f64,
    /// The measures computed, in the order of each security's `values`.
    pub measures: Vec<Measure>,
    /// The names of the fields given, then of the formulas, which each security's `fields`
    /// give as numbers.
    pub fields: Vec<String>,
    /// The formulas, in order, with how many securities have no value of each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formulas: Vec<FormulaResult>,
    /// Each rule, with how many met it.
    pub rules: Vec<RuleResult>,
    /// How the matches are sorted.
    pub sort: Option<Sort>,
    /// The most matches kept.
    pub limit: Option<usize>,
    /// The matches kept, in order.
    pub matches: Vec<String>,
    /// How many securities were measured, passed and matched, and how many lacked each measure.
    pub coverage: Coverage,
    /// One row per security, in the universe's order.
    pub securities: Vec<SecurityScreen>,
    /// Problems that did not stop the screen, such as closes left out.
    pub warnings: ValidationReport,
}

impl ScreenResults {
    /// The row of the security `id`.
    pub fn security(&self, id: &str) -> Option<&SecurityScreen> {
        self.securities.iter().find(|s| s.id == id)
    }

    /// The value of `measure` for the security `id`, if it was computed and the security has
    /// one.
    pub fn value(&self, id: &str, measure: Measure) -> Option<f64> {
        let k = self.measures.iter().position(|m| *m == measure)?;
        self.security(id)?.values[k]
    }

    /// The matches kept, in order.
    pub fn matched(&self) -> Vec<&SecurityScreen> {
        let mut rows: Vec<&SecurityScreen> = self.securities.iter().filter(|s| s.matched).collect();
        rows.sort_by_key(|s| s.rank);
        rows
    }

    /// Every security's value of `measure`, in the universe's order, if it was computed.
    pub fn column(&self, measure: Measure) -> Option<Vec<Option<f64>>> {
        let k = self.measures.iter().position(|m| *m == measure)?;
        Some(self.securities.iter().map(|s| s.values[k]).collect())
    }
}

/// The matches shown when raw results are printed.
const PRINTED: usize = 20;

/// A value as printed in raw results: a percentage for a fraction, a number otherwise.
fn printed(x: Option<f64>, unit: MeasureUnit) -> String {
    let Some(x) = x else {
        return "–".to_owned();
    };
    let text = match unit {
        MeasureUnit::Fraction => format!("{:.1}%", x * 100.0),
        MeasureUnit::Count | MeasureUnit::Flag => format!("{x:.0}"),
        MeasureUnit::Index => format!("{x:.1}"),
        _ => format!("{x:.2}"),
    };
    text.replace('-', "\u{2212}")
}

/// The results as text: the matches, each with the measures the sort and the rules use (or the
/// first measures), and the counts.
impl std::fmt::Display for ScreenResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = &self.coverage;
        writeln!(
            f,
            "ScreenResults · as of {} · {} of {} securities matched",
            self.as_of, c.matched, c.securities
        )?;
        let mut shown: Vec<usize> = Vec::new();
        let wanted = self
            .sort
            .iter()
            .filter_map(|s| s.measure.as_deref())
            .chain(self.rules.iter().filter_map(|r| r.rule.measure.as_deref()));
        for name in wanted {
            if let Some(k) = self.measures.iter().position(|m| m.name() == name)
                && !shown.contains(&k)
            {
                shown.push(k);
            }
        }
        for k in 0..self.measures.len() {
            if shown.len() >= 4 {
                break;
            }
            if !shown.contains(&k) {
                shown.push(k);
            }
        }
        shown.truncate(4);
        let matched = self.matched();
        let rows: Vec<Vec<String>> = matched
            .iter()
            .take(PRINTED)
            .map(|s| {
                let mut row = vec![s.rank.unwrap_or_default().to_string(), s.id.clone()];
                row.extend(
                    shown
                        .iter()
                        .map(|k| printed(s.values[*k], self.measures[*k].unit())),
                );
                row
            })
            .collect();
        let mut heads = vec!["#".to_owned(), "Security".to_owned()];
        heads.extend(shown.iter().map(|k| self.measures[*k].name().to_owned()));
        let widths: Vec<usize> = (0..heads.len())
            .map(|i| {
                rows.iter()
                    .map(|r| r[i].chars().count())
                    .chain([heads[i].chars().count()])
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let line = |cells: &[String]| -> String {
            let parts: Vec<String> = cells
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let pad = " ".repeat(widths[i] - cell.chars().count());
                    if i == 1 {
                        format!("{cell}{pad}")
                    } else {
                        format!("{pad}{cell}")
                    }
                })
                .collect();
            format!("  {}", parts.join("  ")).trim_end().to_owned()
        };
        if !rows.is_empty() {
            writeln!(f)?;
            writeln!(f, "{}", line(&heads))?;
            for row in &rows {
                writeln!(f, "{}", line(row))?;
            }
            if matched.len() > PRINTED {
                writeln!(f, "  and {} more", matched.len() - PRINTED)?;
            }
        }
        writeln!(f)?;
        write!(
            f,
            "  {} measured · {} too short · {} stale · {} passed every rule",
            c.measured, c.too_short, c.stale, c.passed
        )
    }
}

// ── The screen ───────────────────────────────────────────────────────────────────────────────

/// What a rule or the sort tests, resolved against the universe.
#[derive(Clone, Copy)]
enum Target {
    /// The measure at this place in the measures computed.
    Measure(usize),
    /// The field at this place in the universe's fields.
    Field(usize),
    /// The formula at this place in the formulas.
    Formula(usize),
}

/// What a name in a formula stands for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Input {
    Measure(usize),
    Field(usize),
    Formula(usize),
}

/// A formula read, with the inputs its names stand for.
struct Compiled {
    expr: formula::Expr,
    inputs: Vec<Input>,
}

enum Test {
    Compare(Op, f64),
    Range(f64, f64),
    Text(bool, Vec<String>),
}

struct Resolved {
    target: Target,
    test: Test,
}

/// A rule's verdict on one security.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    Lacking,
}

/// The options checked against the universe.
struct Plan {
    as_of: Date,
    measures: Vec<Measure>,
    formulas: Vec<Compiled>,
    rules: Vec<Resolved>,
    sort: Option<(Target, Order)>,
    /// The measures the rules and the sort test, by their place in `measures`.
    needed: Vec<Measure>,
}

/// Why `measure` cannot be computed for this universe, if it cannot.
fn unavailable(measure: Measure, securities: &Securities, benchmark: bool) -> Option<String> {
    if measure.needs_benchmark() && !benchmark {
        Some(format!(
            "{measure} is measured against a benchmark: give the universe one"
        ))
    } else if measure.needs_volume() && !securities.has_volume() {
        Some(format!("{measure} needs volumes, and no security has them"))
    } else if measure.needs_dividends() && !securities.has_dividends() {
        Some(format!(
            "{measure} needs dividends, and no security has them"
        ))
    } else {
        None
    }
}

/// The measure `name` names, or why it names none.
fn known_measure(
    name: &str,
    securities: &Securities,
    benchmark: bool,
) -> std::result::Result<Measure, String> {
    match Measure::from_name(name) {
        Some(m) => unavailable(m, securities, benchmark).map_or(Ok(m), Err),
        None if DATES.contains(&name) => Err(format!(
            "{name} is a date, which every result gives: rules and sorting take numbers"
        )),
        None if securities.field(name).is_some() => Err(format!(
            "{name} is a field, not a measure: give it as the field"
        )),
        None => Err(format!("unknown measure {name}")),
    }
}

/// The edit distance between two names, for suggesting the one meant.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let current = row[j + 1];
            row[j + 1] = if ca == *cb {
                previous
            } else {
                1 + previous.min(row[j]).min(row[j + 1])
            };
            previous = current;
        }
    }
    row[b.len()]
}

/// The name among `names` closest to `name`, if one is near.
fn nearest<'a>(name: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    names
        .into_iter()
        .map(|n| (distance(name, n), n))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, n)| n)
}

/// The field `name` names, or why it names none.
fn known_field(name: &str, securities: &Securities) -> std::result::Result<usize, String> {
    match securities.fields.iter().position(|f| f.name == name) {
        Some(k) => Ok(k),
        None if Measure::from_name(name).is_some() => Err(format!(
            "{name} is a measure, not a field: give it as the measure"
        )),
        None if securities.fields.is_empty() => Err(format!(
            "unknown measure or field {name}: the universe has no fields"
        )),
        None => Err(format!(
            "unknown measure or field {name}; the fields are {}",
            list(&securities.field_names())
        )),
    }
}

fn plan(
    securities: &Securities,
    benchmark: Option<usize>,
    risk_free_rate: f64,
    options: &ScreenOptions,
) -> Result<Plan> {
    let mut report = ValidationReport::new();
    let bench = benchmark.is_some();
    if !(risk_free_rate.is_finite() && risk_free_rate > -1.0) {
        report.error(
            "risk_free_rate",
            format!("must be an annual rate above -1, such as 0.04, not {risk_free_rate}"),
        );
    }
    let as_of = match options.as_of.or_else(|| securities.latest()) {
        Some(d) => d,
        None => {
            report.error("series", "no security has a close");
            Date::EPOCH
        }
    };

    let mut measures: Vec<Measure> = Vec::new();
    let add = |m: Measure, measures: &mut Vec<Measure>| -> usize {
        match measures.iter().position(|x| *x == m) {
            Some(k) => k,
            None => {
                measures.push(m);
                measures.len() - 1
            }
        }
    };
    match &options.measures {
        Some(names) => {
            for (i, name) in names.iter().enumerate() {
                if DATES.contains(&name.as_str()) {
                    continue;
                }
                match Measure::from_name(name) {
                    Some(m) => match unavailable(m, securities, bench) {
                        Some(problem) => report.error(format!("measures[{i}]"), problem),
                        None => {
                            add(m, &mut measures);
                        }
                    },
                    None => {
                        report.error(format!("measures[{i}]"), format!("unknown measure {name}"))
                    }
                }
            }
        }
        None => {
            for m in Measure::all().filter(|m| unavailable(*m, securities, bench).is_none()) {
                measures.push(m);
            }
        }
    }

    // The formulas, in order: each may use the measures, the numeric fields and the formulas
    // before it.
    let mut formulas: Vec<Compiled> = Vec::new();
    let names: Vec<&str> = options.formulas.0.iter().map(|(n, _)| n.as_str()).collect();
    for (k, (name, text)) in options.formulas.0.iter().enumerate() {
        let path = format!("formulas.{name}");
        let well_formed = name.chars().next().is_some_and(char::is_alphabetic)
            && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !well_formed {
            report.error(
                &path,
                format!(
                    "{name:?} cannot be a formula's name: use letters, digits and underscores, \
                     starting with a letter"
                ),
            );
            continue;
        }
        if let Some(problem) = field_name_problem(name) {
            report.error(&path, problem.replace("the field", "the formula"));
            continue;
        }
        if securities.field(name).is_some() {
            report.error(
                &path,
                format!("{name} is a field of the universe: give the formula another name"),
            );
            continue;
        }
        let mut inputs: Vec<Input> = Vec::new();
        let mut resolve = |used: &str| -> std::result::Result<usize, String> {
            let input = if let Some(m) = Measure::from_name(used) {
                if let Some(problem) = unavailable(m, securities, bench) {
                    return Err(problem);
                }
                Input::Measure(add(m, &mut measures))
            } else if let Some(f) = securities.fields.iter().position(|f| f.name == used) {
                if securities.fields[f].is_text() {
                    return Err(format!("{used} is text: a formula takes numbers"));
                }
                Input::Field(f)
            } else if let Some(j) = names.iter().position(|n| *n == used) {
                if j >= k {
                    return Err(if j == k {
                        "is this formula's own name".to_owned()
                    } else {
                        format!("{used} is a formula given after this one: give it first")
                    });
                }
                Input::Formula(j)
            } else if DATES.contains(&used) {
                return Err(format!("{used} is a date: a formula takes numbers"));
            } else {
                let candidates: Vec<String> = Measure::all()
                    .map(|m| m.name().to_owned())
                    .chain(securities.fields.iter().map(|f| f.name.clone()))
                    .chain(names[..k].iter().map(|n| (*n).to_owned()))
                    .collect();
                return Err(match nearest(used, candidates.iter().map(String::as_str)) {
                    Some(near) => format!("unknown name; did you mean {near}?"),
                    None => "unknown name: not a measure, a numeric field or an earlier formula"
                        .to_owned(),
                });
            };
            Ok(match inputs.iter().position(|x| *x == input) {
                Some(p) => p,
                None => {
                    inputs.push(input);
                    inputs.len() - 1
                }
            })
        };
        match formula::parse(text, &mut resolve) {
            Ok(expr) => formulas.push(Compiled { expr, inputs }),
            Err(problem) => {
                report.error(&path, problem);
                // Its place is kept, so that later formulas' inputs stay where they are.
                formulas.push(Compiled {
                    expr: formula::Expr::Number(f64::NAN),
                    inputs: Vec::new(),
                });
            }
        }
    }

    let mut needed = Vec::new();
    let mut resolve = |path: &str,
                       measure: &Option<String>,
                       field: &Option<String>,
                       report: &mut ValidationReport,
                       measures: &mut Vec<Measure>|
     -> Option<Target> {
        match (measure, field) {
            (Some(_), Some(_)) => {
                report.error(path, "give a measure or a field, not both");
                None
            }
            (None, None) => {
                report.error(path, "give the measure or the field it tests");
                None
            }
            (Some(name), None) => match known_measure(name, securities, bench) {
                Ok(m) => {
                    if !needed.contains(&m) {
                        needed.push(m);
                    }
                    Some(Target::Measure(add(m, measures)))
                }
                Err(problem) => {
                    report.error(format!("{path}.measure"), problem);
                    None
                }
            },
            (None, Some(name)) => match names.iter().position(|n| n == name) {
                Some(k) => Some(Target::Formula(k)),
                None => match known_field(name, securities) {
                    Ok(k) => Some(Target::Field(k)),
                    Err(problem) => {
                        report.error(format!("{path}.field"), problem);
                        None
                    }
                },
            },
        }
    };

    let mut rules = Vec::new();
    for (i, rule) in options.rules.iter().enumerate() {
        let path = format!("rules[{i}]");
        let target = resolve(
            &path,
            &rule.measure,
            &rule.field,
            &mut report,
            &mut measures,
        );
        let text_target =
            matches!(target, Some(Target::Field(k)) if securities.fields[k].is_text());
        let name = rule.name();
        let test = if rule.op.is_text() {
            if rule.value.is_some() {
                report.error(
                    format!("{path}.value"),
                    format!(
                        "{} takes text values: give them as values",
                        rule.op.symbol()
                    ),
                );
            }
            if target.is_some() && !text_target {
                report.error(
                    format!("{path}.op"),
                    format!(
                        "{name} is a number: compare it with <, <=, >, >= or between, not {}",
                        rule.op.symbol()
                    ),
                );
            }
            match &rule.values {
                Some(values) if !values.is_empty() => {
                    Some(Test::Text(rule.op == Op::In, values.clone()))
                }
                _ => {
                    report.error(
                        format!("{path}.values"),
                        "give the text values to look for, such as [\"Technology\"]",
                    );
                    None
                }
            }
        } else {
            if rule.values.is_some() {
                report.error(
                    format!("{path}.values"),
                    format!(
                        "{} compares with a number: give it as value",
                        rule.op.symbol()
                    ),
                );
            }
            if text_target {
                report.error(
                    format!("{path}.op"),
                    format!(
                        "{name} is text: test it with in or not_in, not {}",
                        rule.op.symbol()
                    ),
                );
            }
            match (rule.op, rule.value) {
                (Op::Between, Some(RuleValue::Range([low, high]))) => {
                    if !(low.is_finite() && high.is_finite()) {
                        report.error(format!("{path}.value"), "the range must be finite numbers");
                        None
                    } else if low > high {
                        report.error(
                            format!("{path}.value"),
                            format!("the low end {low} is above the high end {high}"),
                        );
                        None
                    } else {
                        Some(Test::Range(low, high))
                    }
                }
                (Op::Between, _) => {
                    report.error(
                        format!("{path}.value"),
                        "between takes a range: give it as [low, high]",
                    );
                    None
                }
                (op, Some(RuleValue::Number(x))) => {
                    if x.is_finite() {
                        Some(Test::Compare(op, x))
                    } else {
                        report.error(
                            format!("{path}.value"),
                            format!("{x} is not a finite number"),
                        );
                        None
                    }
                }
                (op, Some(RuleValue::Range(_))) => {
                    report.error(
                        format!("{path}.value"),
                        format!(
                            "{} compares with one number; between takes a range",
                            op.symbol()
                        ),
                    );
                    None
                }
                (op, None) => {
                    report.error(
                        format!("{path}.value"),
                        format!("give the number {name} is compared with by {}", op.symbol()),
                    );
                    None
                }
            }
        };
        if let (Some(target), Some(test)) = (target, test) {
            rules.push(Resolved { target, test });
        }
    }

    let sort = match &options.sort {
        Some(sort) => {
            let target = resolve(
                "sort",
                &sort.measure,
                &sort.field,
                &mut report,
                &mut measures,
            );
            match target {
                Some(Target::Field(k)) if securities.fields[k].is_text() => {
                    report.error(
                        "sort.field",
                        format!(
                            "{} is text: sort by a measure or a numeric field",
                            securities.fields[k].name
                        ),
                    );
                    None
                }
                Some(t) => Some((t, sort.order)),
                None => None,
            }
        }
        None => None,
    };
    if options.limit == Some(0) {
        report.error("limit", "must be at least 1");
    }
    report.into_result(Plan {
        as_of,
        measures,
        formulas,
        rules,
        sort,
        needed,
    })
}

/// One security measured, judged on every rule.
struct Row {
    values: Vec<Option<f64>>,
    /// The value of each formula.
    formulas: Vec<Option<f64>>,
    /// For each measure, whether it has no value for want of history.
    short: Vec<bool>,
    status: Status,
    first_day: Option<i32>,
    last_day: Option<i32>,
    verdicts: Vec<Verdict>,
}

/// Screens `securities` on `options`: each security measured on its own history up to the date
/// measured to, judged on the rules, and the matches sorted and limited. `benchmark` is the
/// position of the security measures against a benchmark use, and `risk_free_rate` the annual
/// rate of the Sharpe and Sortino ratios. Every problem with the options is reported together.
///
/// With the `parallel` feature the securities are measured on several threads, with the same
/// results on any number.
pub fn screen(
    securities: &Securities,
    benchmark: Option<usize>,
    risk_free_rate: f64,
    options: &ScreenOptions,
) -> Result<ScreenResults> {
    let plan = plan(securities, benchmark, risk_free_rate, options)?;
    let as_of = plan.as_of.days_since_epoch();
    let mut starts = [0i32; 8];
    for (slot, period) in starts.iter_mut().zip(Period::ALL) {
        let back = |months: i32| plan.as_of.add_months(-months).unwrap_or(Date::MIN);
        *slot = match period {
            Period::Week => plan.as_of.add_days(-7).unwrap_or(Date::MIN),
            Period::Month => back(1),
            Period::Months3 => back(3),
            Period::Months6 => back(6),
            Period::YearToDate => {
                Date::from_ymd(plan.as_of.year() - 1, 12, 31).unwrap_or(Date::MIN)
            }
            Period::Year => back(12),
            Period::Years3 => back(36),
            Period::Years5 => back(60),
        }
        .days_since_epoch();
    }
    let max_carry_days = i32::try_from(options.max_carry_days).unwrap_or(i32::MAX);
    let mut warnings = securities.warnings().clone();
    let any_benchmark_measure = plan.measures.iter().any(|m| m.needs_benchmark());
    let benchmark_prices = benchmark.filter(|_| any_benchmark_measure).and_then(|b| {
        let prices = securities.prices(b);
        let n = prices.days.partition_point(|&d| d <= as_of);
        let last = n.checked_sub(1).map(|k| prices.days[k]);
        match last {
            Some(last) if as_of - last <= max_carry_days => {
                Some(measures::Benchmark::new(prices, &starts, as_of))
            }
            _ => {
                warnings.warning(
                    "benchmark",
                    format!(
                        "{} has no close within {} days before {}, so no measure against it has a value",
                        securities.ids[b], options.max_carry_days, plan.as_of
                    ),
                );
                None
            }
        }
    });
    let setting = Setting {
        as_of,
        max_carry_days,
        risk_free: (1.0 + risk_free_rate).powf(1.0 / DAYS_A_YEAR) - 1.0,
        starts,
        measures: &plan.measures,
        benchmark: benchmark_prices,
    };

    let indices: Vec<usize> = (0..securities.len()).collect();
    let rows: Vec<Row> = finalytics_core::parallel::map(&indices, |&i| {
        let prices = securities.prices(i);
        let measured = measures::measure(&setting, prices);
        let status = if measured.stale {
            Status::Stale
        } else if measured.observations < 2
            || plan
                .needed
                .iter()
                .any(|m| !measures::covers(*m, &prices, measured.observations, &starts))
        {
            Status::TooShort
        } else {
            Status::Measured
        };
        // The formulas, in order, on the measures, the fields and the formulas before each.
        let mut formula_values: Vec<Option<f64>> = Vec::with_capacity(plan.formulas.len());
        for compiled in &plan.formulas {
            let inputs: Vec<Option<f64>> = compiled
                .inputs
                .iter()
                .map(|input| match *input {
                    Input::Measure(k) => measured.values[k],
                    Input::Field(k) => securities.fields[k].number(i),
                    Input::Formula(k) => formula_values[k],
                })
                .collect();
            let value = if measured.stale {
                None
            } else {
                compiled.expr.eval(&inputs)
            };
            formula_values.push(value);
        }
        let short = plan
            .measures
            .iter()
            .zip(&measured.values)
            .map(|(m, v)| {
                v.is_none() && !measures::covers(*m, &prices, measured.observations, &starts)
            })
            .collect();
        let verdicts = plan
            .rules
            .iter()
            .map(|rule| {
                if status == Status::Stale {
                    return Verdict::Fail;
                }
                let verdict = |ok: bool| if ok { Verdict::Pass } else { Verdict::Fail };
                match &rule.test {
                    Test::Text(inside, values) => {
                        let Target::Field(k) = rule.target else {
                            return Verdict::Lacking;
                        };
                        match securities.fields[k].text(i) {
                            Some(t) => verdict(values.iter().any(|v| v == t) == *inside),
                            None => Verdict::Lacking,
                        }
                    }
                    test => {
                        let value = match rule.target {
                            Target::Measure(k) => measured.values[k],
                            Target::Field(k) => securities.fields[k].number(i),
                            Target::Formula(k) => formula_values[k],
                        };
                        let Some(x) = value else {
                            return Verdict::Lacking;
                        };
                        verdict(match test {
                            Test::Compare(Op::Below, v) => x < *v,
                            Test::Compare(Op::AtMost, v) => x <= *v,
                            Test::Compare(Op::Above, v) => x > *v,
                            Test::Compare(Op::AtLeast, v) => x >= *v,
                            Test::Range(low, high) => x >= *low && x <= *high,
                            _ => false,
                        })
                    }
                }
            })
            .collect();
        Row {
            values: measured.values,
            formulas: formula_values,
            short,
            status,
            first_day: measured.first_day,
            last_day: measured.last_day,
            verdicts,
        }
    });

    // The matches: every security that is not stale and meets every rule, sorted.
    let passed: Vec<bool> = rows
        .iter()
        .map(|r| r.status != Status::Stale && r.verdicts.iter().all(|v| *v == Verdict::Pass))
        .collect();
    let mut order: Vec<usize> = (0..rows.len()).filter(|i| passed[*i]).collect();
    if let Some((target, direction)) = plan.sort {
        let key = |i: usize| -> Option<f64> {
            match target {
                Target::Measure(k) => rows[i].values[k],
                Target::Field(k) => securities.fields[k].number(i),
                Target::Formula(k) => rows[i].formulas[k],
            }
        };
        order.sort_by(|a, b| match (key(*a), key(*b)) {
            (Some(x), Some(y)) => match direction {
                Order::Ascending => x.total_cmp(&y),
                Order::Descending => y.total_cmp(&x),
            },
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }
    if let Some(limit) = options.limit {
        order.truncate(limit);
    }
    let mut rank = vec![None; rows.len()];
    for (k, i) in order.iter().enumerate() {
        rank[*i] = Some(k + 1);
    }

    let mut lacking = vec![0usize; plan.measures.len()];
    let mut formulas_lacking = vec![0usize; plan.formulas.len()];
    let mut lacking_history = vec![0usize; plan.measures.len()];
    let mut rule_results: Vec<RuleResult> = options
        .rules
        .iter()
        .map(|rule| RuleResult {
            rule: rule.clone(),
            passed: 0,
            lacking: 0,
            remaining: 0,
        })
        .collect();
    let (mut measured, mut too_short, mut stale) = (0, 0, 0);
    for row in &rows {
        match row.status {
            Status::Stale => {
                stale += 1;
                continue;
            }
            Status::TooShort => too_short += 1,
            Status::Measured => measured += 1,
        }
        for (count, v) in lacking.iter_mut().zip(&row.values) {
            if v.is_none() {
                *count += 1;
            }
        }
        for (count, v) in formulas_lacking.iter_mut().zip(&row.formulas) {
            if v.is_none() {
                *count += 1;
            }
        }
        for (count, short) in lacking_history.iter_mut().zip(&row.short) {
            if *short {
                *count += 1;
            }
        }
        let mut still = true;
        for (result, verdict) in rule_results.iter_mut().zip(&row.verdicts) {
            match verdict {
                Verdict::Pass => result.passed += 1,
                Verdict::Lacking => result.lacking += 1,
                Verdict::Fail => {}
            }
            still &= *verdict == Verdict::Pass;
            if still {
                result.remaining += 1;
            }
        }
    }
    if stale == rows.len() && !rows.is_empty() {
        warnings.warning(
            "as_of",
            format!(
                "every security is stale on {}: none has a close within {} days before it",
                plan.as_of, options.max_carry_days
            ),
        );
    }
    let sparse: Vec<&str> = (0..securities.len())
        .filter(|&i| {
            let days = securities.prices(i).days;
            days.len() > 2
                && f64::from(days[days.len() - 1] - days[0]) / (days.len() - 1) as f64 > 4.0
        })
        .map(|i| securities.ids[i].as_str())
        .collect();
    if !sparse.is_empty() {
        warnings.warning(
            "series",
            format!(
                "{} securities have fewer closes than daily prices would, such as {}: the \
                 screen annualises daily returns with {} days a year",
                sparse.len(),
                list(&sparse[..sparse.len().min(3)]),
                DAYS_A_YEAR
            ),
        );
    }

    let day = |d: Option<i32>| d.and_then(|d| Date::from_days_since_epoch(d).ok());
    let formula_names: Vec<&str> = options.formulas.0.iter().map(|(n, _)| n.as_str()).collect();
    let securities_rows: Vec<SecurityScreen> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let mut fields = securities.fields(i);
            for (name, value) in formula_names.iter().zip(&row.formulas) {
                if let Some(x) = value {
                    fields.insert((*name).to_owned(), FieldValue::Number(*x));
                }
            }
            SecurityScreen {
                id: securities.ids[i].clone(),
                status: row.status,
                passed: passed[i],
                matched: rank[i].is_some(),
                rank: rank[i],
                first_date: day(row.first_day),
                last_date: day(row.last_day),
                values: row.values,
                fields,
            }
        })
        .collect();
    let coverage = Coverage {
        securities: securities.len(),
        measured,
        too_short,
        stale,
        passed: passed.iter().filter(|p| **p).count(),
        matched: order.len(),
        lacking,
        lacking_history,
    };
    Ok(ScreenResults {
        as_of: plan.as_of,
        max_carry_days: options.max_carry_days,
        benchmark: benchmark.map(|b| securities.ids[b].clone()),
        risk_free_rate,
        measures: plan.measures,
        fields: securities
            .field_names()
            .into_iter()
            .chain(formula_names.iter().copied())
            .map(str::to_owned)
            .collect(),
        formulas: options
            .formulas
            .0
            .iter()
            .zip(formulas_lacking)
            .map(|((name, formula), lacking)| FormulaResult {
                name: name.clone(),
                formula: formula.clone(),
                lacking,
            })
            .collect(),
        rules: rule_results,
        sort: options.sort.clone(),
        limit: options.limit,
        matches: order.iter().map(|i| securities.ids[*i].clone()).collect(),
        coverage,
        securities: securities_rows,
        warnings,
    })
}
