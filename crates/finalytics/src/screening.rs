//! Screening: measures and rules on many securities, each on its own history.
//!
//! A [`Universe`] holds many securities, each with its own dates: a close on each, and its
//! volumes, dividends and fields where known. [`Universe::screen`] measures every security up
//! to the date measured to, with no alignment between them: trailing returns, risk, measures
//! against a benchmark, technical indicators, income, volume and the state of its data. Rules on
//! the measures and on the fields keep the securities that meet them all, sorted and limited.
//!
//! ```
//! use finalytics::prelude::*;
//! use finalytics::screening::Series;
//!
//! let dates: Vec<Date> = (0..400)
//!     .map(|k| Date::from_ymd(2025, 6, 2).and_then(|d| d.add_days(k)))
//!     .collect::<Result<_>>()?;
//! let rising: Vec<f64> = (0..400).map(|k| 100.0 * 1.001f64.powi(k)).collect();
//! let falling: Vec<f64> = (0..400).map(|k| 100.0 * 0.999f64.powi(k)).collect();
//! let universe = Universe::from_series(vec![
//!     Series::new("UP", dates.clone(), rising).with_field("sector", "Energy"),
//!     Series::new("DOWN", dates, falling).with_field("sector", "Utilities"),
//! ])?
//! .with_risk_free_rate(0.04);
//!
//! let screen = universe
//!     .screen()
//!     .above("return_1y", 0.0)
//!     .one_of("sector", ["Energy", "Materials"])
//!     .sort("sharpe_ratio", Order::Descending)
//!     .build()?;
//! assert_eq!(screen.results.matches, ["UP"]);
//! # Ok::<(), finalytics::Error>(())
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, FieldCells, closes_from_csv, fields_from_csv};
use finalytics_risk::screening::Securities;

pub use finalytics_reports::{
    MeasureSpread, ScreenColumn, ScreenContext, ScreenMatch, ScreenReport,
};
pub use finalytics_risk::screening::{
    Columns, Coverage, FieldValue, FormulaResult, Formulas, Measure, MeasureUnit, Op, Order, Rule,
    RuleResult, RuleValue, ScreenOptions, ScreenResults, SecurityScreen, Series, Sort, Status,
};

/// Many securities, each with its own dates and closes, and their volumes, dividends and fields
/// where known, with the benchmark and the risk-free rate a screen measures them against.
///
/// Build one from series ([`Universe::from_series`]), from CSV ([`Universe::from_csv`]) or step
/// by step ([`Universe::builder`]); [`Universe::screen`] screens it.
#[derive(Debug, Clone, PartialEq)]
pub struct Universe {
    securities: Securities,
    benchmark: Option<usize>,
    risk_free_rate: f64,
}

/// A screen: the screening report and the raw results. Printing it prints the report, and
/// showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Screen {
    /// The screening report: how the rules narrow the universe, the matches in order, the
    /// securities that fail just one rule, where each threshold falls and every measure.
    pub report: ScreenReport,
    /// The raw results: one row per security, with each measure's value, its fields and
    /// whether it matched.
    pub results: ScreenResults,
}

impl std::fmt::Display for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl Universe {
    /// A builder for a universe from its prices and fields, with a benchmark and a risk-free
    /// rate.
    pub fn builder() -> UniverseBuilder {
        UniverseBuilder::default()
    }

    /// The securities of `series`, in their order, checked: identifiers given once each,
    /// increasing dates and closes above zero. A close that is not a number is no close: its
    /// date is left out, with a warning.
    pub fn from_series(series: Vec<Series>) -> Result<Universe> {
        Ok(Universe::of(Securities::from_series(series)?))
    }

    /// The securities of prices in CSV text, in either layout: the long layout of `id`, `date`
    /// and `close` columns, with `volume` and `dividends` if known, one row per security and
    /// date in any order; or a `date` column and one column of closes per security, empty where
    /// it has none. The securities keep the order in which the file first names them.
    pub fn from_csv(text: &str) -> Result<Universe> {
        let series = closes_from_csv(text, CsvOptions::default())?
            .into_iter()
            .map(|c| {
                let mut s = Series::new(c.id, c.dates, c.close);
                s.volume = c.volume;
                s.dividends = c.dividends;
                s
            })
            .collect();
        Universe::from_series(series)
    }

    /// The securities of flat columns: every security's rows one after another, with the row
    /// each starts at. The bindings pass large universes this way.
    #[doc(hidden)]
    pub fn from_columns(columns: Columns) -> Result<Universe> {
        Ok(Universe::of(Securities::from_columns(columns)?))
    }

