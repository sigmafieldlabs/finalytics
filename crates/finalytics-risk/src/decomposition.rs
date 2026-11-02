//! How much each holding adds to a portfolio's risk.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::matrix::Matrix;

/// One asset's share of a portfolio's volatility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RiskContribution {
    /// The asset, as labelled in the covariance matrix.
    pub label: String,
    /// Its weight.
    pub weight: f64,
    /// The change in the portfolio's volatility per unit of weight: `(C w)_i / volatility`.
    /// Absent when the portfolio has no volatility.
    pub marginal: Option<f64>,
    /// `weight * marginal`. The components add up to the portfolio's volatility.
    pub component: f64,
    /// `component / volatility`. The shares add up to one. Absent when the portfolio has no
    /// volatility.
    pub share: Option<f64>,
}

/// A portfolio's volatility split into each asset's contribution (Euler allocation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RiskDecomposition {
    /// The portfolio's volatility, `sqrt(w' C w)`, in the units of the covariance matrix (scale
    /// the matrix to annualise).
    pub volatility: f64,
    /// Each asset's contribution, in the order of the covariance matrix.
    pub contributions: Vec<RiskContribution>,
}

/// A group's share of a portfolio's volatility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct GroupRisk {
    /// The group.
    pub group: String,
    /// The sum of its assets' weights.
    pub weight: f64,
    /// The sum of its assets' components.
    pub component: f64,
    /// The sum of its assets' shares, absent when the portfolio has no volatility.
    pub share: Option<f64>,
}

/// Splits the volatility of a portfolio with `weights` into each asset's contribution.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod risk { pub use finalytics_risk::*; } }
/// use finalytics::risk::{Matrix, risk_decomposition};
///
/// let covariance = Matrix::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![0.0400, 0.0020, 0.0020, 0.0049],
/// )?;
/// let risk = risk_decomposition(&[0.6, 0.4], &covariance)?;
/// let total: f64 = risk.contributions.iter().map(|c| c.component).sum();
/// assert!((total - risk.volatility).abs() < 1e-15);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn risk_decomposition(weights: &[f64], covariance: &Matrix) -> Result<RiskDecomposition> {
    let product = covariance.times(weights)?;
    let variance: f64 = product.iter().zip(weights).map(|(a, b)| a * b).sum();
    let volatility = variance.max(0.0).sqrt();
    let contributions = covariance
        .labels()
        .iter()
        .zip(weights)
        .zip(&product)
        .map(|((label, &weight), &cw)| {
            let marginal = (volatility > 0.0).then(|| cw / volatility);
            RiskContribution {
                label: label.clone(),
                weight,
                marginal,
                component: marginal.map_or(0.0, |m| weight * m),
                share: marginal.map(|m| weight * m / volatility),
            }
        })
        .collect();
    Ok(RiskDecomposition {
        volatility,
        contributions,
    })
}

/// The ex-ante tracking error of a portfolio against a benchmark, split into each asset's
/// contribution: the decomposition of the active weights `weights - benchmark`.
pub fn tracking_error_decomposition(
    weights: &[f64],
    benchmark: &[f64],
    covariance: &Matrix,
) -> Result<RiskDecomposition> {
    covariance.check_vector(weights, "weights")?;
    covariance.check_vector(benchmark, "benchmark")?;
    let active: Vec<f64> = weights.iter().zip(benchmark).map(|(w, b)| w - b).collect();
    risk_decomposition(&active, covariance)
}

/// Adds up a decomposition by group, using `group_of` to name the group of each asset from its
/// position and label. Groups are returned in name order.
pub fn group_risk(
    decomposition: &RiskDecomposition,
    mut group_of: impl FnMut(usize, &str) -> String,
) -> Vec<GroupRisk> {
    let mut groups: BTreeMap<String, GroupRisk> = BTreeMap::new();
    for (k, c) in decomposition.contributions.iter().enumerate() {
        let group = group_of(k, &c.label);
        let entry = groups.entry(group.clone()).or_insert(GroupRisk {
            group,
            weight: 0.0,
            component: 0.0,
            share: c.share.map(|_| 0.0),
        });
        entry.weight += c.weight;
        entry.component += c.component;
        entry.share = entry.share.zip(c.share).map(|(a, b)| a + b);
    }
    groups.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covariance() -> Matrix {
        Matrix::new(
            vec!["a".into(), "b".into(), "cash".into()],
            vec![0.04, 0.01, 0.0, 0.01, 0.09, 0.0, 0.0, 0.0, 0.0],
        )
        .unwrap()
    }

    #[test]
    fn components_add_up_to_the_volatility() {
        let w = [0.5, 0.3, 0.2];
        let risk = risk_decomposition(&w, &covariance()).unwrap();
        // w'Cw = 0.25 * 0.04 + 0.09 * 0.09 + 2 * 0.15 * 0.01 = 0.0211.
        assert!((risk.volatility - 0.0211f64.sqrt()).abs() < 1e-15);
        let total: f64 = risk.contributions.iter().map(|c| c.component).sum();
        assert!((total - risk.volatility).abs() < 1e-15);
        let shares: f64 = risk.contributions.iter().filter_map(|c| c.share).sum();
        assert!((shares - 1.0).abs() < 1e-15);
        assert_eq!(risk.contributions[2].component, 0.0);

        let groups = group_risk(&risk, |_, label| {
            if label == "cash" { "Cash" } else { "Risky" }.to_owned()
        });
        assert_eq!(groups.len(), 2);
        assert!((groups[1].share.unwrap() - 1.0).abs() < 1e-15);
        assert!((groups[1].weight - 0.8).abs() < 1e-15);
    }

    #[test]
    fn all_cash_has_no_risk_to_split() {
        let risk = risk_decomposition(&[0.0, 0.0, 1.0], &covariance()).unwrap();
        assert_eq!(risk.volatility, 0.0);
        assert!(risk.contributions.iter().all(|c| c.share.is_none()));
        assert!(risk_decomposition(&[1.0], &covariance()).is_err());
    }

    #[test]
    fn tracking_error_of_active_weights() {
        let te = tracking_error_decomposition(&[0.6, 0.4, 0.0], &[0.5, 0.5, 0.0], &covariance())
            .unwrap();
        // Active weights 0.1 and -0.1: 0.01 * 0.04 + 0.01 * 0.09 - 2 * 0.01 * 0.01 = 0.0011.
        assert!((te.volatility - 0.0011f64.sqrt()).abs() < 1e-15);
    }
}
