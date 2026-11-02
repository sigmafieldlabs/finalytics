//! Factor models of asset returns: estimating them, and splitting a portfolio's risk by factor.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};

use crate::covariance::{CovarianceMethod, covariance};
use crate::decomposition::RiskContribution;
use crate::input::invalid;
use crate::linalg::{LeastSquares, symmetric_eigen};
use crate::matrix::{Matrix, ReturnsTable};

/// A linear factor model of asset returns, `r = B f + e`: each asset's exposures `B` to a set of
/// factors, the covariance `F` of the factor returns, and the variance `d` of each asset's
/// specific return `e`, which is uncorrelated across assets and with the factors.
///
/// The model implies the covariance matrix `B F B' + diag(d)` for the assets. The factor
/// covariance and the specific variances share units, for example both daily or both
/// annualised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "FactorModelRepr", into = "FactorModelRepr")]
#[non_exhaustive]
pub struct FactorModel {
    assets: Vec<String>,
    exposures: Vec<Vec<f64>>,
    factor_covariance: Matrix,
    specific_variances: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct FactorModelRepr {
    /// The asset labels.
    assets: Vec<String>,
    /// Each asset's exposure to each factor: one row per asset, one entry per factor, in the
    /// order of the factor covariance matrix's labels.
    exposures: Vec<Vec<f64>>,
    /// The covariance matrix of the factor returns, labelled with the factor names.
    factor_covariance: Matrix,
    /// The variance of each asset's specific return, one per asset.
    specific_variances: Vec<f64>,
}

impl TryFrom<FactorModelRepr> for FactorModel {
    type Error = Error;

    fn try_from(repr: FactorModelRepr) -> Result<FactorModel> {
        FactorModel::new(
            repr.assets,
            repr.exposures,
            repr.factor_covariance,
            repr.specific_variances,
        )
    }
}

impl From<FactorModel> for FactorModelRepr {
    fn from(model: FactorModel) -> FactorModelRepr {
        FactorModelRepr {
            assets: model.assets,
            exposures: model.exposures,
            factor_covariance: model.factor_covariance,
            specific_variances: model.specific_variances,
        }
    }
}

impl FactorModel {
    /// A model from its parts: the asset labels, one row of exposures per asset (one entry per
    /// factor), the factor covariance matrix (labelled with the factor names) and one specific
    /// variance per asset.
    ///
    /// The factor covariance must be symmetric and positive semidefinite, and the specific
    /// variances must not be negative. This is how a fundamental model with exposures from
    /// elsewhere (industries, countries, styles) is supplied; [`fundamental_model`] estimates the
    /// factor returns and specific variances for given exposures.
    pub fn new(
        assets: Vec<String>,
        exposures: Vec<Vec<f64>>,
        factor_covariance: Matrix,
        specific_variances: Vec<f64>,
    ) -> Result<FactorModel> {
        let mut report = ValidationReport::new();
        let n = assets.len();
        let k = factor_covariance.dimension();
        if n == 0 {
            report.error("assets", "needs at least one asset");
        }
        unique(&assets, "assets", &mut report);
        if k == 0 {
            report.error("factor_covariance", "needs at least one factor");
        }
        unique(
            factor_covariance.labels(),
            "factor_covariance.labels",
            &mut report,
        );
        check_covariance(&factor_covariance, "factor_covariance", &mut report);
        if exposures.len() != n {
            report.error(
                "exposures",
                format!("has {} rows for {n} assets", exposures.len()),
            );
        }
        for (i, row) in exposures.iter().enumerate() {
            if row.len() != k {
                report.error(
                    format!("exposures[{i}]"),
                    format!("has {} entries for {k} factors", row.len()),
                );
            }
            for (j, x) in row.iter().enumerate() {
                if !x.is_finite() {
                    report.error(format!("exposures[{i}][{j}]"), "must be a finite number");
                }
            }
        }
        if specific_variances.len() != n {
            report.error(
                "specific_variances",
                format!("has {} entries for {n} assets", specific_variances.len()),
            );
        }
        for (i, d) in specific_variances.iter().enumerate() {
            if !(d.is_finite() && *d >= 0.0) {
                report.error(
                    format!("specific_variances[{i}]"),
                    "must be a finite number, not negative",
                );
            }
        }
        report.into_result(FactorModel {
            assets,
            exposures,
            factor_covariance,
            specific_variances,
        })
    }

