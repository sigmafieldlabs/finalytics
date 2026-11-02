//! Tables of prices, and the returns and frequency they imply.

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Error, Result, ValidationReport};

use crate::matrix::ReturnsTable;

/// The prices of several assets on the same dates, one column per asset. A missing price is
/// written as absent (`null`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "PriceTableRepr", into = "PriceTableRepr")]
pub struct PriceTable {
    dates: Vec<Date>,
    labels: Vec<String>,
    columns: Vec<Vec<Option<f64>>>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct PriceTableRepr {
    /// The dates, in increasing order.
    dates: Vec<Date>,
    /// The assets, one per column.
    labels: Vec<String>,
    /// The prices of each asset, one per date; absent where there is no price.
    columns: Vec<Vec<Option<f64>>>,
}

impl TryFrom<PriceTableRepr> for PriceTable {
    type Error = Error;

    fn try_from(repr: PriceTableRepr) -> Result<PriceTable> {
        PriceTable::new(repr.dates, repr.labels, repr.columns)
    }
}

impl From<PriceTable> for PriceTableRepr {
    fn from(table: PriceTable) -> PriceTableRepr {
        PriceTableRepr {
            dates: table.dates,
            labels: table.labels,
            columns: table.columns,
        }
    }
}

/// The returns a table of prices implies.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PriceReturns {
    /// The date each return ends on.
    pub dates: Vec<Date>,
    /// The returns, one column per asset.
    pub table: ReturnsTable,
    /// The dates left out because an asset had no price on them.
    pub warnings: ValidationReport,
}

impl PriceTable {
    /// A table of prices: dates in increasing order, one unique label per column, and one price
    /// per date in each column, greater than zero where given.
    pub fn new(
        dates: Vec<Date>,
        labels: Vec<String>,
        columns: Vec<Vec<Option<f64>>>,
    ) -> Result<PriceTable> {
        let mut report = ValidationReport::new();
        for (i, pair) in dates.windows(2).enumerate() {
            if pair[1] <= pair[0] {
                report.error(
                    format!("dates[{}]", i + 1),
                    format!("{} does not follow {}", pair[1], pair[0]),
                );
            }
        }
        if labels.len() != columns.len() {
            report.error(
                "labels",
                format!("{} labels for {} columns", labels.len(), columns.len()),
            );
        }
        if columns.is_empty() {
            report.error("columns", "need at least one asset");
        }
        for (i, label) in labels.iter().enumerate() {
            if label.trim().is_empty() {
                report.error(format!("labels[{i}]"), "must not be empty");
            } else if labels[..i].contains(label) {
                report.error(format!("labels[{i}]"), format!("{label} appears twice"));
            }
        }
        for (j, column) in columns.iter().enumerate() {
            if column.len() != dates.len() {
                report.error(
                    format!("columns[{j}]"),
                    format!("{} prices for {} dates", column.len(), dates.len()),
                );
            }
            for (i, price) in column.iter().enumerate() {
                if let Some(p) = price
                    && !(p.is_finite() && *p > 0.0)
                {
                    report.error(
                        format!("columns[{j}][{i}]"),
                        "must be a price greater than zero",
                    );
                }
            }
        }
        report.into_result(PriceTable {
            dates,
            labels,
            columns,
        })
    }

