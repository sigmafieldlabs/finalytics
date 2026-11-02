//! Performance attribution: Brinson–Fachler by segment, linked over periods, and currency
//! attribution by the method of Karnosky and Singer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Result, ValidationReport};

/// A segment of a portfolio and of its benchmark over one period: an asset class, a sector, a
/// region or any other grouping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Segment {
    /// The segment's name.
    pub name: String,
    /// The portfolio's weight in the segment at the start of the period.
    pub portfolio_weight: f64,
    /// The return of the portfolio's holdings in the segment over the period.
    pub portfolio_return: f64,
    /// The benchmark's weight in the segment at the start of the period.
    pub benchmark_weight: f64,
    /// The return of the benchmark's holdings in the segment over the period.
    pub benchmark_return: f64,
}

/// The segments over one period. The portfolio weights add up to one, as do the benchmark
/// weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AttributionPeriod {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// The segments.
    pub segments: Vec<Segment>,
}

/// How the effects of several periods are linked so that they add up to the excess return over
/// all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Linking {
    /// Logarithmic coefficients (Cariño, 1999).
    #[default]
    Carino,
    /// Coefficients that spread the difference as evenly as possible (Menchero, 2000).
    Menchero,
}

/// What one segment added to the excess return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct SegmentEffects {
    /// The segment's name.
    pub name: String,
    /// The effect of weighting the segment differently from the benchmark,
    /// `(w - b)(s - B)`.
    pub allocation: f64,
    /// The effect of the portfolio's holdings in the segment returning differently from the
    /// benchmark's, `b (r - s)`.
    pub selection: f64,
    /// The joint effect of the two, `(w - b)(r - s)`.
    pub interaction: f64,
    /// The sum of the three.
    pub total: f64,
}

/// The attribution of one period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PeriodAttribution {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// The portfolio's return, `sum w r`.
    pub portfolio_return: f64,
    /// The benchmark's return, `sum b s`.
    pub benchmark_return: f64,
    /// The coefficient by which the period's effects are multiplied before they are added up
    /// over the periods (one for a single period).
    pub linking_coefficient: f64,
    /// Each segment's effects, in the order given.
    pub segments: Vec<SegmentEffects>,
}

/// Brinson–Fachler attribution over one or more periods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Attribution {
    /// How the periods were linked.
    pub linking: Linking,
    /// The portfolio's return over all the periods, compounded.
    pub portfolio_return: f64,
    /// The benchmark's return over all the periods, compounded.
    pub benchmark_return: f64,
    /// `portfolio_return - benchmark_return`, which the linked effects add up to.
    pub excess_return: f64,
    /// Each segment's effects over all the periods, linked, in the order the segments first
    /// appear.
    pub segments: Vec<SegmentEffects>,
    /// The allocation effects of all the segments, linked.
    pub allocation: f64,
    /// The selection effects of all the segments, linked.
    pub selection: f64,
    /// The interaction effects of all the segments, linked.
    pub interaction: f64,
    /// Each period's own attribution, before linking.
    pub periods: Vec<PeriodAttribution>,
}

