//! Covariance estimators.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::input::invalid;
use crate::matrix::{Matrix, ReturnsTable};

/// How a covariance matrix is estimated from returns.
///
/// In JSON it is an object with a `type`, such as `{"type": "ewma", "decay": 0.97}`, or the
/// type's name alone, such as `"ledoit_wolf"`; an exponentially weighted covariance without a
/// decay has 0.94.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CovarianceMethod {
    /// The sample covariance, with divisor n - 1.
    Sample,
    /// An exponentially weighted covariance: the return `k` periods before the latest has
    /// weight proportional to `decay^k`, the weights sum to one, and deviations are taken from
    /// the weighted mean. A decay of 0.94 for daily returns follows RiskMetrics.
    Ewma {
        /// The decay factor, between 0 and 1: 0.94 by default.
        #[serde(default = "default_decay")]
        decay: f64,
    },
    /// The empirical covariance (divisor n) shrunk towards a multiple of the identity with the
    /// intensity of Ledoit and Wolf (2004).
    LedoitWolf,
    /// The empirical covariance (divisor n) shrunk towards a multiple of the identity with the
    /// oracle approximating shrinkage intensity of Chen, Wiesel, Eldar and Hero (2010).
    Oas,
}

/// The RiskMetrics decay.
fn default_decay() -> f64 {
    0.94
}

/// The tagged form of [`CovarianceMethod`], as JSON gives it.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Tagged {
    Sample,
    Ewma {
        #[serde(default = "default_decay")]
        decay: f64,
    },
    LedoitWolf,
    Oas,
}

impl From<Tagged> for CovarianceMethod {
    fn from(tagged: Tagged) -> Self {
        match tagged {
            Tagged::Sample => CovarianceMethod::Sample,
            Tagged::Ewma { decay } => CovarianceMethod::Ewma { decay },
            Tagged::LedoitWolf => CovarianceMethod::LedoitWolf,
            Tagged::Oas => CovarianceMethod::Oas,
        }
    }
}

/// Reads a covariance method by name or as a tagged object.
struct MethodVisitor;

impl<'de> serde::de::Visitor<'de> for MethodVisitor {
    type Value = CovarianceMethod;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "a covariance method: sample, ewma, ledoit_wolf or oas, or an object with a type",
        )
    }

    fn visit_str<E: serde::de::Error>(self, name: &str) -> std::result::Result<Self::Value, E> {
        match name {
            "sample" => Ok(CovarianceMethod::Sample),
            "ewma" => Ok(CovarianceMethod::Ewma {
                decay: default_decay(),
            }),
            "ledoit_wolf" => Ok(CovarianceMethod::LedoitWolf),
            "oas" => Ok(CovarianceMethod::Oas),
            _ => Err(E::custom(format!(
                "unknown covariance method {name:?}: use sample, ewma, ledoit_wolf or oas"
            ))),
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        Tagged::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(Into::into)
    }
}

impl<'de> Deserialize<'de> for CovarianceMethod {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(MethodVisitor)
    }
}

/// An estimated covariance matrix, per period of the returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CovarianceEstimate {
    /// The estimator used.
    pub method: CovarianceMethod,
    /// The covariance matrix, labelled like the returns. Scale it to annualise, for example by
    /// 252 for daily returns.
    pub matrix: Matrix,
    /// The shrinkage intensity, between 0 and 1, for the shrinkage estimators.
    pub shrinkage: Option<f64>,
    /// The number of returns per asset used.
    pub observations: usize,
}

/// Estimates the covariance matrix of the assets in `returns`.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod risk { pub use finalytics_risk::*; } }
/// use finalytics::risk::{CovarianceMethod, ReturnsTable, covariance};
///
/// let returns = ReturnsTable::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![vec![0.012, -0.008, 0.004, 0.010], vec![-0.002, 0.003, 0.001, -0.001]],
/// )?;
/// let estimate = covariance(&returns, CovarianceMethod::LedoitWolf)?;
/// let annual = estimate.matrix.scaled(252.0);
/// assert!(annual.get(0, 0) > annual.get(1, 1));
/// # Ok::<(), finalytics::Error>(())
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
    // delta_ = sum of squared entries of X'X / t^2; beta_ = sum of the entries of (X^2)'(X^2),
    // which is sum_t (sum_i x_ti^2)^2 and so needs one pass over the observations.
    let delta_: f64 = empirical.iter().map(|v| v * v).sum();
    let mut row_squares = vec![0.0; t];
    for column in centred {
        for (sum, x) in row_squares.iter_mut().zip(column) {
            *sum += x * x;
        }
    }
    let beta_: f64 = row_squares.iter().map(|s| s * s).sum();
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
    fn methods_read_by_name_or_as_objects() {
        let read = |text: &str| serde_json::from_str::<CovarianceMethod>(text);
        assert_eq!(
            read(r#""ledoit_wolf""#).unwrap(),
            CovarianceMethod::LedoitWolf
        );
        assert_eq!(
            read(r#""ewma""#).unwrap(),
            CovarianceMethod::Ewma { decay: 0.94 }
        );
        assert_eq!(
            read(r#"{"type": "ewma"}"#).unwrap(),
            CovarianceMethod::Ewma { decay: 0.94 }
        );
        assert_eq!(
            read(r#"{"type": "ewma", "decay": 0.97}"#).unwrap(),
            CovarianceMethod::Ewma { decay: 0.97 }
        );
        assert_eq!(read(r#"{"type": "oas"}"#).unwrap(), CovarianceMethod::Oas);
        let unknown = read(r#""ledoitwolf""#).unwrap_err().to_string();
        assert!(
            unknown.contains("use sample, ewma, ledoit_wolf or oas"),
            "{unknown}"
        );
        assert!(read(r#"{"type": "ewma", "decy": 0.9}"#).is_err());
        assert!(read(r#"{"type": "shrunk"}"#).is_err());
        // Written as objects, as before.
        let ewma = CovarianceMethod::Ewma { decay: 0.97 };
        assert_eq!(
            serde_json::to_string(&ewma).unwrap(),
            r#"{"type":"ewma","decay":0.97}"#
        );
        assert_eq!(read(&serde_json::to_string(&ewma).unwrap()).unwrap(), ewma);
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
