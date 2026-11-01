//! Covariance estimators.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::input::invalid;
use crate::matrix::{Matrix, ReturnsTable};

/// How a covariance matrix is estimated from returns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum CovarianceMethod {
    /// The sample covariance, with divisor n - 1.
    Sample,
    /// An exponentially weighted covariance: the return `k` periods before the latest has
    /// weight proportional to `decay^k`, the weights sum to one, and deviations are taken from
    /// the weighted mean. A decay of 0.94 for daily returns follows RiskMetrics.
    Ewma {
        /// The decay factor, between 0 and 1.
        decay: f64,
    },
    /// The empirical covariance (divisor n) shrunk towards a multiple of the identity with the
    /// intensity of Ledoit and Wolf (2004).
    LedoitWolf,
    /// The empirical covariance (divisor n) shrunk towards a multiple of the identity with the
    /// oracle approximating shrinkage intensity of Chen, Wiesel, Eldar and Hero (2010).
    Oas,
}

/// An estimated covariance matrix, per period of the returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CovarianceEstimate {
    /// The estimator used.
    pub method: CovarianceMethod,
    /// The covariance matrix, labelled like the returns. Scale it to annualise, for example
    /// with [`Matrix::scaled`] by 252 for daily returns.
    pub matrix: Matrix,
    /// The shrinkage intensity, between 0 and 1, for the shrinkage estimators.
    pub shrinkage: Option<f64>,
    /// The number of returns per asset used.
    pub observations: usize,
}

/// Estimates the covariance matrix of the assets in `returns`.
///
/// ```
/// use finalytics_risk::{CovarianceMethod, ReturnsTable, covariance};
///
/// let returns = ReturnsTable::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![vec![0.012, -0.008, 0.004, 0.010], vec![-0.002, 0.003, 0.001, -0.001]],
/// )?;
/// let estimate = covariance(&returns, CovarianceMethod::LedoitWolf)?;
/// let annual = estimate.matrix.scaled(252.0);
/// assert!(annual.get(0, 0) > annual.get(1, 1));
/// # Ok::<(), finalytics_core::Error>(())
/// ```
pub fn covariance(returns: &ReturnsTable, method: CovarianceMethod) -> Result<CovarianceEstimate> {
    let t = returns.observations();
    let n = returns.assets();
    let (values, shrinkage) = match method {
        CovarianceMethod::Sample => {
            let centred = centred(returns.columns(), None);
            (cross_products(&centred, None, (t - 1) as f64), None)
        }
        CovarianceMethod::Ewma { decay } => {
            if !(decay > 0.0 && decay < 1.0) {
                return Err(invalid(
                    "decay",
                    "must be between 0 and 1, for example 0.94",
                ));
            }
            let mut weights: Vec<f64> = (0..t).map(|i| decay.powi((t - 1 - i) as i32)).collect();
            let total: f64 = weights.iter().sum();
            weights.iter_mut().for_each(|w| *w /= total);
            let centred = centred(returns.columns(), Some(&weights));
            (cross_products(&centred, Some(&weights), 1.0), None)
        }
        CovarianceMethod::LedoitWolf => {
            let centred = centred(returns.columns(), None);
            let empirical = cross_products(&centred, None, t as f64);
            let shrinkage = ledoit_wolf(&centred, &empirical, n, t);
            (shrink(empirical, n, shrinkage), Some(shrinkage))
        }
        CovarianceMethod::Oas => {
            let centred = centred(returns.columns(), None);
            let empirical = cross_products(&centred, None, t as f64);
            let shrinkage = oas(&empirical, n, t);
            (shrink(empirical, n, shrinkage), Some(shrinkage))
        }
    };
    Ok(CovarianceEstimate {
        method,
        matrix: Matrix::from_parts(returns.labels().to_vec(), values),
        shrinkage,
        observations: t,
    })
}

/// Each column less its mean (weighted, if weights are given).
fn centred(columns: &[Vec<f64>], weights: Option<&[f64]>) -> Vec<Vec<f64>> {
    columns
        .iter()
        .map(|column| {
            let mean = match weights {
                Some(w) => column.iter().zip(w).map(|(x, w)| x * w).sum::<f64>(),
                None => {
                    finalytics_core::numeric::compensated_sum(column.iter().copied())
                        / column.len() as f64
                }
            };
            column.iter().map(|x| x - mean).collect()
        })
        .collect()
}

/// The matrix of `sum_t w_t x_ti x_tj / divisor`, row by row.
fn cross_products(centred: &[Vec<f64>], weights: Option<&[f64]>, divisor: f64) -> Vec<f64> {
    let n = centred.len();
    let mut values = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let sum: f64 = match weights {
                Some(w) => centred[i]
                    .iter()
                    .zip(&centred[j])
                    .zip(w)
                    .map(|((a, b), w)| a * b * w)
                    .sum(),
                None => centred[i].iter().zip(&centred[j]).map(|(a, b)| a * b).sum(),
            };
            values[i * n + j] = sum / divisor;
            values[j * n + i] = sum / divisor;
        }
    }
    values
}

