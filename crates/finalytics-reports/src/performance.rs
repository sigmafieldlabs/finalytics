//! The performance report: value and growth over time, returns over the standard periods,
//! monthly and calendar-year returns, contribution, flows and income, and the household.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Result, ValidationReport};
use finalytics_ledger::{Allocation, Books, MarketData};
use finalytics_perf::{
    Contribution, GroupContribution, Household, HouseholdOptions, Performance, Period,
    group_contributions, household,
};

use crate::context::{
    Context, Provenance, ReportKind, ReportOptions, Source, group_of, time_series, to_f64,
};
use crate::shapes::{Grid, Series, Unit};

/// Returns over one reporting period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PeriodRow {
    /// The period.
    pub period: Period,
    /// Its short name: "MTD", "QTD", "YTD", "1Y", "3Y", "5Y" or "ITD".
    pub label: String,
    /// The valuation the period starts from, `None` when the history does not reach back to
    /// the period's start (the other figures are then `None` too).
    pub start: Option<Date>,
    /// The valuation it ends on.
    pub end: Option<Date>,
    /// The cumulative time-weighted return.
    pub cumulative: Option<f64>,
    /// The annualised time-weighted return, for periods of a year or more.
    pub annualised: Option<f64>,
    /// The money-weighted return (annualised internal rate of return).
    pub money_weighted: Option<f64>,
    /// The Modified Dietz return.
    pub modified_dietz: Option<f64>,
    /// The benchmark's return over the same dates.
    pub benchmark: Option<f64>,
    /// `cumulative - benchmark`.
    pub excess_arithmetic: Option<f64>,
    /// `(1 + cumulative) / (1 + benchmark) - 1`.
    pub excess_geometric: Option<f64>,
}

/// The return of one calendar year (or the part of it covered).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct YearReturn {
    /// The year.
    pub year: i32,
    /// The valuation the year's return starts from: the last one of the year before, or the
    /// first valuation.
    pub start: Date,
    /// The last valuation of the year.
    pub end: Date,
    /// Whether the valuations cover the whole year.
    pub complete: bool,
    /// The portfolio's time-weighted return.
    pub portfolio: f64,
    /// The benchmark's return.
    pub benchmark: Option<f64>,
    /// `portfolio - benchmark`.
    pub excess: Option<f64>,
}

/// Contributions added up by the groups of one classification scheme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemeContributions {
    /// The classification scheme.
    pub scheme: String,
    /// Each group, in name order.
    pub groups: Vec<GroupContribution>,
}

/// External flows and income in one calendar month, in the base currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MonthFlows {
    /// The month, as `YYYY-MM`.
    pub month: String,
    /// Deposits and transfers in, positive.
    pub inflows: f64,
    /// Withdrawals and transfers out, negative.
    pub outflows: f64,
    /// Income, net of tax withheld.
    pub income: f64,
}

/// The performance report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PerformanceReport {
    /// Where the numbers come from.
    pub provenance: Provenance,
    /// The portfolio's value on each valuation date.
    pub value: Series,
    /// The growth of 100 at the time-weighted return.
    pub growth: Series,
    /// The benchmark's growth of 100 over the same dates, with a benchmark.
    pub benchmark_growth: Option<Series>,
    /// The daily time-weighted returns.
    pub returns: Series,
    /// Returns over MTD, QTD, YTD, one, three and five years, and since inception.
    pub periods: Vec<PeriodRow>,
    /// Monthly returns: one row per year, one column per month and a last column for the year.
    pub monthly: Grid,
    /// The benchmark's monthly returns, in the same layout, with a benchmark.
    pub benchmark_monthly: Option<Grid>,
    /// Calendar-year returns.
    pub calendar_years: Vec<YearReturn>,
    /// Each holding's contribution to the return from the first valuation to the last.
    pub contributions: Vec<Contribution>,
    /// Contributions by each scheme of the options.
    pub groups: Vec<SchemeContributions>,
    /// The holdings that contributed most, largest first.
    pub top_contributors: Vec<Contribution>,
    /// The holdings that took away most, most negative first.
    pub top_detractors: Vec<Contribution>,
    /// External flows and income by month.
    pub flows: Vec<MonthFlows>,
    /// The household by account and by group of accounts, when the portfolio has more than one
    /// account.
    pub household: Option<Household>,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Performance from the first valuation date to the report's date.
