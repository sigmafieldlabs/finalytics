//! Resampled optimisation: the average of the optimal portfolios for estimates re-made from
//! simulated histories (Michaud, 1998).

use serde::{Deserialize, Serialize};
use web_time::Instant;

use finalytics_core::{Result, ValidationReport};
use finalytics_risk::Matrix;

use crate::invalid;
use crate::linalg::{Factor, cholesky};
use crate::problem::{Objective, Problem};
use crate::solution::SolveStatus;

/// How to resample a problem's estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Resampling {
    /// The number of simulated histories, at least 2; 100 to 500 is usual.
    pub samples: usize,
    /// The number of periods in each simulated history, at least 2: the length of the history
    /// the estimates came from, so that the simulated estimates are as uncertain as the real
    /// ones.
    pub observations: usize,
    /// The seed of the random numbers. The same seed gives the same result on every platform.
    pub seed: u64,
}

/// The result of resampled optimisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ResampledSolution {
    /// The objective each simulated problem was solved for.
    pub objective: Objective,
    /// The asset labels.
    pub labels: Vec<String>,
    /// The average of the simulated problems' optimal weights, absent if none was solved.
    pub weights: Option<Vec<f64>>,
    /// The standard deviation of each weight across the simulated problems (divisor `m - 1` for
    /// `m` solved problems), absent with fewer than two.
    pub weight_deviations: Option<Vec<f64>>,
    /// The expected return of the average weights under the problem's own estimates.
    pub expected_return: Option<f64>,
    /// The volatility of the average weights under the problem's own estimates.
    pub volatility: Option<f64>,
    /// The largest amount by which the average weights break the problem's constraints.
    pub max_violation: Option<f64>,
    /// The number of simulated problems solved and averaged.
    pub solved: usize,
    /// The number of simulated problems the solver could not solve, left out of the average.
    pub failed: usize,
    /// The time taken, in seconds.
    pub solve_time: f64,
}

impl Problem {
    /// Resampled optimisation (Michaud, 1998): simulates `resampling.samples` histories of
    /// `resampling.observations` periods from the problem's expected returns and covariance,
    /// re-estimates both from each history (sample means and sample covariance), solves each
    /// re-estimated problem for `objective` under the same constraints, and averages the optimal
    /// weights.
    ///
    /// The average portfolio is less sensitive to estimation error than the optimum for the
    /// estimates themselves, and more diversified. Simulated returns are `mu + L z`, with `L` the
    /// Cholesky factor of the covariance and `z` standard normal numbers from a seeded generator
    /// (SplitMix64 and the Box–Muller transform). The average of portfolios that each satisfy a
    /// convex constraint satisfies it too, so the average meets every constraint that does not
    /// depend on the estimates; constraints that do (a tracking-error limit) hold for each
    /// simulated problem's own estimates.
    ///
    /// Objectives that use scenario returns (minimum CVaR) and factor exposure limits are not
    /// supported. With a factor model, the simulated histories come from the covariance it
    /// implies.
    pub fn resampled(
        &self,
        objective: &Objective,
        resampling: &Resampling,
    ) -> Result<ResampledSolution> {
        let started = Instant::now();
        let prepared = self.prepare(objective)?;
        let mut report = ValidationReport::new();
        if matches!(objective, Objective::MinimumCvar { .. }) {
            report.error(
                "objective",
                "resampling needs an objective that uses the covariance matrix",
            );
        }
        if !self.constraints.factor_exposures.is_empty() {
            report.error(
                "constraints.factor_exposures",
                "are not supported with resampling",
            );
        }
        if resampling.samples < 2 {
            report.error("resampling.samples", "must be at least 2");
        }
        if resampling.observations < 2 {
            report.error("resampling.observations", "must be at least 2");
        }
        report.into_result(())?;

        let n = prepared.n;
        let covariance = prepared.risk().dense().into_owned();
        let mu = self.expected_returns.clone();
        let means = mu.clone().unwrap_or_else(|| vec![0.0; n]);
        let root = Root::of(&covariance)?;
        let mut normal = Normal::new(resampling.seed);
        let t = resampling.observations;

        let mut solutions: Vec<Vec<f64>> = Vec::with_capacity(resampling.samples);
        let mut failed = 0;
        let mut history = vec![vec![0.0; t]; n];
        let mut z = vec![0.0; root.width()];
        for _ in 0..resampling.samples {
            for s in 0..t {
                z.iter_mut().for_each(|x| *x = normal.next());
                for (i, column) in history.iter_mut().enumerate() {
                    column[s] = means[i] + root.apply(i, &z);
                }
            }
            let sample_means: Vec<f64> = history
                .iter()
                .map(|c| c.iter().sum::<f64>() / t as f64)
                .collect();
            let mut values = vec![0.0; n * n];
            for i in 0..n {
                for j in i..n {
                    let v = history[i]
                        .iter()
                        .zip(&history[j])
                        .map(|(a, b)| (a - sample_means[i]) * (b - sample_means[j]))
                        .sum::<f64>()
                        / (t as f64 - 1.0);
                    values[i * n + j] = v;
                    values[j * n + i] = v;
                }
            }
            let mut simulated = self.clone();
            simulated.covariance =
                Some(Matrix::new(self.labels.clone(), values).map_err(|_| {
                    invalid(
                        "covariance",
                        "is too large for histories to be simulated in floating point",
                    )
                })?);
            simulated.factor_model = None;
            if mu.is_some() {
                simulated.expected_returns = Some(sample_means);
            }
            match simulated.solve(objective) {
                Ok(solution)
                    if matches!(
                        solution.status,
                        SolveStatus::Optimal | SolveStatus::NearlyOptimal
                    ) && solution.weights.is_some() =>
                {
                    solutions.extend(solution.weights);
                }
                _ => failed += 1,
            }
        }

        let solved = solutions.len();
        let weights = (solved > 0).then(|| {
            (0..n)
                .map(|i| solutions.iter().map(|w| w[i]).sum::<f64>() / solved as f64)
                .collect::<Vec<f64>>()
        });
        let weight_deviations = weights.as_ref().filter(|_| solved > 1).map(|average| {
            (0..n)
                .map(|i| {
                    let squares: f64 = solutions
                        .iter()
                        .map(|w| (w[i] - average[i]) * (w[i] - average[i]))
                        .sum();
                    (squares / (solved as f64 - 1.0)).sqrt()
                })
                .collect()
        });
        let expected_return = weights
            .as_ref()
            .zip(mu.as_ref())
            .map(|(w, m)| w.iter().zip(m).map(|(a, b)| a * b).sum());
        let volatility = weights.as_ref().map(|w| prepared.risk().variance(w).sqrt());
        let max_violation = weights.as_ref().map(|w| prepared.violation(w));
        Ok(ResampledSolution {
            objective: objective.clone(),
            labels: self.labels.clone(),
            weights,
            weight_deviations,
            expected_return,
            volatility,
            max_violation,
            solved,
            failed,
            solve_time: started.elapsed().as_secs_f64(),
        })
    }
}