    fn of(securities: Securities) -> Universe {
        Universe {
            securities,
            benchmark: None,
            risk_free_rate: 0.0,
        }
    }

    /// The same universe with one of its securities, such as an index, as the benchmark of
    /// beta, correlation and relative returns.
    pub fn with_benchmark(mut self, id: &str) -> Result<Universe> {
        match self.securities.position(id) {
            Some(i) => {
                self.benchmark = Some(i);
                Ok(self)
            }
            None => {
                let mut report = ValidationReport::new();
                report.error(
                    "benchmark",
                    format!("{id} is not one of the universe's securities"),
                );
                Err(Error::Validation(report))
            }
        }
    }

    /// The same universe with an annual risk-free rate, such as 0.04, for the Sharpe and
    /// Sortino ratios; zero by default.
    pub fn with_risk_free_rate(mut self, rate: f64) -> Universe {
        self.risk_free_rate = rate;
        self
    }

    /// The same universe with fields added, by security and then by name, such as
    /// `{"AAPL": {"sector": "Technology", "pe_ratio": 31.2}}`. A field is numbers or text for
    /// every security. A security the universe does not hold is left out, with a warning.
    pub fn with_fields(
        mut self,
        fields: BTreeMap<String, BTreeMap<String, FieldValue>>,
    ) -> Result<Universe> {
        self.securities = self.securities.with_fields(fields)?;
        Ok(self)
    }

    /// The same universe with fields from CSV text: an `id` column and one column per field,
    /// such as `id,sector,pe_ratio`, in the order of the columns. A column whose every value is a number holds numbers; any
    /// other holds text, and an empty cell is no value.
    pub fn with_fields_csv(self, text: &str) -> Result<Universe> {
        let table = fields_from_csv(text, CsvOptions::default()).map_err(|e| under("fields", e))?;
        let order: Vec<String> = table.fields.iter().map(|(name, _)| name.clone()).collect();
        let mut fields: BTreeMap<String, BTreeMap<String, FieldValue>> = BTreeMap::new();
        for (name, cells) in table.fields {
            for (k, id) in table.ids.iter().enumerate() {
                let value = match &cells {
                    FieldCells::Numbers(v) => v[k].map(FieldValue::Number),
                    FieldCells::Text(v) => v[k].clone().map(FieldValue::Text),
                    _ => None,
                };
                if let Some(value) = value {
                    fields
                        .entry(id.clone())
                        .or_default()
                        .insert(name.clone(), value);
                }
            }
        }
        let mut universe = self.with_fields(fields)?;
        universe.securities = universe.securities.with_field_order(&order);
        Ok(universe)
    }

    /// The same universe with the fields named in `names` in that order, after any others: the
    /// bindings keep the order fields are given in.
    #[doc(hidden)]
    pub fn with_field_order<S: AsRef<str>>(mut self, names: &[S]) -> Universe {
        self.securities = self.securities.with_field_order(names);
        self
    }

    /// The securities' identifiers, in order.
    pub fn ids(&self) -> &[String] {
        self.securities.ids()
    }

    /// The number of securities.
    pub fn len(&self) -> usize {
        self.securities.len()
    }

    /// Whether the universe holds no securities.
    pub fn is_empty(&self) -> bool {
        self.securities.is_empty()
    }

    /// The benchmark, if one is set.
    pub fn benchmark(&self) -> Option<&str> {
        self.benchmark.map(|i| self.securities.ids()[i].as_str())
    }

    /// The annual risk-free rate.
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// The names of the fields given, in the order they were first given.
    pub fn fields(&self) -> Vec<&str> {
        self.securities.field_names()
    }

    /// The latest date of any security's closes: the date a screen measures to by default.
    pub fn latest_date(&self) -> Option<Date> {
        self.securities.latest()
    }

    /// The security `id`'s dates and closes, with its fields.
    pub fn series(&self, id: &str) -> Option<Series> {
        let i = self.securities.position(id)?;
        let mut series = Series::new(
            id,
            self.securities.dates(i),
            self.securities.close(i).to_vec(),
        );
        series.fields = self.securities.fields(i);
        Some(series)
    }

    /// Problems met reading the securities that did not stop the universe being built, such as
    /// closes that are not numbers.
    pub fn warnings(&self) -> &ValidationReport {
        self.securities.warnings()
    }