pub fn performance_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<PerformanceReport> {
    let cx = Context::new(books, market, source, options)?;
    let values = books.value_series(&cx.dates, market)?;
    let mut warnings = values.warnings.clone();
    let performance = Performance::from_values(&values, options.timing)?;
    let holdings = books.holding_series(&cx.dates, market)?;
    let unit = Unit::Money { currency: cx.base };
    let value = Series {
        name: "Value".to_owned(),
        unit,
        dates: cx.dates.clone(),
        values: values.values.iter().copied().map(to_f64).collect(),
    };
    let growth = performance.growth_of(100.0);
    let growth = Series {
        name: "Portfolio".to_owned(),
        unit: Unit::Level,
        dates: growth.dates().to_vec(),
        values: growth.values().to_vec(),
    };
    let levels = cx.benchmark_levels();
    if options.benchmark.is_some() && levels.is_none() {
        warnings.warning(
            "options.benchmark",
            "the benchmark has no price on some valuation dates, so it is left out",
        );
    }
    let benchmark_name = options
        .benchmark
        .as_ref()
        .map_or_else(String::new, ToString::to_string);
    let benchmark_growth = levels.as_ref().map(|l| Series {
        name: benchmark_name.clone(),
        unit: Unit::Level,
        dates: cx.dates.clone(),
        values: l.iter().map(|v| v / l[0] * 100.0).collect(),
    });
    let returns = performance.returns();
    let returns = Series {
        name: "Daily return".to_owned(),
        unit: Unit::Fraction,
        dates: returns.dates().to_vec(),
        values: returns.values().to_vec(),
    };

    let benchmark_series = levels.as_ref().and_then(|l| time_series(&cx.dates, l));
    let periods = [
        Period::MonthToDate,
        Period::QuarterToDate,
        Period::YearToDate,
        Period::Years { years: 1 },
        Period::Years { years: 3 },
        Period::Years { years: 5 },
        Period::SinceInception,
    ]
    .into_iter()
    .map(|period| {
        let found = performance.period(period, Some(cx.as_of));
        let excess = benchmark_series.as_ref().and_then(|b| {
            performance.excess_return(period, Some(cx.as_of), b, market.max_carry_days())
        });
        PeriodRow {
            period,
            label: period.label(),
            start: found.as_ref().map(|p| p.start),
            end: found.as_ref().map(|p| p.end),
            cumulative: found.as_ref().map(|p| p.cumulative),
            annualised: found.as_ref().and_then(|p| p.annualised),
            money_weighted: found
                .as_ref()
                .and_then(|p| performance.money_weighted(p.start, p.end)),
            modified_dietz: found
                .as_ref()
                .and_then(|p| performance.modified_dietz(p.start, p.end)),
            benchmark: excess.as_ref().map(|e| e.benchmark),
            excess_arithmetic: excess.as_ref().map(|e| e.arithmetic),
            excess_geometric: excess.as_ref().map(|e| e.geometric),
        }
    })
    .collect();

    let (monthly, years) = monthly_grid("Monthly returns", &cx.dates, &growth.values);
    let benchmark_monthly = levels
        .as_ref()
        .map(|l| monthly_grid("Benchmark monthly returns", &cx.dates, l));
    let calendar_years = years
        .iter()
        .map(|y| {
            let benchmark = benchmark_monthly
                .as_ref()
                .and_then(|(_, b)| b.iter().find(|x| x.year == y.year))
                .map(|x| x.portfolio);
            YearReturn {
                benchmark,
                excess: benchmark.map(|b| y.portfolio - b),
                ..y.clone()
            }
        })
        .collect();

    let (first, last) = (cx.dates[0], cx.dates[cx.dates.len() - 1]);
    let contributions = performance.contributions(&holdings, first, last)?;
    let groups = options
        .group_by
        .iter()
        .map(|scheme| SchemeContributions {
            scheme: scheme.clone(),
            groups: group_contributions(&contributions, |h| group_of(books, h, scheme)),
        })
        .collect();
    let mut ranked = contributions.clone();
    ranked.sort_by(|a, b| b.contribution.total_cmp(&a.contribution));
    let top_contributors: Vec<Contribution> = ranked
        .iter()
        .filter(|c| c.contribution > 0.0)
        .take(options.top)
        .cloned()
        .collect();
    let top_detractors: Vec<Contribution> = ranked
        .iter()
        .rev()
        .filter(|c| c.contribution < 0.0)
        .take(options.top)
        .cloned()
        .collect();

    let flows = month_flows(books, market, &cx, &mut warnings);
    let household = if books.accounts().len() > 1 {
        let schemes: BTreeSet<String> = books
            .accounts()
            .iter()
            .flat_map(|a| a.classifications.keys().cloned())
            .collect();
        let options = HouseholdOptions {
            group_by: schemes.into_iter().collect(),
            timing: options.timing,
        };
        match household(books, market, &cx.dates, &options) {
            Ok(h) => Some(h),
            Err(e) => {
                warnings.warning(
                    "household",
                    format!("the household could not be measured: {e}"),
                );
                None
            }
        }
    } else {
        None
    };

    Ok(PerformanceReport {
        provenance: cx.provenance(ReportKind::Performance, warnings),
        value,
        growth,
        benchmark_growth,
        returns,
        periods,
        monthly,
        benchmark_monthly: benchmark_monthly.map(|(g, _)| g),
        calendar_years,
        contributions,
        groups,
        top_contributors,
        top_detractors,
        flows,
        household,
    })
}