/// Brinson–Fachler attribution (Brinson and Fachler, 1985) of a portfolio's return against its
/// benchmark's, by segment, over `periods` linked by `linking`.
///
/// In each period, with portfolio weights `w`, benchmark weights `b`, portfolio segment returns
/// `r` and benchmark segment returns `s`, and `B = sum b s` the benchmark's return:
///
/// - allocation `(w - b)(s - B)`: overweighting segments that beat the benchmark;
/// - selection `b (r - s)`: holdings that beat their segment of the benchmark;
/// - interaction `(w - b)(r - s)`: overweighting the segments where the selection worked.
///
/// The effects of one period add up to `sum w r - B`. Over several periods, each period's effects
/// are scaled by its linking coefficient, so that the linked effects add up to the difference of
/// the compounded returns.
///
/// ```
/// # mod finalytics {
/// #     pub use finalytics_core::*;
/// #     pub mod performance { pub use finalytics_perf::*; }
/// # }
/// use finalytics::performance::{AttributionPeriod, Linking, Segment, brinson_fachler};
///
/// let segment = |name: &str, w: f64, r: f64, b: f64, s: f64| Segment {
///     name: name.into(),
///     portfolio_weight: w,
///     portfolio_return: r,
///     benchmark_weight: b,
///     benchmark_return: s,
/// };
/// let period = AttributionPeriod {
///     start: "2026-01-01".parse()?,
///     end: "2026-03-31".parse()?,
///     segments: vec![
///         segment("equities", 0.7, 0.05, 0.6, 0.04),
///         segment("bonds", 0.3, 0.01, 0.4, 0.015),
///     ],
/// };
/// let attribution = brinson_fachler(&[period], Linking::Carino)?;
/// let effects = attribution.allocation + attribution.selection + attribution.interaction;
/// assert!((effects - attribution.excess_return).abs() < 1e-15);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn brinson_fachler(periods: &[AttributionPeriod], linking: Linking) -> Result<Attribution> {
    let mut report = ValidationReport::new();
    check_periods(
        periods.iter().map(|p| (p.start, p.end, p.segments.len())),
        &mut report,
    );
    for (k, period) in periods.iter().enumerate() {
        check_segments(
            period.segments.iter().map(|s| {
                (
                    s.name.as_str(),
                    s.portfolio_weight,
                    s.portfolio_return,
                    s.benchmark_weight,
                    s.benchmark_return,
                )
            }),
            &format!("periods[{k}].segments"),
            &mut report,
        );
    }
    report.into_result(())?;

    let mut attributed: Vec<PeriodAttribution> = periods
        .iter()
        .map(|p| {
            let (portfolio_return, benchmark_return, segments) =
                effects(p.segments.iter().map(|s| {
                    (
                        s.name.as_str(),
                        s.portfolio_weight,
                        s.portfolio_return,
                        s.benchmark_weight,
                        s.benchmark_return,
                    )
                }));
            PeriodAttribution {
                start: p.start,
                end: p.end,
                portfolio_return,
                benchmark_return,
                linking_coefficient: 1.0,
                segments,
            }
        })
        .collect();
    let returns: Vec<(f64, f64, bool)> = attributed
        .iter()
        .map(|p| {
            let finite = finite(p.segments.iter().flat_map(numbers));
            (p.portfolio_return, p.benchmark_return, finite)
        })
        .collect();
    let (portfolio_return, benchmark_return, coefficients) = link(&returns, linking)?;
    for (p, c) in attributed.iter_mut().zip(&coefficients) {
        p.linking_coefficient = *c;
    }
    let segments = linked_segments(
        attributed.iter().map(|p| p.segments.as_slice()),
        &coefficients,
    );
    let sum = |pick: fn(&SegmentEffects) -> f64| segments.iter().map(pick).sum::<f64>();
    let attribution = Attribution {
        linking,
        portfolio_return,
        benchmark_return,
        excess_return: portfolio_return - benchmark_return,
        allocation: sum(|s| s.allocation),
        selection: sum(|s| s.selection),
        interaction: sum(|s| s.interaction),
        segments,
        periods: attributed,
    };
    let totals = [
        attribution.excess_return,
        attribution.allocation,
        attribution.selection,
        attribution.interaction,
    ];
    linked_finite(
        totals
            .into_iter()
            .chain(attribution.segments.iter().flat_map(numbers)),
    )?;
    Ok(attribution)
}

fn numbers(s: &SegmentEffects) -> [f64; 4] {
    [s.allocation, s.selection, s.interaction, s.total]
}

fn finite(values: impl IntoIterator<Item = f64>) -> bool {
    values.into_iter().all(f64::is_finite)
}

/// An error unless every linked number is finite.
fn linked_finite(values: impl IntoIterator<Item = f64>) -> Result<()> {
    let mut report = ValidationReport::new();
    if !finite(values) {
        report.error("periods", "the linked effects are too large to represent");
    }
    report.into_result(())
}

/// The portfolio's and benchmark's returns and each segment's effects, from
/// `(name, w, r, b, s)` for each segment.
fn effects<'a>(
    segments: impl Iterator<Item = (&'a str, f64, f64, f64, f64)> + Clone,
) -> (f64, f64, Vec<SegmentEffects>) {
    let portfolio: f64 = segments.clone().map(|(_, w, r, _, _)| w * r).sum();
    let benchmark: f64 = segments.clone().map(|(_, _, _, b, s)| b * s).sum();
    let effects = segments
        .map(|(name, w, r, b, s)| {
            let allocation = (w - b) * (s - benchmark);
            let selection = b * (r - s);
            let interaction = (w - b) * (r - s);
            SegmentEffects {
                name: name.to_owned(),
                allocation,
                selection,
                interaction,
                total: allocation + selection + interaction,
            }
        })
        .collect();
    (portfolio, benchmark, effects)
}

/// The compounded portfolio and benchmark returns of periods with returns `(R_t, B_t)` (and
/// whether their effects are finite), and each period's linking coefficient, with which
/// `sum c_t (R_t - B_t) = R - B`.
///
/// Every period's returns and effects must be finite and, to link several periods, every return
/// above -100%: with short positions, the portfolio can lose more than it is worth, and neither
/// linking method is defined for such a period.
pub(crate) fn link(returns: &[(f64, f64, bool)], linking: Linking) -> Result<(f64, f64, Vec<f64>)> {
    let mut report = ValidationReport::new();
    for (k, &(r, b, effects)) in returns.iter().enumerate() {
        if !(r.is_finite() && b.is_finite() && effects) {
            report.error(
                format!("periods[{k}]"),
                "has weights and returns too large to attribute",
            );
        } else if returns.len() > 1 {
            for (value, side) in [(r, "portfolio"), (b, "benchmark")] {
                if value <= -1.0 {
                    report.error(
                        format!("periods[{k}]"),
                        format!(
                            "the {side} returns {value}, -100% or less, so the periods cannot be \
                             linked"
                        ),
                    );
                }
            }
        }
    }
    report.into_result(())?;
    let (r, b, coefficients) = coefficients(returns, linking);
    let mut report = ValidationReport::new();
    let compounded =
        r.is_finite() && b.is_finite() && (returns.len() == 1 || (1.0 + r > 0.0 && 1.0 + b > 0.0));
    if !compounded || !finite(coefficients.iter().copied()) {
        report.error(
            "periods",
            "the returns compound to a value too close to -100%, or too large, to link",
        );
    } else {
        // The coefficients must reproduce the excess return; with returns of extreme sizes,
        // floating point may not be able to compute them accurately enough.
        let (mut linked, mut size) = (0.0, 1.0 + r.abs() + b.abs());
        for ((rt, bt, _), c) in returns.iter().zip(&coefficients) {
            linked += c * (rt - bt);
            size += c.abs() * (rt.abs() + bt.abs());
        }
        let gap = (linked - (r - b)).abs();
        if gap.is_nan() || gap > 1e-10 * size {
            report.error(
                "periods",
                "the returns are too far apart in size to link: the linked effects would not add \
                 up to the excess return",
            );
        }
    }
    report.into_result((r, b, coefficients))
}