    /// A screen of the universe, built step by step: the date measured to, the measures, the
    /// rules, the sort and the limit; [`ScreenBuilder::build`] runs it.
    pub fn screen(&self) -> ScreenBuilder<'_> {
        ScreenBuilder {
            universe: self,
            options: ScreenOptions::default(),
            problems: ValidationReport::new(),
            name: None,
        }
    }

    /// The screen of `options`: every security measured on its own history up to the date
    /// measured to, judged on the rules, with the matches sorted and limited; the screening
    /// report, with the raw results. [`Universe::screen`] builds the same options step by step.
    /// Every problem with the options is reported together.
    pub fn screen_with(&self, options: &ScreenOptions) -> Result<Screen> {
        let results = finalytics_risk::screening::screen(
            &self.securities,
            self.benchmark,
            self.risk_free_rate,
            options,
        )?;
        let report = finalytics_reports::screen_report(&results, crate::VERSION);
        Ok(Screen { report, results })
    }
}

/// A screen of a [`Universe`], built step by step; [`ScreenBuilder::build`] runs it. Measures
/// and fields are named as text, such as `"return_1y"` or `"sector"`: a name that is not a
/// measure is a field, and one that names neither is reported with the screen's other problems
/// when it is built.
#[derive(Debug, Clone)]
#[must_use = "a screen does nothing until `build` runs it"]
pub struct ScreenBuilder<'a> {
    universe: &'a Universe,
    options: ScreenOptions,
    problems: ValidationReport,
    name: Option<String>,
}

impl ScreenBuilder<'_> {
    /// The date measured to, such as `"2026-10-30"`: the universe's latest date by default.
    pub fn as_of(mut self, date: &str) -> Self {
        match date.parse() {
            Ok(d) => self.options.as_of = Some(d),
            Err(e) => {
                let message = match e {
                    Error::InvalidDate(m) => m,
                    other => other.to_string(),
                };
                self.problems.error("as_of", message);
            }
        }
        self
    }

    /// The most days a security's last close may be before the date measured to; an older one
    /// is stale: listed, but not measured. 5 by default.
    pub fn max_carry_days(mut self, days: u32) -> Self {
        self.options.max_carry_days = days;
        self
    }

    /// The measures to compute, such as `["return_1y", "volatility", "rsi_14"]`: every measure
    /// the universe allows by default. The measures the rules and the sort use are computed
    /// too.
    pub fn measures<S: AsRef<str>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.options.measures = Some(names.into_iter().map(|n| n.as_ref().to_owned()).collect());
        self
    }

    /// A rule: `name`, a measure or a numeric field, above `value`.
    pub fn above(self, name: &str, value: f64) -> Self {
        self.rule(Rule::above(name, value))
    }

    /// A rule: `name` at least `value`.
    pub fn at_least(self, name: &str, value: f64) -> Self {
        self.rule(Rule::at_least(name, value))
    }

    /// A rule: `name` below `value`.
    pub fn below(self, name: &str, value: f64) -> Self {
        self.rule(Rule::below(name, value))
    }

    /// A rule: `name` at most `value`.
    pub fn at_most(self, name: &str, value: f64) -> Self {
        self.rule(Rule::at_most(name, value))
    }

    /// A rule: `name` from `low` to `high`, both included.
    pub fn between(self, name: &str, low: f64, high: f64) -> Self {
        self.rule(Rule::between(name, low, high))
    }

    /// A rule: the text field `name` one of `values`.
    pub fn one_of<S: Into<String>>(self, name: &str, values: impl IntoIterator<Item = S>) -> Self {
        self.rule(Rule::one_of(name, values))
    }

    /// A rule: the text field `name` none of `values`.
    pub fn none_of<S: Into<String>>(self, name: &str, values: impl IntoIterator<Item = S>) -> Self {
        self.rule(Rule::none_of(name, values))
    }

    /// A [`Rule`].
    pub fn rule(mut self, rule: Rule) -> Self {
        self.options.rules.push(rule);
        self
    }

    /// The matches sorted by `name`, a measure or a numeric field, the largest first
    /// ([`Order::Descending`]) or the smallest; in the universe's order by default.
    pub fn sort(mut self, name: &str, order: Order) -> Self {
        self.options.sort = Some(Sort::by(name, order));
        self
    }

    /// The most matches to keep, after sorting.
    pub fn limit(mut self, limit: usize) -> Self {
        self.options.limit = Some(limit);
        self
    }

    /// Screens the universe's benchmark as a candidate too; by default it serves the measures
    /// against a benchmark, such as beta, and is not listed.
    pub fn include_benchmark(mut self) -> Self {
        self.options.include_benchmark = true;
        self
    }

    /// A formula, such as `formula("risk_adjusted", "return_1y / volatility")`: a numeric field
    /// of every security, computed from the measures, the numeric fields and the formulas
    /// before it, which the rules, the sort and the results then take by its name. A formula
    /// given again under the same name replaces the first.
    pub fn formula(mut self, name: &str, formula: &str) -> Self {
        let formulas = &mut self.options.formulas.0;
        match formulas.iter_mut().find(|(n, _)| n == name) {
            Some((_, f)) => *f = formula.to_owned(),
            None => formulas.push((name.to_owned(), formula.to_owned())),
        }
        self
    }

    /// The report's title, such as `"Quality at a fair price"`.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// The options as built, with the problems found so far.
    pub fn options(&self) -> Result<ScreenOptions> {
        self.problems.clone().into_result(self.options.clone())
    }

    /// Runs the screen, reporting every problem with it together.
    pub fn build(&self) -> Result<Screen> {
        let screen = self.universe.screen_with(&self.options);
        if self.problems.is_empty() {
            let mut screen = screen?;
            screen.report.name.clone_from(&self.name);
            return Ok(screen);
        }
        let mut problems = self.problems.clone();
        if let Err(Error::Validation(more)) = screen {
            problems.merge("", more);
        }
        Err(Error::Validation(problems))
    }
}

