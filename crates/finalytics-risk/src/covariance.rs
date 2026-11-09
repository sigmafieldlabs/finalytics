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
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
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
    /// The empirical covariance (divisor n) shrunk towards the matrix with the same variances
    /// and every correlation the average sample correlation, with the intensity of Ledoit and
    /// Wolf (2004, "Honey, I shrunk the sample covariance matrix").
    ConstantCorrelation,
    /// A statistical factor model of the first `factors` principal components of the sample
    /// covariance, with each asset's variance kept: the covariance the model implies.
    Statistical {
        /// The number of components, between 1 and the smaller of the number of assets and of
        /// returns less one.
        factors: usize,
    },
    /// The sample correlation with the eigenvalues of noise replaced by their average: those
    /// below the Marchenko–Pastur edge `(1 + sqrt(n / t))^2` of a random correlation matrix
    /// of `n` assets and `t` returns. The variances are the sample's.
    Denoised,
    /// The analytical nonlinear shrinkage of Ledoit and Wolf (2020): each eigenvalue of the
    /// sample covariance (divisor t - 1) shrunk by a kernel estimate of the spectral density,
    /// for many assets against few returns.
    NonlinearShrinkage,
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
    ConstantCorrelation,
    Statistical {
        factors: usize,
    },
    Denoised,
    NonlinearShrinkage,
}

impl From<Tagged> for CovarianceMethod {
    fn from(tagged: Tagged) -> Self {
        match tagged {
            Tagged::Sample => CovarianceMethod::Sample,
            Tagged::Ewma { decay } => CovarianceMethod::Ewma { decay },
            Tagged::LedoitWolf => CovarianceMethod::LedoitWolf,
            Tagged::Oas => CovarianceMethod::Oas,
            Tagged::ConstantCorrelation => CovarianceMethod::ConstantCorrelation,
            Tagged::Statistical { factors } => CovarianceMethod::Statistical { factors },
            Tagged::Denoised => CovarianceMethod::Denoised,
            Tagged::NonlinearShrinkage => CovarianceMethod::NonlinearShrinkage,
        }
    }
}

/// Reads a covariance method by name or as a tagged object.
struct MethodVisitor;