    /// The asset labels.
    pub fn assets(&self) -> &[String] {
        &self.assets
    }

    /// The factor names, the labels of the factor covariance matrix.
    pub fn factors(&self) -> &[String] {
        self.factor_covariance.labels()
    }

    /// Each asset's exposures, one row per asset and one entry per factor.
    pub fn exposures(&self) -> &[Vec<f64>] {
        &self.exposures
    }

    /// The covariance matrix of the factor returns.
    pub fn factor_covariance(&self) -> &Matrix {
        &self.factor_covariance
    }

    /// The variance of each asset's specific return.
    pub fn specific_variances(&self) -> &[f64] {
        &self.specific_variances
    }

    /// The model with the factor covariance and the specific variances multiplied by `factor`,
    /// for example 252 to annualise a daily model. The exposures are unchanged.
    pub fn scaled(&self, factor: f64) -> FactorModel {
        FactorModel {
            assets: self.assets.clone(),
            exposures: self.exposures.clone(),
            factor_covariance: self.factor_covariance.scaled(factor),
            specific_variances: self.specific_variances.iter().map(|d| d * factor).collect(),
        }
    }

    /// The covariance matrix of the assets the model implies, `B F B' + diag(d)`.
    pub fn covariance(&self) -> Matrix {
        let n = self.assets.len();
        let loaded: Vec<Vec<f64>> = self.exposures.iter().map(|b| self.times_f(b)).collect();
        let mut values = vec![0.0; n * n];
        for i in 0..n {
            for j in i..n {
                let mut v: f64 = loaded[i]
                    .iter()
                    .zip(&self.exposures[j])
                    .map(|(a, b)| a * b)
                    .sum();
                if i == j {
                    v += self.specific_variances[i];
                }
                values[i * n + j] = v;
                values[j * n + i] = v;
            }
        }
        Matrix::from_parts(self.assets.clone(), values)
    }

    /// The exposures of a portfolio with `weights` (one per asset) to each factor, `B' w`.
    pub fn portfolio_exposures(&self, weights: &[f64]) -> Result<Vec<f64>> {
        self.check_weights(weights, "weights")?;
        Ok(self.exposures_of(weights))
    }

    /// The variance of a portfolio with `weights`, `x' F x + sum d w^2` with `x = B' w`.
    pub fn variance(&self, weights: &[f64]) -> Result<f64> {
        self.check_weights(weights, "weights")?;
        let x = self.exposures_of(weights);
        let fx = self.times_f(&x);
        let systematic: f64 = x.iter().zip(&fx).map(|(a, b)| a * b).sum();
        Ok(systematic.max(0.0) + self.specific_of(weights))
    }

    pub(crate) fn exposures_of(&self, weights: &[f64]) -> Vec<f64> {
        let mut x = vec![0.0; self.factors().len()];
        for (row, w) in self.exposures.iter().zip(weights) {
            for (xk, b) in x.iter_mut().zip(row) {
                *xk += b * w;
            }
        }
        x
    }

    fn times_f(&self, x: &[f64]) -> Vec<f64> {
        let k = self.factors().len();
        (0..k)
            .map(|i| {
                self.factor_covariance
                    .row(i)
                    .iter()
                    .zip(x)
                    .map(|(a, b)| a * b)
                    .sum()
            })
            .collect()
    }

    fn specific_of(&self, weights: &[f64]) -> f64 {
        weights
            .iter()
            .zip(&self.specific_variances)
            .map(|(w, d)| d * w * w)
            .sum()
    }

