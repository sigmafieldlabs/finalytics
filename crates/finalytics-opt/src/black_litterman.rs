//! The Black–Litterman model: expected returns that blend an equilibrium prior with views.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};
use finalytics_risk::Matrix;

use crate::invalid;
use crate::linalg::{Factor, cholesky_solve};
use crate::problem::{Problem, check_symmetric, check_vector};

/// The prior expected returns of the Black–Litterman model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "prior", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Prior {
    /// The equilibrium returns implied by market weights, `pi = risk_aversion * C w + rate`: the
    /// returns at which the market portfolio is optimal for an investor with that risk aversion.
    Market {
        /// Each asset's market capitalisation or weight, one per asset; they are scaled to sum to
        /// one.
        weights: Vec<f64>,
        /// The market's risk aversion, greater than zero; 2.5 is a common choice, and
        /// `implied_risk_aversion` estimates it from the market's returns.
        risk_aversion: f64,
        /// The risk-free rate, added to every implied return.
        #[serde(default)]
        risk_free_rate: f64,
    },
    /// Given prior returns, one per asset.
    Returns {
        /// The prior expected returns.
        returns: Vec<f64>,
    },
}

/// How uncertain a view is: its variance in the model.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "method", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ViewUncertainty {
    /// Proportional to the variance of the view portfolio, `tau * p' C p` (He and Litterman,
    /// 1999).
    #[default]
    Proportional,
    /// From a confidence in the view (Idzorek, 2005), as `(1 - c) / c * tau * p' C p`: 1 holds the
    /// view with certainty, 0.5 is the proportional uncertainty, and lower confidences move the
    /// posterior less.
    Confidence {
        /// The confidence, greater than 0 and at most 1.
        confidence: f64,
    },
    /// The variance of the view, given directly.
    Variance {
        /// The variance, not negative.
        variance: f64,
    },
}

/// A view: the expected return of a portfolio of assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct View {
    /// The view portfolio: each asset's weight in it, such as `{"A": 1}` for an absolute view on
    /// asset A, or `{"A": 1, "B": -1}` for a view that A outperforms B.
    pub portfolio: BTreeMap<String, f64>,
    /// The expected return of the view portfolio, in the units of the covariance matrix.
    pub expected_return: f64,
    /// How uncertain the view is.
    #[serde(default)]
    pub uncertainty: ViewUncertainty,
}

/// The inputs to the Black–Litterman model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BlackLitterman {
    /// The covariance matrix of the assets' returns, in the units of the returns wanted, for
    /// example annualised.
    pub covariance: Matrix,
    /// The prior expected returns.
    pub prior: Prior,
    /// The views; without any, the posterior returns are the prior.
    #[serde(default)]
    pub views: Vec<View>,
    /// The uncertainty of the prior relative to the covariance, greater than zero.
    #[serde(default = "default_tau")]
    pub tau: f64,
}

fn default_tau() -> f64 {
    0.05
}

/// The posterior of the Black–Litterman model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct BlackLittermanEstimate {
    /// The asset labels, in the order of the covariance matrix.
    pub labels: Vec<String>,
    /// The prior expected returns.
    pub prior_returns: Vec<f64>,
    /// The posterior expected returns, which blend the prior with the views.
    pub posterior_returns: Vec<f64>,
    /// The covariance of returns given the views, `C + M`, where `M` is the posterior uncertainty
    /// of the expected returns (He and Litterman, 2002).
    pub posterior_covariance: Matrix,
    /// The variance of each view, in the order of the views.
    pub view_variances: Vec<f64>,
}

impl BlackLittermanEstimate {
    /// An optimisation problem with the posterior covariance and expected returns, and the
    /// default constraints.
    pub fn problem(&self) -> Problem {
        Problem::new(self.posterior_covariance.clone())
            .with_expected_returns(self.posterior_returns.clone())
    }
}