impl<'de> serde::de::Visitor<'de> for MethodVisitor {
    type Value = CovarianceMethod;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "a covariance method: sample, ewma, ledoit_wolf, oas, constant_correlation, denoised \
             or nonlinear_shrinkage, or an object with a type",
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
            "constant_correlation" => Ok(CovarianceMethod::ConstantCorrelation),
            "denoised" => Ok(CovarianceMethod::Denoised),
            "nonlinear_shrinkage" => Ok(CovarianceMethod::NonlinearShrinkage),
            "statistical" => Err(E::custom(
                "a statistical covariance needs its number of factors, as \
                 {\"type\": \"statistical\", \"factors\": 3}",
            )),
            _ => Err(E::custom(format!(
                "unknown covariance method {name:?}: use sample, ewma, ledoit_wolf, oas, \
                 constant_correlation, denoised or nonlinear_shrinkage, or \
                 {{\"type\": \"statistical\", \"factors\": 3}}"
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
        CovarianceMethod::ConstantCorrelation => {
            let centred = centred(returns.columns(), None);
            let (values, shrinkage) = constant_correlation(&centred, n, t);
            (values, Some(shrinkage))
        }
        CovarianceMethod::Statistical { factors } => {
            let model = crate::factor::statistical_model(returns, factors)?.model;
            (model.covariance().values().to_vec(), None)
        }
        CovarianceMethod::Denoised => {
            let centred = centred(returns.columns(), None);
            let sample = cross_products(&centred, None, (t - 1) as f64);
            (denoised(&sample, n, t)?, None)
        }
        CovarianceMethod::NonlinearShrinkage => {
            // The kernel's bandwidth, (t - 1)^(-1/3), must be below 1 / sqrt(5).
            if t < 13 {
                return Err(invalid(
                    "returns",
                    "nonlinear shrinkage needs at least 13 returns per asset",
                ));
            }
            let centred = centred(returns.columns(), None);
            let sample = cross_products(&centred, None, (t - 1) as f64);
            (nonlinear_shrinkage(&sample, n, t - 1)?, None)
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

/// The Ledoit–Wolf (2004) shrinkage towards constant correlation of the empirical covariance of
/// the `centred` returns (`n` assets, `t` returns), with its intensity: a translation of the
/// authors' `covCor`.
fn constant_correlation(centred: &[Vec<f64>], n: usize, t: usize) -> (Vec<f64>, f64) {
    let tf = t as f64;
    let sample = cross_products(centred, None, tf);
    let var: Vec<f64> = (0..n).map(|i| sample[i * n + i]).collect();
    let sd: Vec<f64> = var.iter().map(|v| v.max(0.0).sqrt()).collect();
    if n < 2 || sd.contains(&0.0) {
        return (sample, 0.0);
    }
    let mut correlations = 0.0;
    for i in 0..n {
        for j in 0..n {
            if i != j {
                correlations += sample[i * n + j] / (sd[i] * sd[j]);
            }
        }
    }
    let r_bar = correlations / (n * (n - 1)) as f64;
    let mut prior = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            prior[i * n + j] = if i == j {
                var[i]
            } else {
                r_bar * sd[i] * sd[j]
            };
        }
    }
    // pi: the sum of the asymptotic variances of the sample covariances.
    let mut phi = 0.0;
    let mut phi_diagonal = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mean_square: f64 = centred[i]
                .iter()
                .zip(&centred[j])
                .map(|(a, b)| (a * a) * (b * b))
                .sum::<f64>()
                / tf;
            let entry = mean_square - sample[i * n + j].powi(2);
            phi += entry;
            if i == j {
                phi_diagonal += entry;
            }
        }
    }
    // rho: the asymptotic covariances of the target's entries with the sample's.
    let mut theta_sum = 0.0;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let term1: f64 = centred[i]
                .iter()
                .zip(&centred[j])
                .map(|(a, b)| a.powi(3) * b)
                .sum::<f64>()
                / tf;
            let theta = term1 - var[i] * sample[i * n + j];
            theta_sum += sd[j] / sd[i] * theta;
        }
    }
    let rho = phi_diagonal + r_bar * theta_sum;
    let gamma: f64 = sample
        .iter()
        .zip(&prior)
        .map(|(s, f)| (s - f).powi(2))
        .sum();
    let shrinkage = if gamma > 0.0 {
        ((phi - rho) / gamma / tf).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let values = sample
        .iter()
        .zip(&prior)
        .map(|(s, f)| shrinkage * f + (1.0 - shrinkage) * s)
        .collect();
    (values, shrinkage)
}

/// `sample` (`n` by `n`, from `t` returns) with its correlation's eigenvalues below the
/// Marchenko–Pastur edge replaced by their average, which keeps the trace.
fn denoised(sample: &[f64], n: usize, t: usize) -> Result<Vec<f64>> {
    let sd: Vec<f64> = (0..n).map(|i| sample[i * n + i].max(0.0).sqrt()).collect();
    if sd.contains(&0.0) {
        return Err(invalid(
            "returns",
            "an asset's returns do not vary, so they have no correlation to denoise",
        ));
    }
    let mut correlation = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            correlation[i * n + j] = sample[i * n + j] / (sd[i] * sd[j]);
        }
    }
    let (mut values, vectors) = crate::linalg::symmetric_eigen(&correlation, n)
        .ok_or_else(|| invalid("returns", "the eigendecomposition did not converge"))?;
    let edge = (1.0 + (n as f64 / t as f64).sqrt()).powi(2);
    let noise: Vec<usize> = (0..n).filter(|k| values[*k] < edge).collect();
    if !noise.is_empty() {
        let average = noise.iter().map(|k| values[*k]).sum::<f64>() / noise.len() as f64;
        for k in noise {
            values[k] = average;
        }
    }
    let mut rebuilt = vec![0.0; n * n];
    for (l, v) in values.iter().zip(&vectors) {
        for i in 0..n {
            for j in 0..n {
                rebuilt[i * n + j] += l * v[i] * v[j];
            }
        }
    }
    // Back to a unit diagonal, then to the sample's variances.
    let scale: Vec<f64> = (0..n).map(|i| rebuilt[i * n + i].sqrt()).collect();
    let mut out = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            out[i * n + j] = rebuilt[i * n + j] / (scale[i] * scale[j]) * sd[i] * sd[j];
        }
    }
    Ok(out)
}