    fn check_weights(&self, weights: &[f64], name: &str) -> Result<()> {
        let mut report = ValidationReport::new();
        if weights.len() != self.assets.len() {
            report.error(
                name,
                format!(
                    "has {} entries for {} assets",
                    weights.len(),
                    self.assets.len()
                ),
            );
        }
        for (i, w) in weights.iter().enumerate() {
            if !w.is_finite() {
                report.error(format!("{name}[{i}]"), "must be a finite number");
            }
        }
        report.into_result(())
    }
}

fn unique(labels: &[String], name: &str, report: &mut ValidationReport) {
    let mut seen = HashSet::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        if !seen.insert(label) {
            report.error(format!("{name}[{i}]"), format!("{label} is listed twice"));
        }
    }
}

/// Checks that a matrix is symmetric and positive semidefinite, up to rounding.
fn check_covariance(matrix: &Matrix, name: &str, report: &mut ValidationReport) {
    if matrix.values().iter().any(|v| !v.is_finite()) {
        report.error(name, "must hold finite numbers");
        return;
    }
    let n = matrix.dimension();
    let scale = matrix.values().iter().fold(0.0f64, |m, v| m.max(v.abs()));
    for i in 0..n {
        for j in i + 1..n {
            if (matrix.get(i, j) - matrix.get(j, i)).abs() > 1e-12 * scale {
                report.error(
                    name,
                    format!("is not symmetric: entries ({i}, {j}) and ({j}, {i}) differ"),
                );
                return;
            }
        }
    }
    match symmetric_eigen(matrix.values(), n) {
        Some((values, _)) => {
            if values.last().is_some_and(|v| *v < -1e-10 * scale) {
                report.error(name, "is not positive semidefinite");
            }
        }
        None => report.error(name, "could not be checked for positive semidefiniteness"),
    }
}

/// How a factor model was estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FactorMethod {
    /// Time-series regression of each asset's returns on given factor returns.
    Regression,
    /// Principal components of the sample covariance matrix.
    Statistical,
    /// Cross-sectional regression of each period's returns on given exposures.
    Fundamental,
}

/// How well a factor model explains one asset's returns over the estimation period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AssetFit {
    /// The asset's label.
    pub asset: String,
    /// The intercept: the regression's alpha, or the mean return for a statistical model;
    /// absent for a fundamental model.
    pub intercept: Option<f64>,
    /// The share of the asset's return variance the factors explain. Absent when the asset's
    /// returns do not vary.
    pub r_squared: Option<f64>,
}

/// A factor model estimated from returns, with how well it fits each asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FactorModelEstimate {
    /// How the model was estimated.
    pub method: FactorMethod,
    /// The model, in the units of the returns (per period).
    pub model: FactorModel,
    /// The fit for each asset, in the model's order.
    pub fit: Vec<AssetFit>,
    /// The factor returns over the estimation period, one column per factor: the given factor
    /// returns for a regression model, the principal component scores for a statistical model,
    /// and the estimated factor returns for a fundamental model.
    pub factor_returns: Vec<Vec<f64>>,
    /// For a statistical model, each component's share of the total variance.
    pub explained_variance: Option<Vec<f64>>,
    /// The number of periods of returns used.
    pub observations: usize,
}

