//! Statistics over a moving window.

use finalytics_core::Result;

use crate::input::{self, invalid};
use crate::statistics::{
    RelativeStatistics, Statistics, StatisticsOptions, relative_statistics, statistics,
};
use crate::tail::{TailMethod, TailRisk, tail_risk};

/// Applies `f` to every window of `window` consecutive values. The result has
/// `values.len() - window + 1` entries; entry `k` covers `values[k..k + window]`, so it belongs
/// to the date of `values[k + window - 1]`.
pub fn rolling<T>(
    values: &[f64],
    window: usize,
    f: impl FnMut(&[f64]) -> Result<T>,
) -> Result<Vec<T>> {
    check_window(window, values.len())?;
    values.windows(window).map(f).collect()
}

/// [`statistics`] over every window of `window` returns.
pub fn rolling_statistics(
    returns: &[f64],
    window: usize,
    options: &StatisticsOptions,
) -> Result<Vec<Statistics>> {
    input::returns(returns, "returns", 2)?;
    rolling(returns, window, |w| statistics(w, options))
}

/// [`relative_statistics`] over every window of `window` returns and benchmark returns.
pub fn rolling_relative_statistics(
    returns: &[f64],
    benchmark: &[f64],
    window: usize,
    options: &StatisticsOptions,
) -> Result<Vec<RelativeStatistics>> {
    input::paired_returns(returns, benchmark, 2)?;
    check_window(window, returns.len())?;
    returns
        .windows(window)
        .zip(benchmark.windows(window))
        .map(|(r, b)| relative_statistics(r, b, options))
        .collect()
}

/// [`tail_risk`] over every window of `window` returns.
pub fn rolling_tail_risk(
    returns: &[f64],
    window: usize,
    confidence: f64,
    method: TailMethod,
) -> Result<Vec<TailRisk>> {
    input::returns(returns, "returns", 2)?;
    rolling(returns, window, |w| tail_risk(w, confidence, method))
}

fn check_window(window: usize, len: usize) -> Result<()> {
    if window < 2 || window > len {
        return Err(invalid(
            "window",
            format!("must be between 2 and the number of observations, {len}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_line_up_with_their_last_observation() {
        let r = [0.01, -0.02, 0.03, 0.005, -0.01, 0.02];
        let options = StatisticsOptions::default();
        let rolled = rolling_statistics(&r, 4, &options).unwrap();
        assert_eq!(rolled.len(), 3);
        assert_eq!(rolled[2], statistics(&r[2..], &options).unwrap());
        assert!(rolling_statistics(&r, 7, &options).is_err());
        assert!(rolling_statistics(&r, 1, &options).is_err());

        let b = [0.0, -0.01, 0.02, 0.0, -0.005, 0.01];
        let relative = rolling_relative_statistics(&r, &b, 3, &options).unwrap();
        assert_eq!(relative.len(), 4);
        assert_eq!(
            relative[0],
            relative_statistics(&r[..3], &b[..3], &options).unwrap()
        );

        let tails = rolling_tail_risk(&r, 5, 0.9, TailMethod::Historical).unwrap();
        assert_eq!(tails.len(), 2);
    }
}