/// The analytical nonlinear shrinkage of Ledoit and Wolf (2020) of `sample` (`p` by `p`, with
/// divisor `n`, the returns less one): a translation of the authors' `analytical_shrinkage`.
fn nonlinear_shrinkage(sample: &[f64], p: usize, n: usize) -> Result<Vec<f64>> {
    use std::f64::consts::PI;
    let (mut lambda, mut vectors) = crate::linalg::symmetric_eigen(sample, p)
        .ok_or_else(|| invalid("returns", "the eigendecomposition did not converge"))?;
    // Ascending, as in the authors' code.
    lambda.reverse();
    vectors.reverse();
    let (pf, nf) = (p as f64, n as f64);
    let first = p.saturating_sub(n);
    let kept: Vec<f64> = lambda[first..].to_vec();
    if kept.iter().any(|l| *l <= 0.0) {
        return Err(invalid(
            "returns",
            "the sample covariance has a zero eigenvalue among those kept, as when an asset is \
             a combination of others or does not vary",
        ));
    }
    let m = kept.len();
    let h = nf.powf(-1.0 / 3.0);
    let root5 = 5f64.sqrt();
    let mut ftilde = vec![0.0; m];
    let mut hilbert = vec![0.0; m];
    for i in 0..m {
        for j in 0..m {
            let bandwidth = h * kept[j];
            let x = (kept[i] - kept[j]) / bandwidth;
            ftilde[i] += (3.0 / 4.0 / root5) * (1.0 - x * x / 5.0).max(0.0) / bandwidth;
            let hf = if (x.abs() - root5).abs() < 1e-12 {
                (-3.0 / 10.0 / PI) * x
            } else {
                (-3.0 / 10.0 / PI) * x
                    + (3.0 / 4.0 / root5 / PI)
                        * (1.0 - x * x / 5.0)
                        * ((root5 - x) / (root5 + x)).abs().ln()
            };
            hilbert[i] += hf / bandwidth;
        }
        ftilde[i] /= m as f64;
        hilbert[i] /= m as f64;
    }
    let shrunk: Vec<f64> = if p <= n {
        let c = pf / nf;
        (0..m)
            .map(|i| {
                let l = kept[i];
                l / ((PI * c * l * ftilde[i]).powi(2) + (1.0 - c - PI * c * l * hilbert[i]).powi(2))
            })
            .collect()
    } else {
        let mean_inverse = kept.iter().map(|l| 1.0 / l).sum::<f64>() / m as f64;
        let hilbert0 = (1.0 / PI)
            * (3.0 / 10.0 / h / h
                + 3.0 / 4.0 / root5 / h
                    * (1.0 - 1.0 / 5.0 / h / h)
                    * ((1.0 + root5 * h) / (1.0 - root5 * h)).ln())
            * mean_inverse;
        let zero = 1.0 / (PI * (pf - nf) / nf * hilbert0);
        std::iter::repeat_n(zero, first)
            .chain((0..m).map(|i| {
                let l = kept[i];
                l / (PI * PI * l * l * (ftilde[i].powi(2) + hilbert[i].powi(2)))
            }))
            .collect()
    };
    let mut out = vec![0.0; p * p];
    for (d, v) in shrunk.iter().zip(&vectors) {
        for i in 0..p {
            for j in 0..p {
                out[i * p + j] += d * v[i] * v[j];
            }
        }
    }
    Ok(out)
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
            unknown.contains("use sample, ewma, ledoit_wolf, oas, constant_correlation"),
            "{unknown}"
        );
        assert_eq!(
            read(r#"{"type": "statistical", "factors": 2}"#).unwrap(),
            CovarianceMethod::Statistical { factors: 2 }
        );
        let bare = read(r#""statistical""#).unwrap_err().to_string();
        assert!(bare.contains("needs its number of factors"), "{bare}");
        assert!(read(r#"{"type": "statistical"}"#).is_err());
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