/// Estimates a factor model by regressing each asset's returns on given factor returns, with an
/// intercept (a time-series or macroeconomic model).
///
/// The exposures are the regression coefficients, the factor covariance is the sample covariance
/// of the factor returns, and each specific variance is the residual variance with `T - K - 1`
/// degrees of freedom. `returns` and `factor_returns` must cover the same periods, with more
/// periods than factors plus one.
///
/// ```
/// use finalytics_risk::{ReturnsTable, regression_model};
///
/// let market = vec![0.010, -0.020, 0.015, 0.005, -0.010, 0.020];
/// let asset: Vec<f64> = market.iter().enumerate()
///     .map(|(t, m)| 0.001 + 1.2 * m + if t % 2 == 0 { 0.002 } else { -0.002 })
///     .collect();
/// let returns = ReturnsTable::new(vec!["asset".into()], vec![asset])?;
/// let factors = ReturnsTable::new(vec!["market".into()], vec![market])?;
/// let estimate = regression_model(&returns, &factors)?;
/// let beta = estimate.model.exposures()[0][0];
/// assert!((beta - 1.2).abs() < 0.1);
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn regression_model(
    returns: &ReturnsTable,
    factor_returns: &ReturnsTable,
) -> Result<FactorModelEstimate> {
    let t = returns.observations();
    let k = factor_returns.assets();
    let mut report = ValidationReport::new();
    if factor_returns.observations() != t {
        report.error(
            "factor_returns",
            format!(
                "has {} periods, but the returns have {t}",
                factor_returns.observations()
            ),
        );
    } else if t <= k + 1 {
        report.error(
            "returns",
            format!("needs more than {} periods for {k} factors", k + 1),
        );
    }
    unique(
        factor_returns.labels(),
        "factor_returns.labels",
        &mut report,
    );
    if report.has_errors() {
        return Err(Error::Validation(report));
    }
    let mut design = vec![vec![1.0; t]];
    design.extend(factor_returns.columns().iter().cloned());
    let solver = LeastSquares::new(&design).map_err(|j| match j.checked_sub(1) {
        Some(k) => invalid(
            &format!("factor_returns.columns[{k}]"),
            format!(
                "{} is constant or a combination of the other factors",
                factor_returns.labels()[k]
            ),
        ),
        None => invalid("factor_returns", "leave no room for an intercept"),
    })?;
    let dof = (t - k - 1) as f64;
    let mut exposures = Vec::with_capacity(returns.assets());
    let mut specific = Vec::with_capacity(returns.assets());
    let mut fit = Vec::with_capacity(returns.assets());
    for (label, y) in returns.labels().iter().zip(returns.columns()) {
        let (b, residuals) = solver.solve(y);
        let ssr: f64 = residuals.iter().map(|e| e * e).sum();
        fit.push(AssetFit {
            asset: label.clone(),
            intercept: Some(b[0]),
            r_squared: r_squared(ssr, y),
        });
        exposures.push(b[1..].to_vec());
        specific.push(ssr / dof);
    }
    let factor_covariance = covariance(factor_returns, CovarianceMethod::Sample)?.matrix;
    Ok(FactorModelEstimate {
        method: FactorMethod::Regression,
        model: estimated(returns, exposures, factor_covariance, specific)?,
        fit,
        factor_returns: factor_returns.columns().to_vec(),
        explained_variance: None,
        observations: t,
    })
}