/// Monthly and yearly returns of `levels` (an index such as growth of 100) on `dates`: a grid
/// with a row per year and a column per month, then the year, and the calendar-year returns.
/// Each month's return runs from the last valuation of the month before (or the first
/// valuation) to the month's last.
pub(crate) fn monthly_grid(name: &str, dates: &[Date], levels: &[f64]) -> (Grid, Vec<YearReturn>) {
    let mut month_ends: BTreeMap<(i32, u32), usize> = BTreeMap::new();
    for (k, d) in dates.iter().enumerate() {
        month_ends.insert((d.year(), d.month()), k);
    }
    let years: BTreeSet<i32> = month_ends.keys().map(|(y, _)| *y).collect();
    let mut rows = Vec::new();
    let mut values = Vec::new();
    let mut calendar = Vec::new();
    let mut previous_month = 0usize;
    let mut previous_year = 0usize;
    for &year in &years {
        let mut row = vec![None; 13];
        for (m, cell) in row.iter_mut().enumerate().take(12) {
            if let Some(&end) = month_ends.get(&(year, m as u32 + 1)) {
                if end > previous_month {
                    *cell = Some(levels[end] / levels[previous_month] - 1.0);
                }
                previous_month = end;
            }
        }
        let end = month_ends
            .range((year, 1)..=(year, 12))
            .next_back()
            .map_or(previous_year, |(_, &k)| k);
        if end > previous_year {
            let portfolio = levels[end] / levels[previous_year] - 1.0;
            row[12] = Some(portfolio);
            calendar.push(YearReturn {
                year,
                start: dates[previous_year],
                end: dates[end],
                complete: dates[previous_year].year() < year && dates[end].month() == 12,
                portfolio,
                benchmark: None,
                excess: None,
            });
        }
        previous_year = end;
        rows.push(year.to_string());
        values.push(row);
    }
    let mut columns: Vec<String> = MONTHS.iter().map(|m| (*m).to_owned()).collect();
    columns.push("Year".to_owned());
    (
        Grid {
            name: name.to_owned(),
            unit: Unit::Fraction,
            rows,
            columns,
            values,
        },
        calendar,
    )
}

/// Flows and income by calendar month of their own dates, from the first valuation date to
/// the report's date.
fn month_flows(
    books: &Books,
    market: &MarketData,
    cx: &Context<'_>,
    warnings: &mut ValidationReport,
) -> Vec<MonthFlows> {
    let key = |d: Date| format!("{:04}-{:02}", d.year(), d.month());
    let mut months: BTreeMap<String, MonthFlows> = BTreeMap::new();
    let base = books.base_currency();
    for event in books
        .events()
        .iter()
        .filter(|e| e.date >= cx.start && e.date <= cx.as_of)
    {
        let mut value =
            |currency: finalytics_core::Currency, amount: finalytics_core::Decimal| match market
                .fx_rate(currency, base, event.date)
                .value()
            {
                Some(rate) => Some(to_f64(amount * *rate)),
                None => {
                    warnings.warning(
                        format!("fx.{currency}{base}"),
                        format!(
                            "no rate on or shortly before {}; {amount} {currency} is left out of \
                             the monthly flows",
                            event.date
                        ),
                    );
                    None
                }
            };
        let month = key(event.date);
        let mut add = |inflow: f64, outflow: f64, income: f64| {
            let e = months.entry(month.clone()).or_insert(MonthFlows {
                month: month.clone(),
                inflows: 0.0,
                outflows: 0.0,
                income: 0.0,
            });
            e.inflows += inflow;
            e.outflows += outflow;
            e.income += income;
        };
        for f in &event.flows {
            if let Some(v) = value(f.currency, f.amount) {
                if v > 0.0 {
                    add(v, 0.0, 0.0);
                } else {
                    add(0.0, v, 0.0);
                }
            }
        }
        for allocation in &event.allocations {
            if let Allocation::Income {
                currency, amount, ..
            } = allocation
                && let Some(v) = value(*currency, *amount)
            {
                add(0.0, 0.0, v);
            }
        }
    }
    months.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd(y, m, d).unwrap()
    }

    #[test]
    fn months_and_years_run_from_the_last_valuation_before() {
        let dates = [
            date(2025, 12, 31),
            date(2026, 1, 15),
            date(2026, 1, 30),
            date(2026, 2, 27),
        ];
        let (grid, years) = monthly_grid("Monthly returns", &dates, &[100.0, 104.0, 110.0, 99.0]);
        assert_eq!(grid.rows, ["2025", "2026"]);
        assert_eq!(grid.columns.len(), 13);
        assert!(grid.values[0].iter().all(Option::is_none));
        let row = &grid.values[1];
        assert!((row[0].unwrap() - 0.1).abs() < 1e-15);
        assert!((row[1].unwrap() + 0.1).abs() < 1e-15);
        assert!(row[2..12].iter().all(Option::is_none));
        assert!((row[12].unwrap() + 0.01).abs() < 1e-15);
        assert_eq!(years.len(), 1);
        assert_eq!((years[0].start, years[0].end), (dates[0], dates[3]));
        assert!(!years[0].complete);
    }
}
