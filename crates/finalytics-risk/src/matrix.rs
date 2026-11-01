//! Labelled square matrices and tables of returns.

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};

use crate::input::{check_returns, invalid};

/// The returns of several assets over the same periods, one column per asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReturnsTableRepr", into = "ReturnsTableRepr")]
pub struct ReturnsTable {
    labels: Vec<String>,
    columns: Vec<Vec<f64>>,
}

#[derive(Serialize, Deserialize)]
struct ReturnsTableRepr {
    labels: Vec<String>,
    columns: Vec<Vec<f64>>,
}

impl TryFrom<ReturnsTableRepr> for ReturnsTable {
    type Error = Error;

    fn try_from(repr: ReturnsTableRepr) -> Result<ReturnsTable> {
        ReturnsTable::new(repr.labels, repr.columns)
    }
}

impl From<ReturnsTable> for ReturnsTableRepr {
    fn from(table: ReturnsTable) -> ReturnsTableRepr {
        ReturnsTableRepr {
            labels: table.labels,
            columns: table.columns,
        }
    }
}

impl ReturnsTable {
    /// A table with one label per column. Every column must hold the same number of returns, at
    /// least two, each finite and none below -100%.
    pub fn new(labels: Vec<String>, columns: Vec<Vec<f64>>) -> Result<ReturnsTable> {
        let mut report = ValidationReport::new();
        if labels.len() != columns.len() {
            report.error(
                "labels",
                format!("{} labels for {} columns", labels.len(), columns.len()),
            );
        }
        if columns.is_empty() {
            report.error("columns", "needs at least one column");
        }
        let rows = columns.first().map_or(0, Vec::len);
        for (j, column) in columns.iter().enumerate() {
            let name = format!("columns[{j}]");
            if column.len() != rows {
                report.error(
                    &name,
                    format!(
                        "has {} returns, but the first column has {rows}",
                        column.len()
                    ),
                );
            }
            check_returns(&mut report, column, &name, 2);
        }
        report.into_result(ReturnsTable { labels, columns })
    }

    /// The column labels.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The columns of returns, one per label.
    pub fn columns(&self) -> &[Vec<f64>] {
        &self.columns
    }

    /// The number of assets (columns).
    pub fn assets(&self) -> usize {
        self.columns.len()
    }

    /// The number of returns in each column.
    pub fn observations(&self) -> usize {
        self.columns.first().map_or(0, Vec::len)
    }
}

/// A square matrix with labelled rows and columns, stored row by row; a covariance or
/// correlation matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "MatrixRepr", into = "MatrixRepr")]
pub struct Matrix {
    labels: Vec<String>,
    values: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
struct MatrixRepr {
    labels: Vec<String>,
    values: Vec<f64>,
}

impl TryFrom<MatrixRepr> for Matrix {
    type Error = Error;

    fn try_from(repr: MatrixRepr) -> Result<Matrix> {
        Matrix::new(repr.labels, repr.values)
    }
}

impl From<Matrix> for MatrixRepr {
    fn from(matrix: Matrix) -> MatrixRepr {
        MatrixRepr {
            labels: matrix.labels,
            values: matrix.values,
        }
    }
}

impl Matrix {
    /// A matrix with `labels.len()` rows and columns, from its finite values row by row.
    pub fn new(labels: Vec<String>, values: Vec<f64>) -> Result<Matrix> {
        let n = labels.len();
        if values.len() != n * n {
            return Err(invalid(
                "values",
                format!(
                    "{} values for a {n} by {n} matrix, which needs {}",
                    values.len(),
                    n * n
                ),
            ));
        }
        let mut report = ValidationReport::new();
        for (k, value) in values.iter().enumerate() {
            if !value.is_finite() {
                report.error(format!("values[{k}]"), "must be a finite number");
            }
        }
        report.into_result(Matrix { labels, values })
    }

    pub(crate) fn from_parts(labels: Vec<String>, values: Vec<f64>) -> Matrix {
        Matrix { labels, values }
    }

    /// The row and column labels.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The values, row by row.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The number of rows (and of columns).
    pub fn dimension(&self) -> usize {
        self.labels.len()
    }

    /// The value in row `i` and column `j`.
    ///
    /// # Panics
    ///
    /// If `i` or `j` is not below [`Matrix::dimension`].
    pub fn get(&self, i: usize, j: usize) -> f64 {
        let n = self.dimension();
        assert!(
            i < n && j < n,
            "index ({i}, {j}) outside a {n} by {n} matrix"
        );
        self.values[i * n + j]
    }