/// Estimates a statistical factor model from the first `factors` principal components of the
/// sample covariance matrix `S` (divisor `T - 1`).
///
/// The exposures are the eigenvectors, the factors are uncorrelated with the eigenvalues as their
/// variances, and each specific variance is what the components leave of the asset's variance, so
/// the model reproduces the sample variances exactly. Factors are named `pc1`, `pc2` and so on;
/// each eigenvector's largest loading is positive. `factors` is between 1 and the smaller of the
/// number of assets and `T - 1`.
pub fn statistical_model(returns: &ReturnsTable, factors: usize) -> Result<FactorModelEstimate> {
    let n = returns.assets();
    let t = returns.observations();
    let largest = n.min(t - 1);
    if factors == 0 || factors > largest {
        return Err(invalid(
            "factors",
            format!("must be between 1 and {largest} for {n} assets and {t} periods"),
        ));
    }
    let sample = covariance(returns, CovarianceMethod::Sample)?.matrix;
    let total: f64 = sample.diagonal().iter().sum();
    if total <= 0.0 {
        return Err(invalid(
            "returns",
            "do not vary, so they have no components",
        ));
    }
    let (values, vectors) = symmetric_eigen(sample.values(), n)
        .ok_or_else(|| invalid("returns", "the eigendecomposition did not converge"))?;
    let values: Vec<f64> = values[..factors].iter().map(|v| v.max(0.0)).collect();
    let vectors = &vectors[..factors];
    let names: Vec<String> = (1..=factors).map(|k| format!("pc{k}")).collect();
    let mut f = vec![0.0; factors * factors];
    for (k, v) in values.iter().enumerate() {
        f[k * factors + k] = *v;
    }
    let exposures: Vec<Vec<f64>> = (0..n)
        .map(|i| vectors.iter().map(|v| v[i]).collect())
        .collect();
    let variances = sample.diagonal();
    let specific: Vec<f64> = (0..n)
        .map(|i| {
            let explained: f64 = values
                .iter()
                .zip(vectors)
                .map(|(l, v)| l * v[i] * v[i])
                .sum();
            (variances[i] - explained).max(0.0)
        })
        .collect();
    let means: Vec<f64> = returns
        .columns()
        .iter()
        .map(|c| c.iter().sum::<f64>() / t as f64)
        .collect();
    let scores: Vec<Vec<f64>> = vectors
        .iter()
        .map(|v| {
            (0..t)
                .map(|s| {
                    returns
                        .columns()
                        .iter()
                        .zip(&means)
                        .zip(v)
                        .map(|((c, m), vi)| (c[s] - m) * vi)
                        .sum()
                })
                .collect()
        })
        .collect();
    let fit = returns
        .labels()
        .iter()
        .enumerate()
        .map(|(i, label)| AssetFit {
            asset: label.clone(),
            intercept: Some(means[i]),
            r_squared: (variances[i] > 0.0).then(|| 1.0 - specific[i] / variances[i]),
        })
        .collect();
    Ok(FactorModelEstimate {
        method: FactorMethod::Statistical,
        model: estimated(returns, exposures, Matrix::from_parts(names, f), specific)?,
        fit,
        factor_returns: scores,
        explained_variance: Some(values.iter().map(|v| v / total).collect()),
        observations: t,
    })
}

/// Estimates a fundamental factor model for given exposures: each period's asset returns are
/// regressed on the exposures (ordinary least squares, without an intercept), giving that
/// period's factor returns and each asset's specific return.
///
/// `exposures` has one row per asset of `returns` and one entry per name in `factors`; the
/// exposures are taken as constant over the period. Include a column of ones for a market or
/// intercept factor. The factor covariance is the sample covariance of the estimated factor
/// returns, and each specific variance the sample variance of the asset's specific returns
/// (divisor `T - 1` for both). There must be more assets than factors.
pub fn fundamental_model(
    returns: &ReturnsTable,
    factors: &[String],
    exposures: &[Vec<f64>],
) -> Result<FactorModelEstimate> {
    let n = returns.assets();
    let t = returns.observations();
    let k = factors.len();
    let mut report = ValidationReport::new();
    if k == 0 {
        report.error("factors", "needs at least one factor");
    } else if n <= k {
        report.error(
            "returns",
            format!("needs more than {k} assets for {k} factors"),
        );
    }
    unique(factors, "factors", &mut report);
    if exposures.len() != n {
        report.error(
            "exposures",
            format!("has {} rows for {n} assets", exposures.len()),
        );
    }
    for (i, row) in exposures.iter().enumerate() {
        if row.len() != k {
            report.error(
                format!("exposures[{i}]"),
                format!("has {} entries for {k} factors", row.len()),
            );
        }
        for (j, x) in row.iter().enumerate() {
            if !x.is_finite() {
                report.error(format!("exposures[{i}][{j}]"), "must be a finite number");
            }
        }
    }
    if report.has_errors() {
        return Err(Error::Validation(report));
    }
    let design: Vec<Vec<f64>> = (0..k)
        .map(|j| exposures.iter().map(|row| row[j]).collect())
        .collect();
    let solver = LeastSquares::new(&design).map_err(|j| {
        invalid(
            "exposures",
            format!(
                "the exposures to {} are zero or a combination of the other factors'",
                factors[j]
            ),
        )
    })?;
    let mut factor_returns = vec![Vec::with_capacity(t); k];
    let mut residuals = vec![Vec::with_capacity(t); n];
    for s in 0..t {
        let y: Vec<f64> = returns.columns().iter().map(|c| c[s]).collect();
        let (f, e) = solver.solve(&y);
        for (column, value) in factor_returns.iter_mut().zip(f) {
            column.push(value);
        }
        for (column, value) in residuals.iter_mut().zip(e) {
            column.push(value);
        }
    }
    let factor_covariance =
        Matrix::from_parts(factors.to_vec(), sample_covariance(&factor_returns));
    let specific: Vec<f64> = residuals.iter().map(|e| sample_variance(e)).collect();
    let fit = returns
        .labels()
        .iter()
        .zip(returns.columns())
        .zip(&specific)
        .map(|((label, r), d)| {
            let total = sample_variance(r);
            AssetFit {
                asset: label.clone(),
                intercept: None,
                r_squared: (total > 0.0).then(|| 1.0 - d / total),
            }
        })
        .collect();
    Ok(FactorModelEstimate {
        method: FactorMethod::Fundamental,
        model: estimated(returns, exposures.to_vec(), factor_covariance, specific)?,
        fit,
        factor_returns,
        explained_variance: None,
        observations: t,
    })
}

