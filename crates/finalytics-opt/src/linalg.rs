//! Dense linear algebra for the formulations: a factor of a covariance matrix and Cholesky
//! solves.

use finalytics_core::Result;
use finalytics_risk::Matrix;

use crate::invalid;

/// A factor `R` (`rank` rows by `n` columns) of a positive semidefinite matrix `C`, with
/// `R'R = C` up to rounding, from a Cholesky factorisation with diagonal pivoting. Directions
/// with no variance are dropped, so a matrix with a riskless asset has a factor of lower rank.
pub(crate) struct Factor {
    pub(crate) rows: Vec<Vec<f64>>,
}

impl Factor {
    pub(crate) fn of(matrix: &Matrix) -> Result<Factor> {
        let n = matrix.dimension();
        let scale = matrix.diagonal().iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let tolerance = scale * 1e-13;
        let mut remaining = matrix.diagonal();
        let mut chosen = vec![false; n];
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for _ in 0..n {
            let Some(p) = (0..n)
                .filter(|&i| !chosen[i])
                .max_by(|&a, &b| remaining[a].total_cmp(&remaining[b]))
            else {
                break;
            };
            if remaining[p] <= tolerance {
                break;
            }
            chosen[p] = true;
            let pivot = remaining[p].sqrt();
            let mut row = vec![0.0; n];
            for i in 0..n {
                if chosen[i] && i != p {
                    continue;
                }
                let earlier: f64 = rows.iter().map(|r| r[i] * r[p]).sum();
                row[i] = (matrix.get(i, p) - earlier) / pivot;
            }
            for i in 0..n {
                if !chosen[i] {
                    remaining[i] -= row[i] * row[i];
                }
            }
            rows.push(row);
        }
        let worst = (0..n)
            .filter(|&i| !chosen[i])
            .map(|i| remaining[i])
            .fold(0.0f64, f64::min);
        if worst < -1e-8 * scale.max(f64::MIN_POSITIVE) {
            return Err(invalid(
                "covariance",
                "is not positive semidefinite, so it is not a valid covariance matrix",
            ));
        }
        Ok(Factor { rows })
    }

    /// `R x`.
    pub(crate) fn times(&self, x: &[f64]) -> Vec<f64> {
        self.rows
            .iter()
            .map(|r| r.iter().zip(x).map(|(a, b)| a * b).sum())
            .collect()
    }
}

/// Solves `A x = b` for a symmetric positive definite `A` (`n` by `n`, row by row), overwriting
/// `b` with `x`. Returns `false` if `A` is not positive definite.
pub(crate) fn cholesky_solve(a: &[f64], n: usize, b: &mut [f64]) -> bool {
    let mut l = vec![0.0; n * n];
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= l[j * n + k] * l[j * n + k];
        }
        if d.is_nan() || d <= 0.0 {
            return false;
        }
        let d = d.sqrt();
        l[j * n + j] = d;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            l[i * n + j] = s / d;
        }
    }
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * n + k] * b[k];
        }
        b[i] = s / l[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= l[k * n + i] * b[k];
        }
        b[i] = s / l[i * n + i];
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(values: Vec<f64>) -> Matrix {
        let n = (values.len() as f64).sqrt() as usize;
        Matrix::new((0..n).map(|i| format!("a{i}")).collect(), values).unwrap()
    }

    #[test]
    fn factor_reproduces_the_matrix() {
        let c = matrix(vec![
            0.04, 0.006, 0.0, 0.002, //
            0.006, 0.09, 0.0, 0.003, //
            0.0, 0.0, 0.0, 0.0, //
            0.002, 0.003, 0.0, 0.0225,
        ]);
        let f = Factor::of(&c).unwrap();
        assert_eq!(f.rows.len(), 3);
        for i in 0..4 {
            for j in 0..4 {
                let rr: f64 = f.rows.iter().map(|r| r[i] * r[j]).sum();
                assert!((rr - c.get(i, j)).abs() < 1e-17, "({i}, {j})");
            }
        }
        let x = [0.3, -0.2, 0.5, 0.4];
        let rx = f.times(&x);
        let norm: f64 = rx.iter().map(|v| v * v).sum();
        assert!((norm - c.quadratic_form(&x).unwrap()).abs() < 1e-17);
    }

    #[test]
    fn indefinite_matrices_are_rejected() {
        assert!(Factor::of(&matrix(vec![1.0, 2.0, 2.0, 1.0])).is_err());
    }

    #[test]
    fn solves_positive_definite_systems() {
        let a = [4.0, 1.0, 1.0, 3.0];
        let mut b = [1.0, 2.0];
        assert!(cholesky_solve(&a, 2, &mut b));
        assert!((4.0 * b[0] + b[1] - 1.0).abs() < 1e-15);
        assert!((b[0] + 3.0 * b[1] - 2.0).abs() < 1e-15);
        assert!(!cholesky_solve(&[1.0, 2.0, 2.0, 1.0], 2, &mut [1.0, 1.0]));
    }
}