fn trace(values: &[f64], n: usize) -> f64 {
    (0..n).map(|i| values[i * n + i]).sum()
}

/// `(1 - s) S + s mu I`, where `mu` is the mean variance.
fn shrink(mut values: Vec<f64>, n: usize, shrinkage: f64) -> Vec<f64> {
    let mu = trace(&values, n) / n as f64;
    for v in &mut values {
        *v *= 1.0 - shrinkage;
    }
    for i in 0..n {
        values[i * n + i] += shrinkage * mu;
    }
    values
}

/// The Ledoit–Wolf shrinkage intensity, as computed by scikit-learn.
fn ledoit_wolf(centred: &[Vec<f64>], empirical: &[f64], n: usize, t: usize) -> f64 {
    if n == 1 {
        return 0.0;
    }
    let (nf, tf) = (n as f64, t as f64);
    let mu = trace(empirical, n) / nf;
    // delta_ = sum of squared entries of X'X / t^2; beta_ = sum of the entries of (X^2)'(X^2).
    let delta_: f64 = empirical.iter().map(|v| v * v).sum();
    let squares: Vec<Vec<f64>> = centred
        .iter()
        .map(|c| c.iter().map(|x| x * x).collect())
        .collect();
    let mut beta_ = 0.0;
    for i in 0..n {
        for j in 0..n {
            beta_ += squares[i]
                .iter()
                .zip(&squares[j])
                .map(|(a, b)| a * b)
                .sum::<f64>();
        }
    }
    let beta = (beta_ / tf - delta_) / (nf * tf);
    let delta = (delta_ - 2.0 * mu * trace(empirical, n) + nf * mu * mu) / nf;
    let beta = beta.min(delta);
    if beta == 0.0 { 0.0 } else { beta / delta }
}

/// The oracle approximating shrinkage intensity, as computed by scikit-learn.
fn oas(empirical: &[f64], n: usize, t: usize) -> f64 {
    if n == 1 {
        return 0.0;
    }
    let nf = n as f64;
    let alpha = empirical.iter().map(|v| v * v).sum::<f64>() / (nf * nf);
    let mu = trace(empirical, n) / nf;
    let numerator = alpha + mu * mu;
    let denominator = (t as f64 + 1.0) * (alpha - mu * mu / nf);
    if denominator == 0.0 {
        1.0
    } else {
        (numerator / denominator).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> ReturnsTable {
        ReturnsTable::new(
            vec!["a".into(), "b".into()],
            vec![vec![0.01, -0.02, 0.03, 0.0], vec![0.02, 0.01, -0.01, 0.0]],
        )
        .unwrap()
    }

    #[test]
    fn sample_covariance() {
        let estimate = covariance(&table(), CovarianceMethod::Sample).unwrap();
        // Means 0.005 and 0.005; deviations a: 0.005 -0.025 0.025 -0.005, b: 0.015 0.005 -0.015 -0.005.
        let m = &estimate.matrix;
        assert!((m.get(0, 0) - 0.0013 / 3.0).abs() < 1e-17);
        assert!((m.get(0, 1) - (-0.0004 / 3.0)).abs() < 1e-17);
        assert_eq!(m.get(0, 1), m.get(1, 0));
        assert_eq!(estimate.shrinkage, None);
    }

    #[test]
    fn ewma_weights_recent_returns_more() {
        let estimate = covariance(&table(), CovarianceMethod::Ewma { decay: 0.5 }).unwrap();
        // Weights 1/15, 2/15, 4/15, 8/15.
        let w = [1.0, 2.0, 4.0, 8.0].map(|x| x / 15.0);
        let a = [0.01, -0.02, 0.03, 0.0];
        let mean: f64 = a.iter().zip(&w).map(|(x, w)| x * w).sum();
        let var: f64 = a.iter().zip(&w).map(|(x, w)| w * (x - mean).powi(2)).sum();
        assert!((estimate.matrix.get(0, 0) - var).abs() < 1e-17);
        assert!(covariance(&table(), CovarianceMethod::Ewma { decay: 1.0 }).is_err());
    }

    #[test]
    fn one_asset_is_not_shrunk() {
        let one = ReturnsTable::new(vec!["a".into()], vec![vec![0.01, -0.02, 0.03]]).unwrap();
        for method in [CovarianceMethod::LedoitWolf, CovarianceMethod::Oas] {
            assert_eq!(covariance(&one, method).unwrap().shrinkage, Some(0.0));
        }
    }
}