/// The estimated model, checked like any other; estimates that overflow are reported against the
/// returns, as the model's own paths would mean nothing to the caller.
fn estimated(
    returns: &ReturnsTable,
    exposures: Vec<Vec<f64>>,
    factor_covariance: Matrix,
    specific_variances: Vec<f64>,
) -> Result<FactorModel> {
    FactorModel::new(
        returns.labels().to_vec(),
        exposures,
        factor_covariance,
        specific_variances,
    )
    .map_err(|_| {
        invalid(
            "returns",
            "are too large for the estimates to be computed in floating point",
        )
    })
}

fn r_squared(ssr: f64, y: &[f64]) -> Option<f64> {
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let sst: f64 = y.iter().map(|v| (v - mean) * (v - mean)).sum();
    (sst > 0.0).then(|| 1.0 - ssr / sst)
}

fn sample_variance(x: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mean = x.iter().sum::<f64>() / n;
    x.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0)
}

/// The sample covariance (divisor `T - 1`) of columns of equal length, row by row.
fn sample_covariance(columns: &[Vec<f64>]) -> Vec<f64> {
    let k = columns.len();
    let t = columns.first().map_or(0, Vec::len) as f64;
    let centred: Vec<Vec<f64>> = columns
        .iter()
        .map(|c| {
            let mean = c.iter().sum::<f64>() / t;
            c.iter().map(|v| v - mean).collect()
        })
        .collect();
    let mut values = vec![0.0; k * k];
    for i in 0..k {
        for j in i..k {
            let v = centred[i]
                .iter()
                .zip(&centred[j])
                .map(|(a, b)| a * b)
                .sum::<f64>()
                / (t - 1.0);
            values[i * k + j] = v;
            values[j * k + i] = v;
        }
    }
    values
}

/// One factor's share of a portfolio's volatility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FactorContribution {
    /// The factor's name.
    pub factor: String,
    /// The portfolio's exposure to the factor, `(B' w)_k`.
    pub exposure: f64,
    /// The change in volatility per unit of exposure, `(F x)_k / volatility`. Absent when the
    /// portfolio has no volatility.
    pub marginal: Option<f64>,
    /// `exposure * marginal`. With the specific component, the components add up to the
    /// volatility.
    pub component: f64,
    /// `component / volatility`. Absent when the portfolio has no volatility.
    pub share: Option<f64>,
}

