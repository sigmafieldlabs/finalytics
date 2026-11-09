//! The attribution report: Brinson–Fachler attribution of the portfolio's return against a
//! benchmark's segments, with the portfolio's segments measured from its books.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Result, ValidationReport};
use finalytics_ledger::{Books, MarketData};
use finalytics_perf::{
    Attribution, AttributionPeriod, Linking, Performance, Segment, brinson_fachler,
};

use crate::context::{
    Context, Provenance, ReportKind, ReportOptions, Source, group_of, invalid, to_f64,
};

/// One segment of the benchmark over one period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BenchmarkSegment {
    /// The segment: a classification of the scheme, such as `Equity`.
    pub name: String,
    /// The benchmark's weight in the segment at the start of the period.
    pub benchmark_weight: f64,
    /// The benchmark's return in the segment over the period.
    pub benchmark_return: f64,
}

/// The benchmark's segments over one period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BenchmarkPeriod {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// The segments, with weights adding up to one.
    pub segments: Vec<BenchmarkSegment>,
}

/// What the attribution report compares the portfolio with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AttributionInputs {
    /// The instrument classification scheme that defines the segments, such as `asset_class`.
    /// Cash is the segment "cash", and instruments without the classification "unclassified".
    pub scheme: String,
    /// The benchmark's segments in each period, such as each month. Without them, they are
    /// derived from a blended benchmark: one period for each calendar month of the report, each
    /// segment's weight the sum of its holdings' weights and its return their weighted return
    /// over the month, by the classification `scheme` of the portfolio's instruments. A period
    /// that reaches past the report's start or date is clipped to it, with a warning, and the
    /// benchmark's figures are used as given; one wholly outside the report is left out.
    #[serde(default)]
    pub periods: Vec<BenchmarkPeriod>,
    /// How the periods are linked: Cariño's method by default.
    #[serde(default)]
    pub linking: Linking,
}

/// The attribution report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AttributionReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The classification scheme of the segments.
    pub scheme: String,
    /// Each period's segments: the portfolio's weights and returns from its books, beside the
    /// benchmark's.
    pub periods: Vec<AttributionPeriod>,
    /// The attribution, by period and linked: the totals for a waterfall chart, and each
    /// period's effects for a chart over time.
    pub attribution: Attribution,
    /// The portfolio's time-weighted return over the same span, for comparison with the
    /// attribution's portfolio return (`sum w r` in each period, linked), which leaves out
    /// trading within each period.
    pub time_weighted: Option<f64>,
}