    /// The dates.
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }

    /// The assets.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The prices of each asset.
    pub fn columns(&self) -> &[Vec<Option<f64>>] {
        &self.columns
    }

    /// The simple returns between consecutive dates on which every asset has a price. Dates on
    /// which an asset has no price are left out, with a warning; fewer than three dates with
    /// every price is an error.
    pub fn returns(&self) -> Result<PriceReturns> {
        let mut warnings = ValidationReport::new();
        let complete: Vec<usize> = (0..self.dates.len())
            .filter(|&i| self.columns.iter().all(|c| c[i].is_some()))
            .collect();
        let missing = self.dates.len() - complete.len();
        if missing > 0 {
            warnings.warning(
                "columns",
                format!(
                    "{missing} date(s) where an asset has no price are left out, so that every \
                     asset's returns cover the same periods"
                ),
            );
        }
        if complete.len() < 3 {
            let mut report = ValidationReport::new();
            report.error(
                "dates",
                format!(
                    "need at least 3 dates on which every asset has a price, found {}",
                    complete.len()
                ),
            );
            return Err(Error::Validation(report));
        }
        let returns = self
            .columns
            .iter()
            .map(|column| {
                complete
                    .windows(2)
                    .map(|w| {
                        let (a, b) = (column[w[0]].unwrap_or(1.0), column[w[1]].unwrap_or(1.0));
                        b / a - 1.0
                    })
                    .collect()
            })
            .collect();
        Ok(PriceReturns {
            dates: complete[1..].iter().map(|&i| self.dates[i]).collect(),
            table: ReturnsTable::new(self.labels.clone(), returns)?,
            warnings,
        })
    }
}

/// The number of periods in a year that dates spaced like `dates` imply, from the median gap
/// between consecutive dates: 252 for daily dates (a median gap of up to 4 days, so that
/// weekends do not count), 52 for weekly (up to 10 days), 12 for monthly (up to 45 days), 4 for
/// quarterly (up to 135 days) and 1 otherwise. Fewer than two dates give 252.
pub fn periods_per_year(dates: &[Date]) -> f64 {
    let mut gaps: Vec<i32> = dates.windows(2).map(|w| w[0].days_until(w[1])).collect();
    if gaps.is_empty() {
        return 252.0;
    }
    gaps.sort_unstable();
    let median = gaps[gaps.len() / 2];
    match median {
        ..=4 => 252.0,
        5..=10 => 52.0,
        11..=45 => 12.0,
        46..=135 => 4.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn frequencies() {
        let daily = [
            d("2026-01-02"),
            d("2026-01-05"),
            d("2026-01-06"),
            d("2026-01-07"),
        ];
        assert_eq!(periods_per_year(&daily), 252.0);
        let weekly = [d("2026-01-02"), d("2026-01-09"), d("2026-01-16")];
        assert_eq!(periods_per_year(&weekly), 52.0);
        let monthly = [d("2026-01-31"), d("2026-02-28"), d("2026-03-31")];
        assert_eq!(periods_per_year(&monthly), 12.0);
        let quarterly = [d("2026-03-31"), d("2026-06-30"), d("2026-09-30")];
        assert_eq!(periods_per_year(&quarterly), 4.0);
        let yearly = [d("2024-12-31"), d("2025-12-31"), d("2026-12-31")];
        assert_eq!(periods_per_year(&yearly), 1.0);
    }

    #[test]
    fn returns_skip_dates_with_a_missing_price() {
        let table = PriceTable::new(
            vec![
                d("2026-01-31"),
                d("2026-02-28"),
                d("2026-03-31"),
                d("2026-04-30"),
            ],
            vec!["A".into(), "B".into()],
            vec![
                vec![Some(100.0), Some(110.0), Some(99.0), Some(108.9)],
                vec![Some(50.0), None, Some(55.0), Some(55.0)],
            ],
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let r = table.returns().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.dates, vec![d("2026-03-31"), d("2026-04-30")]);
        let a = &r.table.columns()[0];
        assert!((a[0] - (99.0 / 100.0 - 1.0)).abs() < 1e-15);
        assert!((a[1] - 0.1).abs() < 1e-12);
        assert!(!r.warnings.is_empty());
    }

    #[test]
    fn invalid_tables_list_every_problem() {
        let Err(Error::Validation(report)) = PriceTable::new(
            vec![d("2026-02-28"), d("2026-01-31")],
            vec!["A".into(), "A".into()],
            vec![vec![Some(1.0), Some(-1.0)], vec![Some(1.0)]],
        ) else {
            panic!("expected a validation error");
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(
            paths,
            ["dates[1]", "labels[1]", "columns[0][1]", "columns[1]"]
        );
    }
}
