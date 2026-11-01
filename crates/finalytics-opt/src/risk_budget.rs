//! Risk budgeting by Newton's method on the convex formulation of Spinu (2013), with the
//! interior-point solver as a fallback.

use finalytics_core::Result;
use finalytics_risk::Matrix;

use crate::invalid;
use crate::linalg::cholesky_solve;
use crate::program::{Program, Row};
use crate::solution::{Method, SolveStatus};

pub(crate) struct Found {
    pub(crate) status: SolveStatus,
    pub(crate) weights: Option<Vec<f64>>,
    pub(crate) method: Method,
    pub(crate) iterations: u32,
}

/// Weights whose shares of volatility are the normalised `budgets`.
///
/// Minimises `f(x) = x'Cx / 2 - sum b_i ln x_i` over `x > 0`, which is strictly convex; at its
/// minimum `x_i (Cx)_i = b_i`, so `w = x / sum x` has risk shares equal to the budgets.
pub(crate) fn solve(covariance: &Matrix, budgets: &[f64]) -> Result<Found> {
    let n = covariance.dimension();
    let variances = covariance.diagonal();
    if let Some(i) = variances.iter().position(|v| v.is_nan() || *v <= 0.0) {
        return Err(invalid(
            "covariance",
            format!(
                "{} has no variance, so it cannot take a share of the risk; remove it before \
                 risk budgeting",
                covariance.labels()[i]
            ),
        ));
    }
    let total: f64 = budgets.iter().sum();
    let b: Vec<f64> = budgets.iter().map(|x| x / total).collect();

    if let Some((x, iterations)) = newton(covariance, &b) {
        return Ok(Found {
            status: SolveStatus::Optimal,
            weights: Some(normalise(&x)),
            method: Method::Newton,
            iterations,
        });
    }
    let outcome = conic(covariance, &b, n).solve();
    let weights = outcome
        .status
        .has_solution()
        .then(|| normalise(&outcome.x[..n]));
    Ok(Found {
        status: outcome.status,
        weights,
        method: Method::InteriorPoint,
        iterations: outcome.iterations,
    })
}

fn objective(covariance: &Matrix, b: &[f64], x: &[f64]) -> Option<f64> {
    let quadratic = covariance.quadratic_form(x).ok()?;
    Some(0.5 * quadratic - b.iter().zip(x).map(|(b, x)| b * x.ln()).sum::<f64>())
}

/// Damped Newton's method with a backtracking line search. Returns `None` if it does not
/// converge, so that the caller can fall back to the conic formulation.
fn newton(covariance: &Matrix, b: &[f64]) -> Option<(Vec<f64>, u32)> {
    let n = b.len();
    // Start from inverse-volatility weights scaled so that x'Cx = 1, which holds at the optimum.
    let mut x: Vec<f64> = covariance
        .diagonal()
        .iter()
        .zip(b)
        .map(|(v, b)| b / v.sqrt())
        .collect();
    let scale = covariance.quadratic_form(&x).ok()?.sqrt();
    x.iter_mut().for_each(|v| *v /= scale);
    let mut value = objective(covariance, b, &x)?;

    for iteration in 1..=100 {
        let cx = covariance.times(&x).ok()?;
        let gradient: Vec<f64> = (0..n).map(|i| cx[i] - b[i] / x[i]).collect();
        let mut hessian = covariance.values().to_vec();
        for i in 0..n {
            hessian[i * n + i] += b[i] / (x[i] * x[i]);
        }
        let mut step: Vec<f64> = gradient.iter().map(|g| -g).collect();
        if !cholesky_solve(&hessian, n, &mut step) {
            return None;
        }
        // The squared Newton decrement, g' H^-1 g, bounds the distance to the optimum.
        let decrement: f64 = -gradient.iter().zip(&step).map(|(g, s)| g * s).sum::<f64>();
        if decrement < 1e-24 {
            return Some((x, iteration));
        }
        let mut t = 1.0;
        loop {
            let candidate: Vec<f64> = x.iter().zip(&step).map(|(x, s)| x + t * s).collect();
            if candidate.iter().all(|v| *v > 0.0)
                && let Some(next) = objective(covariance, b, &candidate)
                && next <= value - 0.25 * t * decrement
            {
                x = candidate;
                value = next;
                break;
            }
            t *= 0.5;
            if t < 1e-12 {
                // No further decrease is possible in floating point: accept the point if the
                // gradient is already negligible.
                return (decrement < 1e-16).then_some((x, iteration));
            }
        }
    }
    None
}

/// The same problem as a conic programme: minimise `x'Cx / 2 - b't` with `t_i <= ln x_i`, that
/// is `(t_i, 1, x_i)` in the exponential cone.
fn conic(covariance: &Matrix, b: &[f64], n: usize) -> Program {
    let mut program = Program::default();
    let x = program.variables(n);
    let t = program.variables(n);
    program.quadratic(x, covariance, 1.0);
    for (i, budget) in b.iter().enumerate() {
        program.linear[t + i] = -budget;
        program.exponential_cone([
            Row::new(vec![(t + i, -1.0)], 0.0),
            Row::new(Vec::new(), 1.0),
            Row::new(vec![(x + i, -1.0)], 0.0),
        ]);
    }
    program
}

fn normalise(x: &[f64]) -> Vec<f64> {
    let total: f64 = x.iter().sum();
    x.iter().map(|v| v / total).collect()
}

/// The largest difference between an asset's share of volatility and its budget.
pub(crate) fn largest_gap(covariance: &Matrix, w: &[f64], budgets: Option<&[f64]>) -> Option<f64> {
    let n = w.len();
    let equal = vec![1.0; n];
    let budgets = budgets.unwrap_or(&equal);
    let total: f64 = budgets.iter().sum();
    let cw = covariance.times(w).ok()?;
    let variance: f64 = cw.iter().zip(w).map(|(a, b)| a * b).sum();
    (variance > 0.0).then(|| {
        (0..n)
            .map(|i| (w[i] * cw[i] / variance - budgets[i] / total).abs())
            .fold(0.0, f64::max)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covariance() -> Matrix {
        Matrix::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec![
                0.040, 0.006, 0.010, //
                0.006, 0.090, 0.012, //
                0.010, 0.012, 0.0225,
            ],
        )
        .unwrap()
    }

    #[test]
    fn newton_and_the_conic_formulation_agree() {
        let c = covariance();
        let b = [0.5, 0.3, 0.2];
        let (x, _) = newton(&c, &b).unwrap();
        let w = normalise(&x);
        assert!(largest_gap(&c, &w, Some(&b)).unwrap() < 1e-12);

        let outcome = conic(&c, &b, 3).solve();
        assert!(outcome.status.has_solution());
        // The interior-point solution is accurate to the solver's tolerance, not to rounding.
        let v = normalise(&outcome.x[..3]);
        for (a, b) in w.iter().zip(&v) {
            assert!((a - b).abs() < 1e-4, "{a} against {b}");
        }
        assert!(largest_gap(&c, &v, Some(&b)).unwrap() < 1e-4);
    }

    #[test]
    fn riskless_assets_are_rejected() {
        let c = Matrix::new(vec!["a".into(), "cash".into()], vec![0.04, 0.0, 0.0, 0.0]).unwrap();
        assert!(solve(&c, &[1.0, 1.0]).is_err());
    }
}