/// A portfolio's volatility under a factor model, split into each factor's contribution and the
/// specific (stock-specific) part, and into each asset's contribution (Euler allocation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct FactorRisk {
    /// The volatility, `sqrt(x' F x + sum d w^2)`, in the units of the model.
    pub volatility: f64,
    /// The volatility from the factors alone, `sqrt(x' F x)`.
    pub factor_volatility: f64,
    /// The volatility from specific returns alone, `sqrt(sum d w^2)`.
    pub specific_volatility: f64,
    /// Each factor's contribution, in the model's order.
    pub factors: Vec<FactorContribution>,
    /// The specific part's contribution, `sum d w^2 / volatility`.
    pub specific_component: f64,
    /// `specific_component / volatility`. Absent when the portfolio has no volatility.
    pub specific_share: Option<f64>,
    /// Each asset's contribution under the model's covariance matrix, in the model's order.
    pub assets: Vec<RiskContribution>,
}

/// Splits the volatility of a portfolio with `weights` (one per asset of `model`) by factor and
/// by asset.
///
/// The factor components and the specific component add up to the volatility, as do the asset
/// components. To split the ex-ante tracking error instead, use [`factor_tracking_error`].
///
/// ```
/// use finalytics_risk::{FactorModel, Matrix, factor_risk};
///
/// let model = FactorModel::new(
///     vec!["a".into(), "b".into()],
///     vec![vec![1.1], vec![0.8]],
///     Matrix::new(vec!["market".into()], vec![0.04])?,
///     vec![0.01, 0.02],
/// )?;
/// let risk = factor_risk(&[0.5, 0.5], &model)?;
/// let total = risk.factors[0].component + risk.specific_component;
/// assert!((total - risk.volatility).abs() < 1e-15);
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn factor_risk(weights: &[f64], model: &FactorModel) -> Result<FactorRisk> {
    model.check_weights(weights, "weights")?;
    Ok(decompose(weights, model))
}

/// Splits the ex-ante tracking error of a portfolio with `weights` against `benchmark` weights by
/// factor and by asset: the decomposition of the active weights `weights - benchmark`, whose
/// factor exposures are the active exposures.
pub fn factor_tracking_error(
    weights: &[f64],
    benchmark: &[f64],
    model: &FactorModel,
) -> Result<FactorRisk> {
    model.check_weights(weights, "weights")?;
    model.check_weights(benchmark, "benchmark")?;
    let active: Vec<f64> = weights.iter().zip(benchmark).map(|(w, b)| w - b).collect();
    Ok(decompose(&active, model))
}