fn coefficients(returns: &[(f64, f64, bool)], linking: Linking) -> (f64, f64, Vec<f64>) {
    let compound = |pick: fn(&(f64, f64, bool)) -> f64| {
        returns.iter().map(|x| 1.0 + pick(x)).product::<f64>() - 1.0
    };
    let r = compound(|x| x.0);
    let b = compound(|x| x.1);
    if returns.len() == 1 {
        return (r, b, vec![1.0]);
    }
    let coefficients = match linking {
        Linking::Carino => {
            let k = log_ratio(r, b);
            returns
                .iter()
                .map(|(rt, bt, _)| log_ratio(*rt, *bt) / k)
                .collect()
        }
        Linking::Menchero => {
            let n = returns.len() as f64;
            let difference = libm::pow(1.0 + r, 1.0 / n) - libm::pow(1.0 + b, 1.0 / n);
            let m = if (r - b).abs() < 1e-15 || difference == 0.0 {
                libm::pow(1.0 + r, (n - 1.0) / n)
            } else {
                (r - b) / n / difference
            };
            let gaps: Vec<f64> = returns.iter().map(|(rt, bt, _)| rt - bt).collect();
            let sum: f64 = gaps.iter().sum();
            let squares: f64 = gaps.iter().map(|g| g * g).sum();
            let residual = r - b - m * sum;
            // A residual within rounding error of zero is left alone: dividing it by the squares
            // of tiny differences would turn the rounding error into large coefficients.
            let scale: f64 =
                1.0 + r.abs() + b.abs() + m.abs() * gaps.iter().map(|g| g.abs()).sum::<f64>();
            let noise = 16.0 * f64::EPSILON * scale;
            if !(squares > 0.0 && residual.abs() > noise) {
                return (r, b, vec![m; returns.len()]);
            }
            // M + residual g / squares, rearranged as
            // (M sum_s g_s (g_s - g) + g (R - B)) / squares, so that a coefficient much smaller
            // than M does not come from cancelling two numbers the size of M.
            gaps.iter()
                .map(|&g| {
                    let spread: f64 = gaps.iter().map(|&s| s * (s - g)).sum();
                    (m * spread + g * (r - b)) / squares
                })
                .collect()
        }
    };
    (r, b, coefficients)
}

/// Cariño's `(ln(1 + R) - ln(1 + B)) / (R - B)`, or its limit `1 / (1 + R)` when they are equal
/// or too close to tell apart.
fn log_ratio(r: f64, b: f64) -> f64 {
    let logs = libm::log1p(r) - libm::log1p(b);
    if (r - b).abs() < 1e-15 || logs == 0.0 {
        1.0 / (1.0 + r)
    } else {
        logs / (r - b)
    }
}

/// Each segment's effects over all periods, scaled by the periods' coefficients and added up, in
/// the order the segments first appear.
fn linked_segments<'a>(
    periods: impl Iterator<Item = &'a [SegmentEffects]>,
    coefficients: &[f64],
) -> Vec<SegmentEffects> {
    let mut out: Vec<SegmentEffects> = Vec::new();
    for (segments, c) in periods.zip(coefficients) {
        for s in segments {
            let entry = match out.iter_mut().position(|e| e.name == s.name) {
                Some(k) => &mut out[k],
                None => {
                    out.push(SegmentEffects {
                        name: s.name.clone(),
                        allocation: 0.0,
                        selection: 0.0,
                        interaction: 0.0,
                        total: 0.0,
                    });
                    out.last_mut()
                        .unwrap_or_else(|| unreachable!("just pushed"))
                }
            };
            entry.allocation += c * s.allocation;
            entry.selection += c * s.selection;
            entry.interaction += c * s.interaction;
            entry.total += c * s.total;
        }
    }
    out
}

