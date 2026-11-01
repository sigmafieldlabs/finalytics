//! Checks on the series the analytics take.

use finalytics_core::{Error, Result, ValidationReport};

/// Checks that `values` is a series of at least `min` returns: finite, and none below -100%.
pub(crate) fn returns(values: &[f64], name: &str, min: usize) -> Result<()> {
    let mut report = ValidationReport::new();
    check_returns(&mut report, values, name, min);
    report.into_result(())
}

pub(crate) fn check_returns(report: &mut ValidationReport, values: &[f64], name: &str, min: usize) {
    if values.len() < min {
        report.error(
            name,
            format!("needs at least {min} observations, found {}", values.len()),
        );
    }
    for (k, value) in values.iter().enumerate() {
        if !value.is_finite() {
            report.error(format!("{name}[{k}]"), "must be a finite number");
        } else if *value < -1.0 {
            report.error(format!("{name}[{k}]"), "is a return below -100%");
        }
    }
}

/// Checks that a returns series and its benchmark are valid and of the same length.
pub(crate) fn paired_returns(returns: &[f64], benchmark: &[f64], min: usize) -> Result<()> {
    let mut report = ValidationReport::new();
    check_returns(&mut report, returns, "returns", min);
    check_returns(&mut report, benchmark, "benchmark", min);
    if returns.len() != benchmark.len() {
        report.error(
            "benchmark",
            format!(
                "has {} observations, but the returns have {}",
                benchmark.len(),
                returns.len()
            ),
        );
    }
    report.into_result(())
}

/// An error for an invalid parameter.
pub(crate) fn invalid(name: &str, message: impl Into<String>) -> Error {
    let mut report = ValidationReport::new();
    report.error(name, message);
    Error::Validation(report)
}

/// The mean and central moments of a sample, as population moments.
pub(crate) struct Moments {
    pub(crate) n: usize,
    pub(crate) mean: f64,
    pub(crate) m2: f64,
    pub(crate) m3: f64,
    pub(crate) m4: f64,
    /// The largest absolute value in the sample, the scale for deciding when a dispersion is zero.
    pub(crate) scale: f64,
}

impl Moments {
    pub(crate) fn of(values: &[f64]) -> Moments {
        let n = values.len();
        let count = n as f64;
        let mean = finalytics_core::numeric::compensated_sum(values.iter().copied()) / count;
        let (mut m2, mut m3, mut m4) = (0.0, 0.0, 0.0);
        let mut scale: f64 = 0.0;
        for &x in values {
            let d = x - mean;
            let d2 = d * d;
            m2 += d2;
            m3 += d2 * d;
            m4 += d2 * d2;
            scale = scale.max(x.abs());
        }
        Moments {
            n,
            mean,
            m2: m2 / count,
            m3: m3 / count,
            m4: m4 / count,
            scale,
        }
    }

    /// The sample standard deviation (divisor n - 1).
    pub(crate) fn sample_sd(&self) -> f64 {
        (self.m2 * self.n as f64 / (self.n as f64 - 1.0)).sqrt()
    }

    /// Whether the sample has no dispersion, allowing for rounding in the mean.
    pub(crate) fn is_constant(&self) -> bool {
        self.m2.sqrt() <= 64.0 * f64::EPSILON * self.scale
    }

    /// Skewness from population moments, or `None` for a constant sample.
    pub(crate) fn skewness(&self) -> Option<f64> {
        (!self.is_constant()).then(|| self.m3 / self.m2.powf(1.5))
    }

    /// Excess kurtosis from population moments, or `None` for a constant sample.
    pub(crate) fn excess_kurtosis(&self) -> Option<f64> {
        (!self.is_constant()).then(|| self.m4 / (self.m2 * self.m2) - 3.0)
    }
}
