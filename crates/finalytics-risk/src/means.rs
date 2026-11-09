//! Estimators of the assets' mean returns.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::covariance::{CovarianceMethod, covariance};
use crate::input::invalid;
use crate::matrix::ReturnsTable;

/// How the assets' mean returns are estimated from their returns.
///
/// In JSON it is an object with a `type`, such as `{"type": "ewma", "decay": 0.97}`, or the
/// type's name alone, such as `"james_stein"`; an exponentially weighted mean without a decay
/// has 0.94.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum MeanMethod {
    /// Each asset's average return.
    #[default]
    Historical,
    /// The averages shrunk towards their grand mean, `(1 - c) m + c g`, with the intensity of
    /// Efron and Morris (1973), `c = (n - 3) / (t (m - g)' S^-1 (m - g))` between 0 and 1, for
    /// `n` assets, `t` returns and the sample covariance `S`. It needs more returns than assets,
    /// and at least four assets to shrink.
    JamesStein,
    /// An exponentially weighted average: the return `k` periods before the latest has weight
    /// proportional to `decay^k`.
    Ewma {
        /// The decay factor, between 0 and 1: 0.94 by default.
        #[serde(default = "default_decay")]
        decay: f64,
    },
}

fn default_decay() -> f64 {
    0.94
}

/// Mean returns, per period of the returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MeanEstimate {
    /// The estimator used.
    pub method: MeanMethod,
    /// Each asset's mean return per period, in the order of the returns' columns.
    pub means: Vec<f64>,
    /// For James–Stein, the intensity of the shrinkage towards the grand mean, from 0 to 1.
    pub shrinkage: Option<f64>,
}

/// Estimates the mean return of each asset in `returns`.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod risk { pub use finalytics_risk::*; } }
/// use finalytics::risk::{MeanMethod, ReturnsTable, mean_returns};
///
/// let returns = ReturnsTable::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![vec![0.012, -0.008, 0.004, 0.010], vec![-0.002, 0.003, 0.001, -0.001]],
/// )?;
/// let means = mean_returns(&returns, MeanMethod::Historical)?;
/// assert!((means.means[0] - 0.0045).abs() < 1e-15);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn mean_returns(returns: &ReturnsTable, method: MeanMethod) -> Result<MeanEstimate> {
    let columns = returns.columns();
    let (n, t) = (returns.assets(), returns.observations());
    if t == 0 {
        return Err(invalid("returns", "there are none"));
    }
    let historical: Vec<f64> = columns
        .iter()
        .map(|c| finalytics_core::numeric::compensated_sum(c.iter().copied()) / t as f64)
        .collect();
    let (means, shrinkage) = match method {
        MeanMethod::Historical => (historical, None),
        MeanMethod::Ewma { decay } => {
            if !(decay > 0.0 && decay < 1.0) {
                return Err(invalid(
                    "decay",
                    "must be between 0 and 1, for example 0.94",
                ));
            }
            let weights: Vec<f64> = (0..t).map(|i| decay.powi((t - 1 - i) as i32)).collect();
            let total: f64 = weights.iter().sum();
            let means = columns
                .iter()
                .map(|c| c.iter().zip(&weights).map(|(x, w)| x * w).sum::<f64>() / total)
                .collect();
            (means, None)
        }
        MeanMethod::JamesStein => {
            if t <= n {
                return Err(invalid(
                    "returns",
                    format!(
                        "the James–Stein mean needs more returns than assets: {t} returns for {n} \
                         assets"
                    ),
                ));
            }
            let grand = historical.iter().sum::<f64>() / n as f64;
            let gap: Vec<f64> = historical.iter().map(|m| m - grand).collect();
            let sample = covariance(returns, CovarianceMethod::Sample)?.matrix;
            let distance = match crate::linalg::solve_symmetric(sample.values(), n, &gap) {
                Some(x) => x.iter().zip(&gap).map(|(a, b)| a * b).sum::<f64>(),
                None => {
                    return Err(invalid(
                        "returns",
                        "the sample covariance is singular, as when an asset is a combination \
                         of others",
                    ));
                }
            };
            let intensity = if distance > 0.0 {
                ((n as f64 - 3.0) / (t as f64 * distance)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let means = historical
                .iter()
                .map(|m| (1.0 - intensity) * m + intensity * grand)
                .collect();
            (means, Some(intensity))
        }
    };
    Ok(MeanEstimate {
        method,
        means,
        shrinkage,
    })
}

/// The tagged form of [`MeanMethod`], as JSON gives it.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Tagged {
    Historical,
    JamesStein,
    Ewma {
        #[serde(default = "default_decay")]
        decay: f64,
    },
}

struct MethodVisitor;

impl<'de> serde::de::Visitor<'de> for MethodVisitor {
    type Value = MeanMethod;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a mean method: historical, james_stein or ewma, or an object with a type")
    }

    fn visit_str<E: serde::de::Error>(self, name: &str) -> std::result::Result<Self::Value, E> {
        match name {
            "historical" => Ok(MeanMethod::Historical),
            "james_stein" => Ok(MeanMethod::JamesStein),
            "ewma" => Ok(MeanMethod::Ewma {
                decay: default_decay(),
            }),
            _ => Err(E::custom(format!(
                "unknown mean method {name:?}: use historical, james_stein or ewma"
            ))),
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        Ok(
            match Tagged::deserialize(serde::de::value::MapAccessDeserializer::new(map))? {
                Tagged::Historical => MeanMethod::Historical,
                Tagged::JamesStein => MeanMethod::JamesStein,
                Tagged::Ewma { decay } => MeanMethod::Ewma { decay },
            },
        )
    }
}

impl<'de> Deserialize<'de> for MeanMethod {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(MethodVisitor)
    }
}