/// Brinson–Fachler attribution of the portfolio's return against the benchmark segments in the
/// options. Each segment's weight is its share of the portfolio's value at the start of the
/// period, and its return links its holdings' daily gains, as a holding's own return does. A
/// segment the portfolio does not hold takes the benchmark's return (so it has no selection
/// effect), and a segment the benchmark does not have takes the portfolio's (so it has only an
/// allocation effect).
pub fn attribution_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<AttributionReport> {
    let chosen = crate::context::chosen_books(books, options)?;
    let books: &Books = &chosen;
    let Some(inputs) = &options.attribution else {
        return Err(invalid(
            "options.attribution",
            "the attribution report needs the benchmark's segments",
        ));
    };
    let cx = Context::new(books, market, source, options)?;
    let derived;
    let inputs = if inputs.periods.is_empty() {
        let Some(crate::context::ReportBenchmark::Blend(weights)) = &options.benchmark else {
            return Err(invalid(
                "options.attribution.periods",
                "give the benchmark's segments in each period, or a blended benchmark to derive \
                 them from",
            ));
        };
        derived = AttributionInputs {
            scheme: inputs.scheme.clone(),
            periods: derived_periods(&cx, weights, &inputs.scheme)?,
            linking: inputs.linking,
        };
        &derived
    } else {
        inputs
    };
    // Periods that reach past the report are clipped to it, and the report says so; the
    // benchmark's figures are used as given. Periods wholly outside it are left out.
    let first = cx.dates[0];
    let mut clipped: Vec<(String, String)> = Vec::new();
    let mut kept: Vec<BenchmarkPeriod> = Vec::with_capacity(inputs.periods.len());
    for (k, p) in inputs.periods.iter().enumerate() {
        let path = format!("options.attribution.periods[{k}]");
        if p.end <= first || p.start >= cx.as_of {
            clipped.push((
                path,
                format!(
                    "{} to {} is outside the report, {first} to {}, and is left out",
                    p.start, p.end, cx.as_of
                ),
            ));
            continue;
        }
        let mut period = p.clone();
        if p.start < first {
            period.start = first;
        }
        if p.end > cx.as_of {
            period.end = cx.as_of;
        }
        if period.start != p.start || period.end != p.end {
            clipped.push((
                path,
                format!(
                    "{} to {} reaches past the report, {first} to {}: attributed from {} to {}, \
                     with the benchmark's figures as given",
                    p.start, p.end, cx.as_of, period.start, period.end
                ),
            ));
        }
        kept.push(period);
    }
    if kept.is_empty() {
        return Err(invalid(
            "options.attribution.periods",
            format!("no period falls within the report, {first} to {}", cx.as_of),
        ));
    }
    let inputs = &AttributionInputs {
        scheme: inputs.scheme.clone(),
        periods: kept,
        linking: inputs.linking,
    };
    let mut report = ValidationReport::new();
    let holdings = books.holding_series(&cx.dates, market)?;
    let mut warnings = holdings.warnings.clone();
    for (path, message) in clipped {
        warnings.warning(path, message);
    }
    let index = |date: Date| cx.dates.partition_point(|&d| d <= date).saturating_sub(1);
    let segment_of: Vec<String> = holdings
        .lines
        .iter()
        .map(|l| group_of(books, &l.holding, &inputs.scheme))
        .collect();

    let mut periods = Vec::with_capacity(inputs.periods.len());
    for (k, p) in inputs.periods.iter().enumerate() {
        let (i, j) = (index(p.start), index(p.end));
        // Each segment's value at the start, and its growth over the period.
        let mut start_values: BTreeMap<&str, f64> = BTreeMap::new();
        let mut growth: BTreeMap<&str, f64> = BTreeMap::new();
        for (line, segment) in holdings.lines.iter().zip(&segment_of) {
            *start_values.entry(segment).or_default() += to_f64(line.values_base[i]);
            growth.entry(segment).or_insert(1.0);
        }
        let total: f64 = start_values.values().sum();
        if total.is_nan() || total <= 0.0 {
            report.error(
                format!("options.attribution.periods[{k}]"),
                format!("the portfolio is worth nothing on {}", cx.dates[i]),
            );
            continue;
        }
        for t in i + 1..=j {
            let mut gains: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
            for (line, segment) in holdings.lines.iter().zip(&segment_of) {
                // Fees and taxes charged to the accounts are not a loss on the cash that pays
                // them: segments' returns are before them.
                let gain = to_f64(line.values_base[t])
                    - to_f64(line.values_base[t - 1])
                    - to_f64(line.trades_base[t])
                    + to_f64(line.income_base[t])
                    + to_f64(line.expenses_base[t]);
                let base = to_f64(line.values_base[t - 1]) + to_f64(line.trades_base[t]).max(0.0);
                let e = gains.entry(segment).or_default();
                e.0 += gain;
                e.1 += base;
            }
            for (segment, (gain, base)) in gains {
                let g = growth.entry(segment).or_insert(1.0);
                if base > 0.0 {
                    *g *= 1.0 + gain / base;
                } else if gain.abs() > 1e-9 {
                    warnings.warning(
                        format!("options.attribution.periods[{k}]"),
                        format!(
                            "{segment} gained {gain} on {} with nothing invested; its return \
                             leaves that day out",
                            cx.dates[t]
                        ),
                    );
                }
            }
        }
        let mut segments: Vec<Segment> = Vec::new();
        for b in &p.segments {
            let w = start_values.get(b.name.as_str()).copied().unwrap_or(0.0) / total;
            let r = match growth.get(b.name.as_str()) {
                Some(g) if w != 0.0 => g - 1.0,
                _ => b.benchmark_return,
            };
            segments.push(Segment {
                name: b.name.clone(),
                portfolio_weight: w,
                portfolio_return: r,
                benchmark_weight: b.benchmark_weight,
                benchmark_return: b.benchmark_return,
            });
        }
        for (name, value) in &start_values {
            if *value != 0.0 && !p.segments.iter().any(|b| b.name == *name) {
                let r = growth.get(name).map_or(0.0, |g| g - 1.0);
                segments.push(Segment {
                    name: (*name).to_owned(),
                    portfolio_weight: value / total,
                    portfolio_return: r,
                    benchmark_weight: 0.0,
                    benchmark_return: r,
                });
            }
        }
        periods.push(AttributionPeriod {
            start: p.start,
            end: p.end,
            segments,
        });
    }
    report.into_result(())?;
    let attribution = brinson_fachler(&periods, inputs.linking).map_err(|e| match e {
        finalytics_core::Error::Validation(r) => {
            let mut prefixed = ValidationReport::new();
            prefixed.merge("options.attribution", r);
            finalytics_core::Error::Validation(prefixed)
        }
        other => other,
    })?;
    let time_weighted = match (inputs.periods.first(), inputs.periods.last()) {
        (Some(a), Some(b)) => books
            .value_series(&cx.dates, market)
            .ok()
            .and_then(|v| Performance::from_values(&v, options.timing).ok())
            .and_then(|p| p.time_weighted(a.start, b.end)),
        _ => None,
    };
    Ok(AttributionReport {
        provenance: cx.provenance(ReportKind::Attribution, warnings),
        scheme: inputs.scheme.clone(),
        periods,
        attribution,
        time_weighted,
    })
}