pub(crate) fn check_periods(
    periods: impl Iterator<Item = (Date, Date, usize)>,
    report: &mut ValidationReport,
) {
    let mut previous: Option<Date> = None;
    let mut count = 0;
    for (k, (start, end, segments)) in periods.enumerate() {
        count += 1;
        if start >= end {
            report.error(format!("periods[{k}]"), "must end after it starts");
        }
        if let Some(p) = previous
            && start < p
        {
            report.error(
                format!("periods[{k}].start"),
                "is before the end of the previous period",
            );
        }
        if segments == 0 {
            report.error(
                format!("periods[{k}].segments"),
                "needs at least one segment",
            );
        }
        previous = Some(end);
    }
    if count == 0 {
        report.error("periods", "needs at least one period");
    }
}

/// Checks `(name, w, r, b, s)` for each segment: unique names, finite weights adding up to one,
/// and returns above -100%.
pub(crate) fn check_segments<'a>(
    segments: impl Iterator<Item = (&'a str, f64, f64, f64, f64)>,
    path: &str,
    report: &mut ValidationReport,
) {
    let mut names = std::collections::BTreeSet::new();
    let (mut portfolio, mut benchmark) = (0.0, 0.0);
    for (k, (name, w, r, b, s)) in segments.enumerate() {
        if !names.insert(name) {
            report.error(
                format!("{path}[{k}].name"),
                format!("{name} is listed twice"),
            );
        }
        for (value, field) in [(w, "portfolio_weight"), (b, "benchmark_weight")] {
            if !value.is_finite() {
                report.error(format!("{path}[{k}].{field}"), "must be a finite number");
            }
        }
        for (value, field) in [(r, "portfolio_return"), (s, "benchmark_return")] {
            if !(value.is_finite() && value >= -1.0) {
                report.error(
                    format!("{path}[{k}].{field}"),
                    "must be a finite return of -100% or more",
                );
            }
        }
        portfolio += w;
        benchmark += b;
    }
    for (total, side) in [(portfolio, "portfolio"), (benchmark, "benchmark")] {
        if (total - 1.0).abs() > 1e-6 {
            report.error(path, format!("the {side} weights add up to {total}, not 1"));
        }
    }
}

/// A segment for currency attribution: its returns are in its own currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CurrencySegment {
    /// The segment's name.
    pub name: String,
    /// The currency of the segment's assets.
    pub currency: Currency,
    /// The portfolio's weight in the segment at the start of the period.
    pub portfolio_weight: f64,
    /// The return of the portfolio's holdings in the segment, in the segment's currency.
    pub portfolio_return: f64,
    /// The benchmark's weight in the segment at the start of the period.
    pub benchmark_weight: f64,
    /// The return of the benchmark's holdings in the segment, in the segment's currency.
    pub benchmark_return: f64,
}

/// A currency's returns over a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CurrencyReturn {
    /// The currency.
    pub currency: Currency,
    /// The change in its value in the base currency, such as 0.02 when it gains 2% (0 for the
    /// base currency).
    pub spot_return: f64,
    /// The return on cash in the currency over the period: its short-term interest rate.
    pub cash_return: f64,
}

/// One period for currency attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CurrencyAttributionPeriod {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// The segments, with returns in their own currencies.
    pub segments: Vec<CurrencySegment>,
    /// The returns of every currency of the segments, and of the base currency.
    pub currencies: Vec<CurrencyReturn>,
    /// The portfolio's exposure to each currency after any hedging, adding up to one; without
    /// it, each currency's share of the portfolio's segment weights (no hedging).
    #[serde(default)]
    pub portfolio_exposures: Option<BTreeMap<Currency, f64>>,
    /// The benchmark's exposure to each currency, as for the portfolio.
    #[serde(default)]
    pub benchmark_exposures: Option<BTreeMap<Currency, f64>>,
}

/// What one currency added to the excess return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CurrencyEffect {
    /// The currency.
    pub currency: Currency,
    /// The portfolio's exposure to it.
    pub portfolio_exposure: f64,
    /// The benchmark's exposure to it.
    pub benchmark_exposure: f64,
    /// `(u - v)(f - F)`, with `f` the currency's return including its interest rate and `F` the
    /// benchmark's currency return.
    pub allocation: f64,
}

/// The currency attribution of one period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CurrencyPeriodAttribution {
    /// The start of the period.
    pub start: Date,
    /// The end of the period.
    pub end: Date,
    /// The portfolio's return in the base currency.
    pub portfolio_return: f64,
    /// The benchmark's return in the base currency.
    pub benchmark_return: f64,
    /// The coefficient by which the period's effects are multiplied before they are added up
    /// over the periods (one for a single period).
    pub linking_coefficient: f64,
    /// Each segment's market effects, on returns in excess of its currency's cash return.
    pub segments: Vec<SegmentEffects>,
    /// Each currency's effect.
    pub currencies: Vec<CurrencyEffect>,
    /// What the model leaves out: the compounding of local and currency returns.
    pub residual: f64,
}