/// [`Universe`] built step by step from prices and fields; [`UniverseBuilder::build`] makes it.
///
/// ```
/// use finalytics::prelude::*;
///
/// let universe = Universe::builder()
///     .prices_csv("id,date,close\nA,2026-10-29,10\nA,2026-10-30,11\nB,2026-10-30,20\n")
///     .fields_csv("id,sector\nA,Energy\nB,Utilities\n")
///     .benchmark("B")
///     .risk_free_rate(0.04)
///     .build()?;
/// assert_eq!(universe.ids(), ["A", "B"]);
/// assert_eq!(universe.fields(), ["sector"]);
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "the universe is made by `build`"]
pub struct UniverseBuilder {
    series: Option<Vec<Series>>,
    csv: Option<String>,
    #[cfg(feature = "io")]
    file: Option<std::path::PathBuf>,
    fields_csv: Option<String>,
    #[cfg(feature = "io")]
    fields_file: Option<std::path::PathBuf>,
    settings: UniverseSettings,
}

impl UniverseBuilder {
    /// The securities' prices as series, each on its own dates.
    pub fn series(mut self, series: Vec<Series>) -> Self {
        self.series = Some(series);
        self
    }

    /// The securities' prices as CSV text, as [`Universe::from_csv`] reads it.
    pub fn prices_csv(mut self, text: &str) -> Self {
        self.csv = Some(text.to_owned());
        self
    }

    /// The securities' prices from a CSV file, as [`Universe::from_csv`] reads it.
    #[cfg(feature = "io")]
    pub fn prices_file(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.file = Some(path.as_ref().to_owned());
        self
    }

    /// Fields as CSV text, as [`Universe::with_fields_csv`] reads it.
    pub fn fields_csv(mut self, text: &str) -> Self {
        self.fields_csv = Some(text.to_owned());
        self
    }

    /// Fields from a CSV file, as [`Universe::with_fields_csv`] reads it.
    #[cfg(feature = "io")]
    pub fn fields_file(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.fields_file = Some(path.as_ref().to_owned());
        self
    }

    /// One of the securities, such as an index, as the benchmark of beta, correlation and
    /// relative returns.
    pub fn benchmark(mut self, id: &str) -> Self {
        self.settings.benchmark = Some(id.to_owned());
        self
    }

    /// An annual risk-free rate, such as 0.04, for the Sharpe and Sortino ratios; zero by
    /// default.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.settings.risk_free_rate = Some(rate);
        self
    }

    /// Makes the universe, reporting every problem with what was given.
    pub fn build(self) -> Result<Universe> {
        let mut report = ValidationReport::new();
        #[cfg(feature = "io")]
        let (file, fields_file) = {
            let mut read = |path: &Option<std::path::PathBuf>, name: &str| match path {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(text) => Some(text),
                    Err(e) => {
                        report.error(name, format!("cannot read {}: {e}", path.display()));
                        None
                    }
                },
                None => None,
            };
            (
                read(&self.file, "prices"),
                read(&self.fields_file, "fields"),
            )
        };
        #[cfg(not(feature = "io"))]
        let (file, fields_file): (Option<String>, Option<String>) = (None, None);
        let given = usize::from(self.series.is_some())
            + usize::from(self.csv.is_some())
            + usize::from(file.is_some());
        if given != 1 && report.is_empty() {
            report.error("prices", "give the securities' prices once");
        }
        if self.fields_csv.is_some() && fields_file.is_some() {
            report.error("fields", "give the fields once");
        }
        self.settings.check(&mut report);
        report.into_result(())?;
        let universe = if let Some(series) = self.series {
            Universe::from_series(series)?
        } else if let Some(text) = self.csv.as_deref().or(file.as_deref()) {
            Universe::from_csv(text)?
        } else {
            unreachable!("one source of prices was checked above")
        };
        let universe = match self.fields_csv.as_deref().or(fields_file.as_deref()) {
            Some(text) => universe.with_fields_csv(text)?,
            None => universe,
        };
        self.settings.apply(universe)
    }
}