/// The segments of the blended benchmark `weights` in each calendar month of the report's dates:
/// the blend is rebalanced to its weights at the start of each month, so a segment's weight is
/// the sum of its holdings' and its return their weighted return in the base currency.
fn derived_periods(
    cx: &Context<'_>,
    weights: &BTreeMap<finalytics_ledger::HoldingKey, f64>,
    scheme: &str,
) -> Result<Vec<BenchmarkPeriod>> {
    let dates = &cx.dates;
    // The ends of the periods: the last valuation date of each month, and the report's date.
    let mut ends: Vec<usize> = (0..dates.len().saturating_sub(1))
        .filter(|&t| {
            (dates[t + 1].year(), dates[t + 1].month()) != (dates[t].year(), dates[t].month())
        })
        .filter(|&t| t > 0)
        .collect();
    if dates.len() > 1 {
        ends.push(dates.len() - 1);
    }
    let value = |key: &finalytics_ledger::HoldingKey, t: usize| -> Result<f64> {
        match crate::context::unit_value(cx.books, cx.market, key, dates[t], cx.base) {
            crate::context::UnitValue::Value(v) => Ok(v),
            _ => Err(invalid(
                "options.benchmark",
                format!(
                    "{key} has no price or rate on {}, so the benchmark's segments cannot be \
                     derived",
                    dates[t]
                ),
            )),
        }
    };
    let mut periods = Vec::with_capacity(ends.len());
    let mut start = 0;
    for end in ends {
        let mut segments: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for (key, w) in weights {
            let r = value(key, end)? / value(key, start)? - 1.0;
            let e = segments.entry(group_of(cx.books, key, scheme)).or_default();
            e.0 += w;
            e.1 += w * r;
        }
        periods.push(BenchmarkPeriod {
            start: dates[start],
            end: dates[end],
            segments: segments
                .into_iter()
                .filter(|(_, (w, _))| *w > 0.0)
                .map(|(name, (w, wr))| BenchmarkSegment {
                    name,
                    benchmark_weight: w,
                    benchmark_return: wr / w,
                })
                .collect(),
        });
        start = end;
    }
    Ok(periods)
}