/// Currency attribution over one or more periods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CurrencyAttribution {
    /// How the periods were linked.
    pub linking: Linking,
    /// The portfolio's return in the base currency over all the periods, compounded.
    pub portfolio_return: f64,
    /// The benchmark's return in the base currency over all the periods, compounded.
    pub benchmark_return: f64,
    /// `portfolio_return - benchmark_return`, which the linked effects add up to.
    pub excess_return: f64,
    /// Each segment's market effects, linked.
    pub segments: Vec<SegmentEffects>,
    /// Each currency's allocation effect, linked, in the order the currencies first appear.
    pub currencies: Vec<CurrencyEffect>,
    /// The market allocation effects, linked.
    pub allocation: f64,
    /// The market selection effects, linked.
    pub selection: f64,
    /// The market interaction effects, linked.
    pub interaction: f64,
    /// The currency allocation effects, linked.
    pub currency: f64,
    /// The residuals, linked.
    pub residual: f64,
    /// Each period's own attribution, before linking.
    pub periods: Vec<CurrencyPeriodAttribution>,
}

/// Currency attribution by the method of Karnosky and Singer (1994): the excess return in the
/// base currency is split into market effects, measured on each segment's return in excess of
/// its currency's cash return, and currency effects, measured on each currency's return in the
/// base currency including its interest rate.
///
/// With `x = r - c` and `y = s - c` the portfolio and benchmark segment returns in excess of
/// their currency's cash return `c`, `f = (1 + c)(1 + e) - 1` the base-currency return of cash in
/// the currency (with `e` its spot return), and `u`, `v` the currency exposures:
///
/// - market allocation, selection and interaction are Brinson–Fachler on `x` and `y`;
/// - each currency's allocation effect is `(u - v)(f - F)`, with `F = sum v f`;
/// - the residual is the rest of the excess return in the base currency. Without given
///   exposures, a side's return is its unhedged return in the base currency, with each segment
///   returning `(1 + r)(1 + e) - 1`, and the model leaves out its cross terms (`sum w e x` for the
///   portfolio). With given exposures, a side's return is the model's, `sum w x + sum u f`, which
///   leaves nothing out; when both sides give them, the residual is zero.
///
/// Periods are linked as for [`brinson_fachler`], with the residual linked like the effects.
pub fn karnosky_singer(
    periods: &[CurrencyAttributionPeriod],
    linking: Linking,
) -> Result<CurrencyAttribution> {
    let mut report = ValidationReport::new();
    check_periods(
        periods.iter().map(|p| (p.start, p.end, p.segments.len())),
        &mut report,
    );
    for (k, p) in periods.iter().enumerate() {
        let path = format!("periods[{k}]");
        check_segments(
            p.segments.iter().map(|s| {
                (
                    s.name.as_str(),
                    s.portfolio_weight,
                    s.portfolio_return,
                    s.benchmark_weight,
                    s.benchmark_return,
                )
            }),
            &format!("{path}.segments"),
            &mut report,
        );
        for (j, c) in p.currencies.iter().enumerate() {
            for (value, field) in [
                (c.spot_return, "spot_return"),
                (c.cash_return, "cash_return"),
            ] {
                if !(value.is_finite() && value > -1.0) {
                    report.error(
                        format!("{path}.currencies[{j}].{field}"),
                        "must be a finite return above -100%",
                    );
                }
            }
        }
        for (i, s) in p.segments.iter().enumerate() {
            if !p.currencies.iter().any(|c| c.currency == s.currency) {
                report.error(
                    format!("{path}.segments[{i}].currency"),
                    format!("{} has no returns in currencies", s.currency),
                );
            }
        }
        for (exposures, field) in [
            (&p.portfolio_exposures, "portfolio_exposures"),
            (&p.benchmark_exposures, "benchmark_exposures"),
        ] {
            if let Some(list) = exposures {
                let total: f64 = list.values().sum();
                if !total.is_finite() || (total - 1.0).abs() > 1e-6 {
                    report.error(
                        format!("{path}.{field}"),
                        format!("add up to {total}, not 1"),
                    );
                }
                for c in list.keys() {
                    if !p.currencies.iter().any(|r| r.currency == *c) {
                        report.error(
                            format!("{path}.{field}.{c}"),
                            format!("{c} has no returns in currencies"),
                        );
                    }
                }
            }
        }
    }
    report.into_result(())?;

    let mut attributed: Vec<CurrencyPeriodAttribution> =
        periods.iter().map(currency_period).collect();
    let returns: Vec<(f64, f64, bool)> = attributed
        .iter()
        .map(|p| {
            let currencies = p.currencies.iter().map(|c| c.allocation);
            let finite = finite(
                p.segments
                    .iter()
                    .flat_map(numbers)
                    .chain(currencies)
                    .chain([p.residual]),
            );
            (p.portfolio_return, p.benchmark_return, finite)
        })
        .collect();
    let (portfolio_return, benchmark_return, coefficients) = link(&returns, linking)?;
    for (p, c) in attributed.iter_mut().zip(&coefficients) {
        p.linking_coefficient = *c;
    }
    let segments = linked_segments(
        attributed.iter().map(|p| p.segments.as_slice()),
        &coefficients,
    );
    let mut currencies: Vec<CurrencyEffect> = Vec::new();
    for (p, c) in attributed.iter().zip(&coefficients) {
        for e in &p.currencies {
            match currencies.iter_mut().find(|x| x.currency == e.currency) {
                Some(x) => x.allocation += c * e.allocation,
                None => currencies.push(CurrencyEffect {
                    allocation: c * e.allocation,
                    ..e.clone()
                }),
            }
        }
    }
    // Over several periods the exposures differ from period to period; report the average.
    for x in &mut currencies {
        let (mut u, mut v) = (0.0, 0.0);
        for p in &attributed {
            if let Some(e) = p.currencies.iter().find(|e| e.currency == x.currency) {
                u += e.portfolio_exposure;
                v += e.benchmark_exposure;
            }
        }
        x.portfolio_exposure = u / attributed.len() as f64;
        x.benchmark_exposure = v / attributed.len() as f64;
    }
    let residual = attributed
        .iter()
        .zip(&coefficients)
        .map(|(p, c)| c * p.residual)
        .sum();
    let sum = |pick: fn(&SegmentEffects) -> f64| segments.iter().map(pick).sum::<f64>();
    let attribution = CurrencyAttribution {
        linking,
        portfolio_return,
        benchmark_return,
        excess_return: portfolio_return - benchmark_return,
        allocation: sum(|s| s.allocation),
        selection: sum(|s| s.selection),
        interaction: sum(|s| s.interaction),
        currency: currencies.iter().map(|c| c.allocation).sum(),
        residual,
        segments,
        currencies,
        periods: attributed,
    };
    let totals = [
        attribution.excess_return,
        attribution.allocation,
        attribution.selection,
        attribution.interaction,
        attribution.currency,
        attribution.residual,
    ];
    linked_finite(
        totals
            .into_iter()
            .chain(attribution.segments.iter().flat_map(numbers))
            .chain(attribution.currencies.iter().map(|c| c.allocation)),
    )?;
    Ok(attribution)
}