/// A square root of the covariance matrix for simulation: the lower Cholesky factor, or, for a
/// singular matrix, the transpose of a pivoted factor.
enum Root {
    Cholesky { l: Vec<f64>, n: usize },
    Pivoted(Factor),
}

impl Root {
    fn of(covariance: &Matrix) -> Result<Root> {
        let n = covariance.dimension();
        Ok(match cholesky(covariance.values(), n) {
            Some(l) => Root::Cholesky { l, n },
            None => Root::Pivoted(Factor::of(covariance)?),
        })
    }

    /// The number of standard normal numbers each simulated period needs.
    fn width(&self) -> usize {
        match self {
            Root::Cholesky { n, .. } => *n,
            Root::Pivoted(f) => f.rows.len(),
        }
    }

    /// Entry `i` of `L z` (or `R' z`).
    fn apply(&self, i: usize, z: &[f64]) -> f64 {
        match self {
            Root::Cholesky { l, n } => l[i * n..i * n + i + 1]
                .iter()
                .zip(z)
                .map(|(a, b)| a * b)
                .sum(),
            Root::Pivoted(f) => f.rows.iter().zip(z).map(|(r, b)| r[i] * b).sum(),
        }
    }
}

/// Standard normal numbers from SplitMix64 (Steele, Lea and Flood, 2014) by the Box–Muller
/// transform, with `libm`'s functions so that every platform gives the same numbers.
pub(crate) struct Normal {
    state: u64,
    spare: Option<f64>,
}

impl Normal {
    pub(crate) fn new(seed: u64) -> Normal {
        Normal {
            state: seed,
            spare: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniform number in (0, 1), never 0 or 1.
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub(crate) fn next(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let radius = libm::sqrt(-2.0 * libm::log(self.uniform()));
        let angle = 2.0 * std::f64::consts::PI * self.uniform();
        self.spare = Some(radius * libm::sin(angle));
        radius * libm::cos(angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_numbers_are_reproducible_and_standard() {
        let mut a = Normal::new(42);
        let mut b = Normal::new(42);
        let draws: Vec<f64> = (0..20_000).map(|_| a.next()).collect();
        assert!(
            draws
                .iter()
                .zip((0..20_000).map(|_| b.next()))
                .all(|(x, y)| *x == y)
        );
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        let variance =
            draws.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (draws.len() as f64 - 1.0);
        assert!(mean.abs() < 0.03 && (variance - 1.0).abs() < 0.03);
    }

    #[test]
    fn invalid_resampling_is_reported() {
        let c = Matrix::new(vec!["a".into(), "b".into()], vec![0.04, 0.0, 0.0, 0.09]).unwrap();
        let bad = Resampling {
            samples: 1,
            observations: 1,
            seed: 0,
        };
        let error = Problem::new(c)
            .resampled(&Objective::MinimumVariance, &bad)
            .unwrap_err();
        assert!(error.to_string().contains("resampling.samples"), "{error}");
        assert!(
            error.to_string().contains("resampling.observations"),
            "{error}"
        );
    }
}