impl BlackLitterman {
    /// The posterior expected returns and covariance.
    ///
    /// With `P` the views' portfolios (one row per view), `Q` their expected returns and `Omega`
    /// the diagonal matrix of their variances:
    ///
    /// - posterior returns `pi + tau C P' (P tau C P' + Omega)^-1 (Q - P pi)`;
    /// - posterior covariance `C + tau C - tau C P' (P tau C P' + Omega)^-1 P tau C`.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use finalytics_opt::{BlackLitterman, Prior, View};
    /// use finalytics_risk::Matrix;
    ///
    /// let covariance = Matrix::new(
    ///     vec!["equity".into(), "bonds".into()],
    ///     vec![0.0400, 0.0020, 0.0020, 0.0049],
    /// )?;
    /// let model = BlackLitterman {
    ///     covariance,
    ///     prior: Prior::Market { weights: vec![0.6, 0.4], risk_aversion: 2.5, risk_free_rate: 0.0 },
    ///     views: vec![View {
    ///         portfolio: BTreeMap::from([("bonds".to_owned(), 1.0)]),
    ///         expected_return: 0.05,
    ///         uncertainty: Default::default(),
    ///     }],
    ///     tau: 0.05,
    /// };
    /// let estimate = model.estimate()?;
    /// // The view lifts bonds above their equilibrium return.
    /// assert!(estimate.posterior_returns[1] > estimate.prior_returns[1]);
    /// # Ok::<(), finalytics_core::Error>(())
    /// ```
    pub fn estimate(&self) -> Result<BlackLittermanEstimate> {
        let c = &self.covariance;
        let n = c.dimension();
        let labels = c.labels();
        let mut report = ValidationReport::new();
        if n == 0 {
            report.error("covariance", "needs at least one asset");
        }
        check_symmetric(c, &mut report);
        if !(self.tau.is_finite() && self.tau > 0.0) {
            report.error("tau", "must be greater than zero");
        }
        let prior = self.prior_returns(n, &mut report);
        let index: HashMap<&str, usize> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.as_str(), i))
            .collect();
        let mut picks: Vec<Vec<f64>> = Vec::with_capacity(self.views.len());
        for (k, view) in self.views.iter().enumerate() {
            let path = format!("views[{k}]");
            let mut row = vec![0.0; n];
            if view.portfolio.is_empty() {
                report.error(format!("{path}.portfolio"), "needs at least one asset");
            }
            for (asset, weight) in &view.portfolio {
                match index.get(asset.as_str()) {
                    Some(&i) if weight.is_finite() => row[i] = *weight,
                    Some(_) => report.error(
                        format!("{path}.portfolio.{asset}"),
                        "must be a finite number",
                    ),
                    None => report.error(
                        format!("{path}.portfolio"),
                        format!("unknown asset {asset}"),
                    ),
                }
            }
            if !view.expected_return.is_finite() {
                report.error(format!("{path}.expected_return"), "must be a finite number");
            }
            match view.uncertainty {
                ViewUncertainty::Confidence { confidence }
                    if !(confidence > 0.0 && confidence <= 1.0) =>
                {
                    report.error(
                        format!("{path}.uncertainty.confidence"),
                        "must be greater than 0 and at most 1",
                    );
                }
                ViewUncertainty::Variance { variance }
                    if !(variance >= 0.0 && variance.is_finite()) =>
                {
                    report.error(
                        format!("{path}.uncertainty.variance"),
                        "must be a finite number, not negative",
                    );
                }
                _ => {}
            }
            picks.push(row);
        }
        if report.has_errors() {
            return Err(Error::Validation(report));
        }
        // Factorising checks that the covariance matrix is positive semidefinite.
        Factor::of(c)?;

        let tau = self.tau;
        // tau C P', one column per view, stored as rows: `scaled[k][i] = tau (C p_k)_i`.
        let scaled: Vec<Vec<f64>> = picks
            .iter()
            .map(|p| c.times(p).map(|cp| cp.iter().map(|v| tau * v).collect()))
            .collect::<Result<_>>()?;
        let k = picks.len();
        let view_variances: Vec<f64> = self
            .views
            .iter()
            .zip(picks.iter().zip(&scaled))
            .map(|(view, (p, tcp))| {
                let proportional: f64 = p.iter().zip(tcp).map(|(a, b)| a * b).sum();
                match view.uncertainty {
                    ViewUncertainty::Proportional => proportional,
                    ViewUncertainty::Confidence { confidence } => {
                        (1.0 - confidence) / confidence * proportional
                    }
                    ViewUncertainty::Variance { variance } => variance,
                }
            })
            .collect();
        // A = P tau C P' + Omega.
        let mut a = vec![0.0; k * k];
        for r in 0..k {
            for s in 0..k {
                a[r * k + s] = picks[r].iter().zip(&scaled[s]).map(|(x, y)| x * y).sum();
            }
            a[r * k + r] += view_variances[r];
        }
        let singular = || {
            invalid(
                "views",
                "are not independent: a view held with certainty repeats or contradicts others",
            )
        };
        // Posterior returns: pi + tau C P' A^-1 (Q - P pi).
        let mut gap: Vec<f64> = self
            .views
            .iter()
            .zip(&picks)
            .map(|(v, p)| v.expected_return - p.iter().zip(&prior).map(|(x, y)| x * y).sum::<f64>())
            .collect();
        if k > 0 && !cholesky_solve(&a, k, &mut gap) {
            return Err(singular());
        }
        let posterior_returns: Vec<f64> = (0..n)
            .map(|i| prior[i] + (0..k).map(|r| scaled[r][i] * gap[r]).sum::<f64>())
            .collect();
        // Posterior covariance: C + tau C - (tau C P') A^-1 (P tau C).
        let mut values: Vec<f64> = c.values().iter().map(|v| v * (1.0 + tau)).collect();
        for j in 0..n {
            let mut column: Vec<f64> = (0..k).map(|r| scaled[r][j]).collect();
            if k > 0 && !cholesky_solve(&a, k, &mut column) {
                return Err(singular());
            }
            for i in 0..n {
                values[i * n + j] -= (0..k).map(|r| scaled[r][i] * column[r]).sum::<f64>();
            }
        }
        if posterior_returns
            .iter()
            .chain(&values)
            .any(|v| !v.is_finite())
        {
            return Err(invalid(
                "views",
                "are too extreme for the posterior to be computed in floating point, such as a \
                 view with almost no variance on a portfolio with almost no weight",
            ));
        }
        // The two triangles agree up to rounding; make them agree exactly.
        for i in 0..n {
            for j in i + 1..n {
                let mean = 0.5 * (values[i * n + j] + values[j * n + i]);
                values[i * n + j] = mean;
                values[j * n + i] = mean;
            }
        }
        Ok(BlackLittermanEstimate {
            labels: labels.to_vec(),
            prior_returns: prior,
            posterior_returns,
            posterior_covariance: Matrix::new(labels.to_vec(), values)?,
            view_variances,
        })
    }

    fn prior_returns(&self, n: usize, report: &mut ValidationReport) -> Vec<f64> {
        match &self.prior {
            Prior::Market {
                weights,
                risk_aversion,
                risk_free_rate,
            } => {
                check_vector(weights, n, "prior.weights", report);
                let total: f64 = weights.iter().sum();
                if total.is_nan() || total <= 0.0 {
                    report.error("prior.weights", "must have a positive sum");
                }
                if !(risk_aversion.is_finite() && *risk_aversion > 0.0) {
                    report.error("prior.risk_aversion", "must be greater than zero");
                }
                if !risk_free_rate.is_finite() {
                    report.error("prior.risk_free_rate", "must be a finite number");
                }
                if report.has_errors() || weights.len() != n {
                    return vec![0.0; n];
                }
                let w: Vec<f64> = weights.iter().map(|x| x / total).collect();
                self.covariance
                    .times(&w)
                    .map(|cw| {
                        cw.iter()
                            .map(|v| risk_aversion * v + risk_free_rate)
                            .collect()
                    })
                    .unwrap_or_else(|_| vec![0.0; n])
            }
            Prior::Returns { returns } => {
                check_vector(returns, n, "prior.returns", report);
                returns.clone()
            }
        }
    }
}

