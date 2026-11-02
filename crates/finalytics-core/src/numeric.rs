//! Numerical helpers shared by the analytics crates.

/// A running sum with Neumaier's compensation, which keeps the rounding error of long sums
/// close to that of a single addition.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; }
/// use finalytics::numeric::CompensatedSum;
///
/// let mut sum = CompensatedSum::new();
/// for x in [1e100, 1.0, -1e100] {
///     sum.add(x);
/// }
/// assert_eq!(sum.value(), 1.0); // a plain sum gives 0.0
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompensatedSum {
    sum: f64,
    compensation: f64,
}

impl CompensatedSum {
    /// An empty sum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `x`.
    pub fn add(&mut self, x: f64) {
        let t = self.sum + x;
        if self.sum.abs() >= x.abs() {
            self.compensation += (self.sum - t) + x;
        } else {
            self.compensation += (x - t) + self.sum;
        }
        self.sum = t;
    }

    /// The sum so far.
    pub fn value(&self) -> f64 {
        self.sum + self.compensation
    }
}

/// The compensated sum of `values` (see [`CompensatedSum`]).
pub fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = CompensatedSum::new();
    for x in values {
        sum.add(x);
    }
    sum.value()
}

/// Whether `a` and `b` agree within an absolute tolerance `abs` or a relative tolerance `rel`
/// (relative to the larger magnitude). NaN is never close to anything.
pub fn approx_eq(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    if a == b {
        return true;
    }
    let diff = (a - b).abs();
    diff <= abs || diff <= rel * a.abs().max(b.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compensation_recovers_lost_digits() {
        assert_eq!(compensated_sum([1e100, 1.0, -1e100]), 1.0);
        let tenths = std::iter::repeat_n(0.1, 10_000_000);
        let plain: f64 = std::iter::repeat_n(0.1, 10_000_000).sum();
        let compensated = compensated_sum(tenths);
        assert!((compensated - 1_000_000.0).abs() < (plain - 1_000_000.0).abs());
        assert!((compensated - 1_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn tolerances() {
        assert!(approx_eq(1.0, 1.0 + 1e-12, 1e-9, 0.0));
        assert!(!approx_eq(1.0, 1.001, 1e-9, 0.0));
        assert!(approx_eq(0.0, 1e-13, 0.0, 1e-12));
        assert!(!approx_eq(f64::NAN, f64::NAN, 1.0, 1.0));
        assert!(approx_eq(f64::INFINITY, f64::INFINITY, 0.0, 0.0));
    }
}