/// `error` with each of its paths under `prefix`.
fn under(prefix: &str, error: Error) -> Error {
    match error {
        Error::Validation(found) => {
            let mut report = ValidationReport::new();
            report.merge(prefix, found);
            Error::Validation(report)
        }
        other => other,
    }
}

/// The settings of a [`Universe`] as JSON gives them: the Python and JavaScript packages use
/// it; from Rust, use the builder.
#[doc(hidden)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UniverseSettings {
    pub benchmark: Option<String>,
    pub risk_free_rate: Option<f64>,
}

impl UniverseSettings {
    /// Adds each problem with the settings to `report`.
    pub fn check(&self, report: &mut ValidationReport) {
        if let Some(rate) = self.risk_free_rate
            && !(rate.is_finite() && rate > -1.0)
        {
            report.error(
                "risk_free_rate",
                format!("must be an annual rate above -1, such as 0.04, not {rate}"),
            );
        }
    }

    /// `universe` with these settings, every problem with them reported together.
    pub fn apply(&self, mut universe: Universe) -> Result<Universe> {
        let mut report = ValidationReport::new();
        self.check(&mut report);
        if let Some(id) = &self.benchmark
            && universe.securities.position(id).is_none()
        {
            report.error(
                "benchmark",
                format!("{id} is not one of the universe's securities"),
            );
        }
        report.into_result(())?;
        if let Some(rate) = self.risk_free_rate {
            universe = universe.with_risk_free_rate(rate);
        }
        if let Some(id) = &self.benchmark {
            universe = universe.with_benchmark(id)?;
        }
        Ok(universe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRICES: &str = "id,date,close,volume\n\
        A,2026-10-28,10,100\nA,2026-10-29,10.5,120\nA,2026-10-30,11,90\n\
        B,2026-10-30,20,\nB,2026-10-29,19,\n";

    #[test]
    fn problems_with_a_universe_are_reported_together() {
        let err = Universe::builder()
            .prices_csv(PRICES)
            .series(Vec::new())
            .risk_free_rate(f64::NAN)
            .build()
            .unwrap_err();
        let Error::Validation(report) = err else {
            panic!("not a validation error")
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["prices", "risk_free_rate"]);
        let err = Universe::builder()
            .prices_csv(PRICES)
            .fields_csv("id,rsi_14\nA,1\n")
            .build()
            .unwrap_err();
        let Error::Validation(report) = err else {
            panic!("not a validation error")
        };
        assert_eq!(report.issues()[0].path, "fields.A.rsi_14");
        let err = Universe::builder()
            .prices_csv(PRICES)
            .benchmark("SPY")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("SPY is not one of"));
    }

    #[test]
    fn a_long_csv_keeps_each_securitys_own_dates() {
        let u = Universe::from_csv(PRICES).unwrap();
        assert_eq!(u.ids(), ["A", "B"]);
        let b = u.series("B").unwrap();
        assert_eq!(b.dates.len(), 2);
        assert_eq!(b.close, [19.0, 20.0]);
        assert_eq!(u.latest_date(), Some("2026-10-30".parse().unwrap()));
        let wide = Universe::from_csv("date,A,B\n2026-10-29,1,\n2026-10-30,2,3\n").unwrap();
        assert_eq!(wide.series("B").unwrap().close, [3.0]);
    }

    #[test]
    fn builder_problems_join_the_screens() {
        let u = Universe::from_csv(PRICES).unwrap();
        let err = u
            .screen()
            .as_of("2026-13-01")
            .above("no_such_measure", 1.0)
            .limit(0)
            .build()
            .unwrap_err();
        let Error::Validation(report) = err else {
            panic!("not a validation error")
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["as_of", "rules[0].field", "limit"]);
        let screen = u.screen().name("Two").build().unwrap();
        assert_eq!(screen.report.name.as_deref(), Some("Two"));
        assert!(screen.to_string().contains("FINALYTICS  ·  SCREEN"));
    }
}