/// The market's risk aversion implied by its returns, `(mean - rate) / variance` with both
/// annualised by `periods_per_year`: the risk aversion at which the market portfolio is optimal.
/// The variance is the sample variance (divisor `n - 1`).
pub fn implied_risk_aversion(
    returns: &[f64],
    periods_per_year: f64,
    risk_free_rate: f64,
) -> Result<f64> {
    let mut report = ValidationReport::new();
    if returns.len() < 2 {
        report.error("returns", "needs at least 2 observations");
    }
    for (k, r) in returns.iter().enumerate() {
        if !r.is_finite() {
            report.error(format!("returns[{k}]"), "must be a finite number");
        }
    }
    if !(periods_per_year.is_finite() && periods_per_year > 0.0) {
        report.error("periods_per_year", "must be greater than zero");
    }
    if !risk_free_rate.is_finite() {
        report.error("risk_free_rate", "must be a finite number");
    }
    if report.has_errors() {
        return Err(Error::Validation(report));
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean) * (r - mean)).sum::<f64>() / (n - 1.0);
    if variance == 0.0 {
        return Err(invalid(
            "returns",
            "do not vary, so they imply no risk aversion",
        ));
    }
    Ok((mean * periods_per_year - risk_free_rate) / (variance * periods_per_year))
}