fn currency_period(p: &CurrencyAttributionPeriod) -> CurrencyPeriodAttribution {
    let rates = |c: Currency| {
        p.currencies
            .iter()
            .find(|r| r.currency == c)
            .map_or((0.0, 0.0), |r| (r.spot_return, r.cash_return))
    };
    // Market effects on returns in excess of each currency's cash return.
    let (market_portfolio, market_benchmark, segments) = effects(p.segments.iter().map(|s| {
        let (_, cash) = rates(s.currency);
        (
            s.name.as_str(),
            s.portfolio_weight,
            s.portfolio_return - cash,
            s.benchmark_weight,
            s.benchmark_return - cash,
        )
    }));
    // Exposures: given, or each currency's share of the segment weights.
    let implied = |benchmark: bool| -> BTreeMap<Currency, f64> {
        let mut out: BTreeMap<Currency, f64> = BTreeMap::new();
        for s in &p.segments {
            let w = if benchmark {
                s.benchmark_weight
            } else {
                s.portfolio_weight
            };
            *out.entry(s.currency).or_insert(0.0) += w;
        }
        out
    };
    let u = p
        .portfolio_exposures
        .clone()
        .unwrap_or_else(|| implied(false));
    let v = p
        .benchmark_exposures
        .clone()
        .unwrap_or_else(|| implied(true));
    let currency_return = |c: Currency| {
        let (spot, cash) = rates(c);
        (1.0 + cash) * (1.0 + spot) - 1.0
    };
    let weight = |map: &BTreeMap<Currency, f64>, c: Currency| map.get(&c).copied().unwrap_or(0.0);
    // The currencies in the order they first appear in the segments, then any others the
    // exposures name.
    let mut names: Vec<Currency> = Vec::new();
    for c in p
        .segments
        .iter()
        .map(|s| s.currency)
        .chain(u.keys().copied())
        .chain(v.keys().copied())
    {
        if !names.contains(&c) {
            names.push(c);
        }
    }
    let benchmark_currency: f64 = names
        .iter()
        .map(|c| weight(&v, *c) * currency_return(*c))
        .sum();
    let portfolio_currency: f64 = names
        .iter()
        .map(|c| weight(&u, *c) * currency_return(*c))
        .sum();
    let currencies: Vec<CurrencyEffect> = names
        .iter()
        .map(|c| {
            let (uc, vc) = (weight(&u, *c), weight(&v, *c));
            CurrencyEffect {
                currency: *c,
                portfolio_exposure: uc,
                benchmark_exposure: vc,
                allocation: (uc - vc) * (currency_return(*c) - benchmark_currency),
            }
        })
        .collect();
    let model_portfolio = market_portfolio + portfolio_currency;
    let model_benchmark = market_benchmark + benchmark_currency;
    // A side with given exposures has the model's return; a side without has its unhedged
    // return in the base currency, of which the model leaves out the cross terms.
    let unhedged = |benchmark: bool| -> f64 {
        p.segments
            .iter()
            .map(|s| {
                let (w, r) = if benchmark {
                    (s.benchmark_weight, s.benchmark_return)
                } else {
                    (s.portfolio_weight, s.portfolio_return)
                };
                w * ((1.0 + r) * (1.0 + rates(s.currency).0) - 1.0)
            })
            .sum()
    };
    let portfolio_return = if p.portfolio_exposures.is_some() {
        model_portfolio
    } else {
        unhedged(false)
    };
    let benchmark_return = if p.benchmark_exposures.is_some() {
        model_benchmark
    } else {
        unhedged(true)
    };
    let residual = (portfolio_return - benchmark_return) - (model_portfolio - model_benchmark);
    CurrencyPeriodAttribution {
        start: p.start,
        end: p.end,
        portfolio_return,
        benchmark_return,
        linking_coefficient: 1.0,
        segments,
        currencies,
        residual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finalytics_core::Error;

    fn d(s: &str) -> Date {
        s.parse().unwrap_or_else(|_| unreachable!())
    }

    fn segment(name: &str, w: f64, r: f64, b: f64, s: f64) -> Segment {
        Segment {
            name: name.into(),
            portfolio_weight: w,
            portfolio_return: r,
            benchmark_weight: b,
            benchmark_return: s,
        }
    }

    /// A worked example: portfolio 8.3%, benchmark 6.4%.
    fn worked() -> AttributionPeriod {
        AttributionPeriod {
            start: d("2026-01-01"),
            end: d("2026-12-31"),
            segments: vec![
                segment("UK", 0.4, 0.20, 0.4, 0.10),
                segment("Japan", 0.3, -0.05, 0.2, -0.04),
                segment("US", 0.3, 0.06, 0.4, 0.08),
            ],
        }
    }

    #[test]
    fn a_worked_example_by_hand() {
        let a = brinson_fachler(&[worked()], Linking::Carino).unwrap_or_else(|e| panic!("{e}"));
        let close = |x: f64, y: f64| assert!((x - y).abs() < 1e-15, "{x} against {y}");
        close(a.portfolio_return, 0.083);
        close(a.benchmark_return, 0.064);
        // Allocation: UK 0, Japan 0.1 x (-0.04 - 0.064), US -0.1 x (0.08 - 0.064).
        close(a.segments[1].allocation, -0.0104);
        close(a.segments[2].allocation, -0.0016);
        // Selection: UK 0.4 x 0.10, Japan 0.2 x -0.01, US 0.4 x -0.02.
        close(a.selection, 0.04 - 0.002 - 0.008);
        // Interaction: Japan 0.1 x -0.01, US -0.1 x -0.02.
        close(a.interaction, -0.001 + 0.002);
        close(a.allocation + a.selection + a.interaction, a.excess_return);
    }

    #[test]
    fn linked_effects_add_up_to_the_excess_return() {
        let mut second = worked();
        second.start = d("2027-01-01");
        second.end = d("2027-12-31");
        second.segments[0].portfolio_return = -0.03;
        second.segments[2].benchmark_return = 0.12;
        for linking in [Linking::Carino, Linking::Menchero] {
            let a = brinson_fachler(&[worked(), second.clone()], linking)
                .unwrap_or_else(|e| panic!("{e}"));
            let total = a.allocation + a.selection + a.interaction;
            assert!((total - a.excess_return).abs() < 1e-15, "{linking:?}");
            let by_segment: f64 = a.segments.iter().map(|s| s.total).sum();
            assert!((by_segment - a.excess_return).abs() < 1e-15, "{linking:?}");
        }
    }

    fn currency_example(
        portfolio_exposures: Option<BTreeMap<Currency, f64>>,
    ) -> CurrencyAttribution {
        let segment = |name: &str, currency, w, r, b, s| CurrencySegment {
            name: name.into(),
            currency,
            portfolio_weight: w,
            portfolio_return: r,
            benchmark_weight: b,
            benchmark_return: s,
        };
        let rate = |currency, spot_return, cash_return| CurrencyReturn {
            currency,
            spot_return,
            cash_return,
        };
        let period = CurrencyAttributionPeriod {
            start: d("2026-01-01"),
            end: d("2026-12-31"),
            segments: vec![
                segment("US", Currency::USD, 0.5, 0.03, 0.6, 0.035),
                segment("Europe", Currency::EUR, 0.5, 0.05, 0.4, 0.04),
            ],
            currencies: vec![
                rate(Currency::USD, 0.0, 0.04),
                rate(Currency::EUR, 0.03, 0.025),
            ],
            portfolio_exposures,
            benchmark_exposures: None,
        };
        karnosky_singer(&[period], Linking::Carino).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn currency_effects_by_hand() {
        let close = |x: f64, y: f64| assert!((x - y).abs() < 1e-15, "{x} against {y}");
        let a = currency_example(None);
        // Unhedged: 0.5 x 0.03 + 0.5 x (1.05 x 1.03 - 1), and 0.6 x 0.035 + 0.4 x (1.04 x 1.03 - 1).
        close(a.portfolio_return, 0.05575);
        close(a.benchmark_return, 0.04948);
        // Euro cash returns 1.025 x 1.03 - 1 = 0.05575 in dollars, and the benchmark's currencies
        // 0.6 x 0.04 + 0.4 x 0.05575 = 0.0463.
        close(a.currencies[0].allocation, -0.1 * (0.04 - 0.0463));
        close(a.currencies[1].allocation, 0.1 * (0.05575 - 0.0463));
        // The cross terms: 0.5 x 0.03 x 0.025 for the portfolio, 0.4 x 0.03 x 0.015 for the
        // benchmark.
        close(a.residual, 0.000375 - 0.00018);

        // Hedging half the euros changes the portfolio's return and currency effects, not the
        // benchmark's return.
        let hedged = currency_example(Some(BTreeMap::from([
            (Currency::USD, 0.75),
            (Currency::EUR, 0.25),
        ])));
        close(hedged.benchmark_return, a.benchmark_return);
        close(hedged.allocation, a.allocation);
        close(
            hedged.portfolio_return,
            0.0075 + 0.75 * 0.04 + 0.25 * 0.05575,
        );
        close(hedged.residual, -0.00018);
        let total = hedged.allocation
            + hedged.selection
            + hedged.interaction
            + hedged.currency
            + hedged.residual;
        close(total, hedged.excess_return);
    }

    #[test]
    fn a_loss_of_more_than_the_portfolio_is_attributed_but_not_linked() {
        // Short the bonds to double the equities: the portfolio loses 230%.
        let mut short = worked();
        short.segments = vec![
            segment("equities", 2.0, -0.9, 0.6, -0.5),
            segment("bonds", -1.0, 0.5, 0.4, 0.01),
        ];
        let a = brinson_fachler(std::slice::from_ref(&short), Linking::Carino)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!((a.portfolio_return + 2.3).abs() < 1e-15);
        assert!((a.allocation + a.selection + a.interaction - a.excess_return).abs() < 1e-15);

        let mut next = worked();
        next.start = d("2027-01-01");
        next.end = d("2027-12-31");
        for linking in [Linking::Carino, Linking::Menchero] {
            let Err(Error::Validation(report)) =
                brinson_fachler(&[short.clone(), next.clone()], linking)
            else {
                panic!("expected a validation error");
            };
            assert_eq!(report.issues()[0].path, "periods[0]");
        }
    }

    #[test]
    fn differences_within_rounding_error_are_not_amplified() {
        // In each period the portfolio and benchmark returns differ by one unit in the last place,
        // while allocation and selection are around 1% each and offset. Dividing the rounding
        // error of the compounded returns by the squared differences would give Menchero
        // coefficients of about -0.08, -0.08 and 0.2; they should all be close to M.
        let rows = [
            (0.11, -0.01, 0.09, -0.039999999999999966),
            (0.05, 0.03, 0.030000000000000002, 0.050000000000000024),
            (-0.04, 0.01, -0.06, 0.065),
        ];
        let periods: Vec<AttributionPeriod> = rows
            .iter()
            .enumerate()
            .map(|(k, &(s1, s2, r1, r2))| AttributionPeriod {
                start: d(&format!("{}-01-01", 2026 + k)),
                end: d(&format!("{}-12-31", 2026 + k)),
                segments: vec![
                    segment("equities", 0.6, r1, 0.4, s1),
                    segment("bonds", 0.4, r2, 0.6, s2),
                ],
            })
            .collect();
        let a = brinson_fachler(&periods, Linking::Menchero).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            a.periods
                .iter()
                .all(|p| p.portfolio_return != p.benchmark_return)
        );
        let m = libm::pow(1.0 + a.portfolio_return, 2.0 / 3.0);
        let allocation: f64 = a
            .periods
            .iter()
            .flat_map(|p| p.segments.iter().map(|s| s.allocation))
            .sum();
        assert!(
            (a.allocation - m * allocation).abs() < 1e-12,
            "{}",
            a.allocation
        );
    }

    #[test]
    fn menchero_coefficients_far_below_m_are_computed_without_cancellation() {
        // One period's return dwarfs the others, so M is about 4e61 while that period's
        // coefficient is about 1.7: M + a g would cancel two numbers of the size of M.
        let periods: Vec<AttributionPeriod> =
            [(8.608812913674839e92, 0.0), (0.0, 0.0), (0.733, 0.0)]
                .iter()
                .enumerate()
                .map(|(k, &(r, s))| AttributionPeriod {
                    start: d(&format!("{}-01-01", 2026 + k)),
                    end: d(&format!("{}-12-31", 2026 + k)),
                    segments: vec![segment("all", 1.0, r, 1.0, s)],
                })
                .collect();
        let a = brinson_fachler(&periods, Linking::Menchero).unwrap_or_else(|e| panic!("{e}"));
        let c: Vec<f64> = a.periods.iter().map(|p| p.linking_coefficient).collect();
        assert!(c[0] > 1.7 && c[0] < 1.8, "{c:?}");
        assert!((a.selection - a.excess_return).abs() <= 1e-12 * a.excess_return);
    }

    #[test]
    fn weights_must_add_up_to_one() {
        let mut bad = worked();
        bad.segments[0].portfolio_weight = 0.5;
        bad.segments[1].name = "UK".into();
        let Err(Error::Validation(report)) = brinson_fachler(&[bad], Linking::Carino) else {
            panic!("expected a validation error");
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(
            paths,
            ["periods[0].segments[1].name", "periods[0].segments"]
        );
    }
}