    /// Row `i`.
    pub fn row(&self, i: usize) -> &[f64] {
        let n = self.dimension();
        &self.values[i * n..(i + 1) * n]
    }

    /// The diagonal: the variances, for a covariance matrix.
    pub fn diagonal(&self) -> Vec<f64> {
        (0..self.dimension()).map(|i| self.get(i, i)).collect()
    }

    /// Every value multiplied by `factor`, for example 252 to annualise a daily covariance.
    pub fn scaled(&self, factor: f64) -> Matrix {
        Matrix {
            labels: self.labels.clone(),
            values: self.values.iter().map(|v| v * factor).collect(),
        }
    }

    /// The product with the vector `x`.
    pub fn times(&self, x: &[f64]) -> Result<Vec<f64>> {
        self.check_vector(x, "weights")?;
        Ok((0..self.dimension())
            .map(|i| self.row(i).iter().zip(x).map(|(a, b)| a * b).sum())
            .collect())
    }

    /// `x' M x`: the variance of a portfolio with weights `x`, for a covariance matrix.
    pub fn quadratic_form(&self, x: &[f64]) -> Result<f64> {
        Ok(self.times(x)?.iter().zip(x).map(|(a, b)| a * b).sum())
    }

    /// The correlation matrix of a covariance matrix. Every variance must be greater than zero.
    pub fn correlation(&self) -> Result<Matrix> {
        let sd: Vec<f64> = self.diagonal().iter().map(|v| v.sqrt()).collect();
        let mut report = ValidationReport::new();
        for (i, s) in sd.iter().enumerate() {
            if s.is_nan() || *s <= 0.0 {
                report.error(
                    format!("labels[{i}]"),
                    format!(
                        "{} has no variance, so its correlations are undefined",
                        self.labels[i]
                    ),
                );
            }
        }
        if report.has_errors() {
            return Err(Error::Validation(report));
        }
        let n = self.dimension();
        let mut values = Vec::with_capacity(n * n);
        for i in 0..n {
            for j in 0..n {
                values.push(if i == j {
                    1.0
                } else {
                    (self.get(i, j) / (sd[i] * sd[j])).clamp(-1.0, 1.0)
                });
            }
        }
        Ok(Matrix {
            labels: self.labels.clone(),
            values,
        })
    }

    pub(crate) fn check_vector(&self, x: &[f64], name: &str) -> Result<()> {
        if x.len() != self.dimension() {
            return Err(invalid(
                name,
                format!(
                    "has {} entries for a matrix of dimension {}",
                    x.len(),
                    self.dimension()
                ),
            ));
        }
        let mut report = ValidationReport::new();
        for (k, value) in x.iter().enumerate() {
            if !value.is_finite() {
                report.error(format!("{name}[{k}]"), "must be a finite number");
            }
        }
        report.into_result(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("a{i}")).collect()
    }

    #[test]
    fn products_and_correlation() {
        let m = Matrix::new(labels(2), vec![0.04, 0.006, 0.006, 0.09]).unwrap();
        let product = m.times(&[1.0, 2.0]).unwrap();
        assert!((product[0] - 0.052).abs() < 1e-15 && (product[1] - 0.186).abs() < 1e-15);
        assert!((m.quadratic_form(&[0.5, 0.5]).unwrap() - 0.0355).abs() < 1e-15);
        let c = m.correlation().unwrap();
        assert!((c.get(0, 1) - 0.1).abs() < 1e-15);
        assert_eq!(c.get(1, 1), 1.0);
        assert!(m.times(&[1.0]).is_err());

        let cash = Matrix::new(labels(2), vec![0.0, 0.0, 0.0, 0.09]).unwrap();
        assert!(cash.correlation().is_err());
    }

    #[test]
    fn tables_and_matrices_are_validated() {
        assert!(ReturnsTable::new(labels(2), vec![vec![0.1, 0.2], vec![0.1]]).is_err());
        assert!(ReturnsTable::new(labels(1), vec![vec![0.1, f64::NAN]]).is_err());
        assert!(ReturnsTable::new(labels(1), vec![]).is_err());
        assert!(Matrix::new(labels(2), vec![1.0; 3]).is_err());
        let table: ReturnsTable =
            serde_json::from_str(r#"{"labels":["a"],"columns":[[0.01,0.02]]}"#).unwrap();
        assert_eq!(table.observations(), 2);
        assert!(
            serde_json::from_str::<ReturnsTable>(r#"{"labels":["a"],"columns":[[0.01]]}"#).is_err()
        );
    }
}
