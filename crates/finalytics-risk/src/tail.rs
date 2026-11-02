//! Value at risk and expected shortfall.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::distribution::{normal_cdf, normal_pdf, normal_quantile};
use crate::input::{self, Moments, invalid};

/// How the tail of the return distribution is estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TailMethod {
    /// From the observed returns: the quantile with linear interpolation between order
    /// statistics, and the mean of the worst `floor((n - 1) * alpha) + 1` returns.
    Historical,
    /// From a normal distribution with the sample mean and standard deviation.
    Parametric,
    /// From the normal quantile adjusted for skewness and excess kurtosis by the Cornish–Fisher
    /// expansion. Expected shortfall integrates the Edgeworth expansion of the density to that
    /// quantile (Boudt, Peterson and Croux, 2008), and is never less than the value at risk.
    CornishFisher,
}

/// Value at risk and expected shortfall over one period of the returns.
///
/// Both are losses, reported as positive returns: a value at risk of 0.021 at 95% confidence
/// means that one period in twenty is expected to lose 2.1% or more.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TailRisk {
    /// The confidence level, for example 0.95.
    pub confidence: f64,
    /// The estimation method.
    pub method: TailMethod,
    /// The loss exceeded with probability `1 - confidence`.
    pub value_at_risk: f64,
    /// The expected loss when the value at risk is exceeded (also called conditional value at
    /// risk).
    pub expected_shortfall: f64,
}

/// Estimates value at risk and expected shortfall from at least two returns.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod risk { pub use finalytics_risk::*; } }
/// use finalytics::risk::{TailMethod, tail_risk};
///
/// let returns = [0.004, -0.011, 0.007, -0.002, 0.013, -0.019, 0.001, 0.006];
/// let tail = tail_risk(&returns, 0.95, TailMethod::Parametric)?;
/// assert!(tail.expected_shortfall > tail.value_at_risk);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn tail_risk(returns: &[f64], confidence: f64, method: TailMethod) -> Result<TailRisk> {
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(invalid(
            "confidence",
            "must be between 0 and 1, for example 0.95",
        ));
    }
    input::returns(returns, "returns", 2)?;
    let alpha = 1.0 - confidence;
    let (value_at_risk, expected_shortfall) = match method {
        TailMethod::Historical => historical(returns, alpha),
        TailMethod::Parametric => {
            let m = Moments::of(returns);
            let sd = m.sample_sd();
            let z = normal_quantile(alpha);
            (-(m.mean + sd * z), -(m.mean - sd * normal_pdf(z) / alpha))
        }
        TailMethod::CornishFisher => {
            let m = Moments::of(returns);
            let sd = m.sample_sd();
            let s = m.skewness().unwrap_or(0.0);
            let k = m.excess_kurtosis().unwrap_or(0.0);
            let h = cornish_fisher_quantile(normal_quantile(alpha), s, k);
            let tail_mean = edgeworth_tail_mean(h, s, k, alpha).min(h);
            (-(m.mean + sd * h), -(m.mean + sd * tail_mean))
        }
    };
    Ok(TailRisk {
        confidence,
        method,
        value_at_risk,
        expected_shortfall,
    })
}

fn historical(returns: &[f64], alpha: f64) -> (f64, f64) {
    let mut sorted = returns.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = (sorted.len() - 1) as f64 * alpha;
    let below = position.floor();
    // `below` is at most n - 1, so the conversion is exact.
    let k = below as usize;
    let quantile = match sorted.get(k + 1) {
        Some(next) => sorted[k] + (position - below) * (next - sorted[k]),
        None => sorted[k],
    };
    let tail = &sorted[..=k];
    let tail_mean = tail.iter().sum::<f64>() / tail.len() as f64;
    (-quantile, -tail_mean)
}

/// The Cornish–Fisher expansion of the standard normal quantile `z` for skewness `s` and excess
/// kurtosis `k`.
fn cornish_fisher_quantile(z: f64, s: f64, k: f64) -> f64 {
    let z2 = z * z;
    let z3 = z2 * z;
    z + (z2 - 1.0) * s / 6.0 + (z3 - 3.0 * z) * k / 24.0 - (2.0 * z3 - 5.0 * z) * s * s / 36.0
}

/// The mean of a standardised return below `h` under the Edgeworth expansion of the density,
/// `(1 / alpha) * integral of x e(x) from -infinity to h`, where
/// `e(x) = phi(x) (1 + s/6 He3(x) + k/24 He4(x) + s^2/72 He6(x))`.
fn edgeworth_tail_mean(h: f64, s: f64, k: f64, alpha: f64) -> f64 {
    // i[n] = integral of x^n phi(x) from -infinity to h, by integration by parts:
    // i[n] = -h^(n-1) phi(h) + (n - 1) i[n - 2].
    let density = normal_pdf(h);
    let mut i = [0.0; 8];
    i[0] = normal_cdf(h);
    i[1] = -density;
    for n in 2..8 {
        i[n] = -h.powi(n as i32 - 1) * density + (n - 1) as f64 * i[n - 2];
    }
    let integral = i[1]
        + s / 6.0 * (i[4] - 3.0 * i[2])
        + k / 24.0 * (i[5] - 6.0 * i[3] + 3.0 * i[1])
        + s * s / 72.0 * (i[7] - 15.0 * i[5] + 45.0 * i[3] - 15.0 * i[1]);
    integral / alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_quantile_interpolates() {
        // Sorted: -0.05, -0.03, -0.01, 0.00, 0.02; at alpha 0.25 the position is 1.0.
        let r = [0.02, -0.01, -0.05, 0.0, -0.03];
        let tail = tail_risk(&r, 0.75, TailMethod::Historical).unwrap();
        assert!((tail.value_at_risk - 0.03).abs() < 1e-15);
        assert!((tail.expected_shortfall - 0.04).abs() < 1e-15);
        // At alpha 0.1 the position is 0.4: -0.05 + 0.4 * 0.02.
        let tail = tail_risk(&r, 0.9, TailMethod::Historical).unwrap();
        assert!((tail.value_at_risk - 0.042).abs() < 1e-15);
        assert!((tail.expected_shortfall - 0.05).abs() < 1e-15);
    }

    #[test]
    fn cornish_fisher_without_skew_or_kurtosis_is_normal() {
        let z = normal_quantile(0.01);
        assert_eq!(cornish_fisher_quantile(z, 0.0, 0.0), z);
        let normal_tail_mean = -normal_pdf(z) / 0.01;
        assert!((edgeworth_tail_mean(z, 0.0, 0.0, 0.01) - normal_tail_mean).abs() < 1e-12);
    }

    #[test]
    fn confidence_must_be_a_probability() {
        let r = [0.01, -0.02, 0.005];
        for c in [0.0, 1.0, -0.5, f64::NAN] {
            assert!(tail_risk(&r, c, TailMethod::Parametric).is_err());
        }
    }
}
