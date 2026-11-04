//! How much each holding adds to a portfolio's risk.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::input::invalid;
use crate::matrix::{Matrix, ReturnsTable};

/// One asset's share of a portfolio's risk: its volatility, or its expected shortfall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RiskContribution {
    /// The asset, as labelled in the covariance matrix.
    pub label: String,
    /// Its weight.
    pub weight: f64,
    /// The change in the portfolio's risk per unit of weight: for volatility,
    /// `(C w)_i / volatility`; for expected shortfall, the asset's mean loss in the tail
    /// scenarios. Absent when the portfolio has no volatility.
    pub marginal: Option<f64>,
    /// `weight * marginal`. The components add up to the portfolio's risk.
    pub component: f64,
    /// `component` over the portfolio's risk. The shares add up to one. Absent when the
    /// portfolio has no risk.
    pub share: Option<f64>,
}

/// A portfolio's expected shortfall split into each asset's contribution (historical Euler
/// allocation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ShortfallDecomposition {
    /// The confidence, for example 0.95.
    pub confidence: f64,
    /// The portfolio's expected shortfall over one period of the scenarios, as a positive loss:
    /// the mean loss in the worst `1 - confidence` of them (the Rockafellar–Uryasev tail mean,
    /// which minimum CVaR minimises).
    pub expected_shortfall: f64,
    /// Each asset's contribution: its weight times its mean loss in those scenarios, in the
    /// order of the scenarios' columns. The components add up to the expected shortfall.
    pub contributions: Vec<RiskContribution>,
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
    /// The sum of its assets' shares, absent when the portfolio has no risk.
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

/// Splits the expected shortfall at `confidence` of a portfolio with `weights` of the assets of
/// `scenarios` (returns, one column per asset) into each asset's contribution: its weight times
/// its mean return, as a loss, in the scenarios of the tail, each weighted as the tail mean
/// weighs it. The contributions add up to the expected shortfall.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod risk { pub use finalytics_risk::*; } }
/// use finalytics::risk::{ReturnsTable, shortfall_decomposition};
///
/// let scenarios = ReturnsTable::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![vec![-0.08, 0.02, 0.03, -0.01], vec![0.01, 0.0, 0.005, -0.004]],
/// )?;
/// let es = shortfall_decomposition(&[0.6, 0.4], &scenarios, 0.75)?;
/// let total: f64 = es.contributions.iter().map(|c| c.component).sum();
/// assert!((total - es.expected_shortfall).abs() < 1e-15);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn shortfall_decomposition(
    weights: &[f64],
    scenarios: &ReturnsTable,
    confidence: f64,
) -> Result<ShortfallDecomposition> {
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(invalid(
            "confidence",
            "must be between 0 and 1, for example 0.95",
        ));
    }
    let columns = scenarios.columns();
    if weights.len() != columns.len() {
        return Err(invalid(
            "weights",
            format!("{} weights for {} assets", weights.len(), columns.len()),
        ));
    }
    if let Some(k) = weights.iter().position(|w| !w.is_finite()) {
        return Err(invalid(&format!("weights[{k}]"), "must be a finite number"));
    }
    let t = scenarios.observations();
    if t == 0 {
        return Err(invalid("scenarios", "there are none"));
    }
    let loss = |s: usize| {
        -columns
            .iter()
            .zip(weights)
            .map(|(c, w)| c[s] * w)
            .sum::<f64>()
    };
    let mut order: Vec<usize> = (0..t).collect();
    order.sort_by(|a, b| loss(*b).total_cmp(&loss(*a)));
    // The tail holds `m = (1 - confidence) t` scenarios: the worst `floor(m)` whole, and a
    // fraction of the next.
    let m = (1.0 - confidence) * t as f64;
    let mut tail = Vec::new();
    let mut left = m;
    for &s in &order {
        if left <= 0.0 {
            break;
        }
        let part = left.min(1.0);
        tail.push((s, part / m));
        left -= part;
    }
    let expected_shortfall: f64 = tail.iter().map(|(s, p)| p * loss(*s)).sum();
    let contributions = scenarios
        .labels()
        .iter()
        .zip(columns)
        .zip(weights)
        .map(|((label, column), w)| {
            let marginal: f64 = tail.iter().map(|(s, p)| -p * column[*s]).sum();
            let component = w * marginal;
            RiskContribution {
                label: label.clone(),
                weight: *w,
                marginal: Some(marginal),
                component,
                share: (expected_shortfall != 0.0).then(|| component / expected_shortfall),
            }
        })
        .collect();
    Ok(ShortfallDecomposition {
        confidence,
        expected_shortfall,
        contributions,
    })
}

/// Adds up a decomposition by group, using `group_of` to name the group of each asset from its
/// position and label. Groups are returned in name order.
pub fn group_risk(
    decomposition: &RiskDecomposition,
    group_of: impl FnMut(usize, &str) -> String,
) -> Vec<GroupRisk> {
    group_contributions(&decomposition.contributions, group_of)
}

/// Adds up contributions to a portfolio's risk, of a [`RiskDecomposition`] or a
/// [`ShortfallDecomposition`], by group, as [`group_risk`] does.
pub fn group_contributions(
    contributions: &[RiskContribution],
    mut group_of: impl FnMut(usize, &str) -> String,
) -> Vec<GroupRisk> {
    let mut groups: BTreeMap<String, GroupRisk> = BTreeMap::new();
    for (k, c) in contributions.iter().enumerate() {
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