fn decompose(weights: &[f64], model: &FactorModel) -> FactorRisk {
    let x = model.exposures_of(weights);
    let fx = model.times_f(&x);
    let systematic: f64 = x.iter().zip(&fx).map(|(a, b)| a * b).sum::<f64>().max(0.0);
    let specific = model.specific_of(weights);
    let volatility = (systematic + specific).sqrt();
    let positive = volatility > 0.0;
    let factors = model
        .factors()
        .iter()
        .zip(x.iter().zip(&fx))
        .map(|(name, (xk, fxk))| {
            let marginal = positive.then(|| fxk / volatility);
            FactorContribution {
                factor: name.clone(),
                exposure: *xk,
                marginal,
                component: marginal.map_or(0.0, |m| xk * m),
                share: positive.then(|| xk * fxk / (volatility * volatility)),
            }
        })
        .collect();
    let assets = model
        .assets
        .iter()
        .zip(weights)
        .zip(model.exposures.iter().zip(&model.specific_variances))
        .map(|((label, w), (row, d))| {
            let sigma_w: f64 = row.iter().zip(&fx).map(|(b, f)| b * f).sum::<f64>() + d * w;
            let marginal = positive.then(|| sigma_w / volatility);
            RiskContribution {
                label: label.clone(),
                weight: *w,
                marginal,
                component: marginal.map_or(0.0, |m| w * m),
                share: positive.then(|| w * sigma_w / (volatility * volatility)),
            }
        })
        .collect();
    FactorRisk {
        volatility,
        factor_volatility: systematic.sqrt(),
        specific_volatility: specific.sqrt(),
        factors,
        specific_component: if positive { specific / volatility } else { 0.0 },
        specific_share: positive.then(|| specific / (volatility * volatility)),
        assets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> FactorModel {
        FactorModel::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec![vec![1.0, 0.5], vec![0.8, -0.3], vec![1.2, 0.0]],
            Matrix::new(
                vec!["market".into(), "value".into()],
                vec![0.04, 0.002, 0.002, 0.01],
            )
            .unwrap(),
            vec![0.01, 0.02, 0.015],
        )
        .unwrap()
    }

    #[test]
    fn covariance_and_variance_agree() {
        let m = model();
        let w = [0.5, 0.3, 0.2];
        let dense = m.covariance().quadratic_form(&w).unwrap();
        assert!((dense - m.variance(&w).unwrap()).abs() < 1e-17);
    }

    #[test]
    fn contributions_add_up() {
        let m = model();
        let risk = factor_risk(&[0.5, 0.3, 0.2], &m).unwrap();
        let by_factor: f64 =
            risk.factors.iter().map(|f| f.component).sum::<f64>() + risk.specific_component;
        let by_asset: f64 = risk.assets.iter().map(|a| a.component).sum();
        assert!((by_factor - risk.volatility).abs() < 1e-15);
        assert!((by_asset - risk.volatility).abs() < 1e-15);
        let shares: f64 =
            risk.factors.iter().filter_map(|f| f.share).sum::<f64>() + risk.specific_share.unwrap();
        assert!((shares - 1.0).abs() < 1e-14);
        let te = factor_tracking_error(&[0.5, 0.3, 0.2], &[0.5, 0.3, 0.2], &m).unwrap();
        assert_eq!(te.volatility, 0.0);
        assert!(te.factors.iter().all(|f| f.share.is_none()));
    }

    #[test]
    fn invalid_models_are_reported() {
        let bad = FactorModel::new(
            vec!["a".into(), "a".into()],
            vec![vec![1.0]],
            Matrix::new(vec!["m".into(), "v".into()], vec![1.0, 2.0, 2.0, 1.0]).unwrap(),
            vec![-0.1, 0.0],
        );
        let Err(Error::Validation(report)) = bad else {
            panic!("expected a validation error")
        };
        let paths: Vec<&str> = report.issues().iter().map(|i| i.path.as_str()).collect();
        for path in [
            "assets[1]",
            "factor_covariance",
            "exposures",
            "exposures[0]",
            "specific_variances[0]",
        ] {
            assert!(paths.contains(&path), "{path} missing from {paths:?}");
        }
    }

    #[test]
    fn statistical_model_reproduces_variances() {
        let table = ReturnsTable::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec![
                vec![0.01, -0.02, 0.015, 0.003, -0.01],
                vec![0.012, -0.018, 0.01, 0.0, -0.008],
                vec![-0.005, 0.01, 0.002, 0.004, 0.001],
            ],
        )
        .unwrap();
        let estimate = statistical_model(&table, 2).unwrap();
        let sample = covariance(&table, CovarianceMethod::Sample).unwrap().matrix;
        let implied = estimate.model.covariance();
        for i in 0..3 {
            assert!((implied.get(i, i) - sample.get(i, i)).abs() < 1e-16);
        }
        let all = statistical_model(&table, 3).unwrap().model.covariance();
        for (a, b) in all.values().iter().zip(sample.values()) {
            assert!((a - b).abs() < 1e-16);
        }
        assert!(statistical_model(&table, 5).is_err());
    }
}
