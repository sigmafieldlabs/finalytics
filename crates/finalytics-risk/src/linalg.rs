//! Dense linear algebra for the factor models: least squares by Householder QR, and the
//! eigendecomposition of a symmetric matrix.

/// Least squares with one design matrix and any number of right-hand sides, by Householder QR
/// (Golub and Van Loan, *Matrix Computations*, §5.2).
pub(crate) struct LeastSquares {
    /// The design matrix, column by column.
    columns: Vec<Vec<f64>>,
    /// The factor each column was divided by (its largest magnitude) before factorising.
    scales: Vec<f64>,
    /// The Householder vector of each column, from its diagonal entry down.
    reflectors: Vec<Vec<f64>>,
    /// `2 / v'v` for each reflector, or 0 for an identity reflection.
    betas: Vec<f64>,
    /// The upper triangle `R`, row by row (`p` by `p`).
    r: Vec<f64>,
}

impl LeastSquares {
    /// Factorises the design matrix given as `columns`, each with one entry per observation.
    /// Returns the index of the first column that depends linearly on the earlier ones, if any.
    pub(crate) fn new(columns: &[Vec<f64>]) -> Result<LeastSquares, usize> {
        let p = columns.len();
        let t = columns.first().map_or(0, Vec::len);
        // Each column is scaled to a largest magnitude of one, which keeps the arithmetic in
        // range for any finite input and makes the rank test independent of units.
        let scales: Vec<f64> = columns
            .iter()
            .map(|c| c.iter().fold(0.0f64, |m, x| m.max(x.abs())))
            .collect();
        if let Some(j) = scales.iter().position(|s| !(*s > 0.0 && s.is_finite())) {
            return Err(j);
        }
        let mut work: Vec<Vec<f64>> = columns
            .iter()
            .zip(&scales)
            .map(|(c, s)| c.iter().map(|x| x / s).collect())
            .collect();
        let mut reflectors = Vec::with_capacity(p);
        let mut betas = Vec::with_capacity(p);
        let mut r = vec![0.0; p * p];
        for j in 0..p {
            if j >= t {
                return Err(j);
            }
            // A column is dependent when little of it is left after removing its projection on
            // the earlier ones, relative to its own size.
            let own = work[j].iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm = work[j][j..].iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm.is_nan() || norm <= 1e-12 * own {
                return Err(j);
            }
            let alpha = if work[j][j] > 0.0 { -norm } else { norm };
            let mut v = work[j][j..].to_vec();
            v[0] -= alpha;
            let vv: f64 = v.iter().map(|x| x * x).sum();
            let beta = if vv == 0.0 { 0.0 } else { 2.0 / vv };
            r[j * p + j] = alpha;
            for k in j + 1..p {
                let s: f64 = v.iter().zip(&work[k][j..]).map(|(a, b)| a * b).sum();
                for (x, vi) in work[k][j..].iter_mut().zip(&v) {
                    *x -= beta * s * vi;
                }
                r[j * p + k] = work[k][j];
            }
            reflectors.push(v);
            betas.push(beta);
        }
        Ok(LeastSquares {
            columns: columns.to_vec(),
            scales,
            reflectors,
            betas,
            r,
        })
    }

    /// The coefficients `b` minimising `|y - X b|`, and the residuals `y - X b`.
    pub(crate) fn solve(&self, y: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let p = self.columns.len();
        let mut qy = y.to_vec();
        for (j, (v, beta)) in self.reflectors.iter().zip(&self.betas).enumerate() {
            let s: f64 = v.iter().zip(&qy[j..]).map(|(a, b)| a * b).sum();
            for (x, vi) in qy[j..].iter_mut().zip(v) {
                *x -= beta * s * vi;
            }
        }
        let mut b = vec![0.0; p];
        for i in (0..p).rev() {
            let later: f64 = self.r[i * p + i + 1..(i + 1) * p]
                .iter()
                .zip(&b[i + 1..])
                .map(|(r, x)| r * x)
                .sum();
            b[i] = (qy[i] - later) / self.r[i * p + i];
        }
        for (bi, scale) in b.iter_mut().zip(&self.scales) {
            *bi /= scale;
        }
        let residuals = y
            .iter()
            .enumerate()
            .map(|(t, yt)| {
                yt - self
                    .columns
                    .iter()
                    .zip(&b)
                    .map(|(c, bk)| c[t] * bk)
                    .sum::<f64>()
            })
            .collect();
        (b, residuals)
    }
}

/// The solution of `A x = b` for a symmetric positive definite `A` (`n` by `n`, row by row), by
/// Cholesky factorisation; `None` if `A` is not positive definite.
pub(crate) fn solve_symmetric(a: &[f64], n: usize, b: &[f64]) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= l[j * n + k] * l[j * n + k];
        }
        if d.is_nan() || d <= 0.0 {
            return None;
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
    let mut x = b.to_vec();
    for i in 0..n {
        let mut s = x[i];
        for k in 0..i {
            s -= l[i * n + k] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = x[i];
        for k in i + 1..n {
            s -= l[k * n + i] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    Some(x)
}

/// The eigenvalues of a symmetric matrix (`n` by `n`, row by row), in decreasing order, with
/// unit eigenvectors as rows in the same order. Each eigenvector's entry of largest magnitude
/// (the first such entry, on ties) is positive, the convention of scikit-learn's PCA.
///
/// Householder reduction to tridiagonal form followed by the implicit QL algorithm (the EISPACK
/// routines `tred2` and `tql2`). Returns `None` if the iteration fails to converge, which does
/// not happen for finite input in practice.
pub(crate) fn symmetric_eigen(matrix: &[f64], n: usize) -> Option<(Vec<f64>, Vec<Vec<f64>>)> {
    if n == 0 {
        return Some((Vec::new(), Vec::new()));
    }
    // `v[i][j]`, row by row; after the reduction its columns hold the transformation.
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|i| matrix[i * n..(i + 1) * n].to_vec())
        .collect();
    let mut d = vec![0.0; n];
    let mut e = vec![0.0; n];
    tridiagonalise(&mut v, &mut d, &mut e);
    // The QL rotations combine pairs of columns; on the transpose they combine pairs of
    // contiguous rows, which is much faster for large matrices.
    let mut vt: Vec<Vec<f64>> = (0..n).map(|j| (0..n).map(|i| v[i][j]).collect()).collect();
    drop(v);
    diagonalise(&mut vt, &mut d, &mut e)?;

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| d[b].total_cmp(&d[a]));
    let values = order.iter().map(|&k| d[k]).collect();
    let vectors = order
        .iter()
        .map(|&k| {
            let mut column = vt[k].clone();
            let largest = column
                .iter()
                .enumerate()
                .fold((0, 0.0f64), |best, (i, x)| {
                    if x.abs() > best.1 { (i, x.abs()) } else { best }
                })
                .0;
            if column[largest] < 0.0 {
                column.iter_mut().for_each(|x| *x = -*x);
            }
            column
        })
        .collect();
    Some((values, vectors))
}

/// Householder reduction of the symmetric matrix in `v` to tridiagonal form: `d` gets the
/// diagonal, `e` the subdiagonal (in `e[1..]`), and `v` the orthogonal transformation.
fn tridiagonalise(v: &mut [Vec<f64>], d: &mut [f64], e: &mut [f64]) {
    let n = d.len();
    d.copy_from_slice(&v[n - 1]);
    for i in (1..n).rev() {
        let scale: f64 = d[..i].iter().map(|x| x.abs()).sum();
        let mut h = 0.0;
        if scale == 0.0 {
            e[i] = d[i - 1];
            for j in 0..i {
                d[j] = v[i - 1][j];
                v[i][j] = 0.0;
                v[j][i] = 0.0;
            }
        } else {
            for x in &mut d[..i] {
                *x /= scale;
                h += *x * *x;
            }
            let mut f = d[i - 1];
            let mut g = if f > 0.0 { -h.sqrt() } else { h.sqrt() };
            e[i] = scale * g;
            h -= f * g;
            d[i - 1] = f - g;
            e[..i].iter_mut().for_each(|x| *x = 0.0);
            for j in 0..i {
                f = d[j];
                v[j][i] = f;
                g = e[j] + v[j][j] * f;
                for k in j + 1..i {
                    g += v[k][j] * d[k];
                    e[k] += v[k][j] * f;
                }
                e[j] = g;
            }
            f = 0.0;
            for j in 0..i {
                e[j] /= h;
                f += e[j] * d[j];
            }
            let hh = f / (h + h);
            for j in 0..i {
                e[j] -= hh * d[j];
            }
            for j in 0..i {
                f = d[j];
                g = e[j];
                for k in j..i {
                    v[k][j] -= f * e[k] + g * d[k];
                }
                d[j] = v[i - 1][j];
                v[i][j] = 0.0;
            }
        }
        d[i] = h;
    }
    for i in 0..n - 1 {
        v[n - 1][i] = v[i][i];
        v[i][i] = 1.0;
        let h = d[i + 1];
        if h != 0.0 {
            for k in 0..=i {
                d[k] = v[k][i + 1] / h;
            }
            for j in 0..=i {
                let g: f64 = (0..=i).map(|k| v[k][i + 1] * v[k][j]).sum();
                for k in 0..=i {
                    v[k][j] -= g * d[k];
                }
            }
        }
        for row in v.iter_mut().take(i + 1) {
            row[i + 1] = 0.0;
        }
    }
    d.copy_from_slice(&v[n - 1]);
    v[n - 1].fill(0.0);
    v[n - 1][n - 1] = 1.0;
    e[0] = 0.0;
}

/// The implicit QL algorithm on the tridiagonal matrix in `d` and `e`, accumulating the
/// rotations into the rows of `vt`, the transpose of the reduction's transformation. On return
/// `d` holds the eigenvalues, unsorted, and row `k` of `vt` the eigenvector of `d[k]`.
fn diagonalise(vt: &mut [Vec<f64>], d: &mut [f64], e: &mut [f64]) -> Option<()> {
    let n = d.len();
    for i in 1..n {
        e[i - 1] = e[i];
    }
    e[n - 1] = 0.0;
    let mut f = 0.0;
    let mut tst1 = 0.0f64;
    let eps = f64::EPSILON;
    for l in 0..n {
        tst1 = tst1.max(d[l].abs() + e[l].abs());
        let mut m = l;
        while m < n && e[m].abs() > eps * tst1 {
            m += 1;
        }
        if m > l {
            let mut iterations = 0;
            loop {
                iterations += 1;
                if iterations > 60 {
                    return None;
                }
                let mut g = d[l];
                let mut p = (d[l + 1] - g) / (2.0 * e[l]);
                let mut r = p.hypot(1.0);
                if p < 0.0 {
                    r = -r;
                }
                d[l] = e[l] / (p + r);
                d[l + 1] = e[l] * (p + r);
                let dl1 = d[l + 1];
                let mut h = g - d[l];
                for x in &mut d[l + 2..] {
                    *x -= h;
                }
                f += h;
                p = d[m];
                let (mut c, mut c2, mut c3) = (1.0, 1.0, 1.0);
                let el1 = e[l + 1];
                let (mut s, mut s2) = (0.0, 0.0);
                for i in (l..m).rev() {
                    c3 = c2;
                    c2 = c;
                    s2 = s;
                    g = c * e[i];
                    h = c * p;
                    r = p.hypot(e[i]);
                    e[i + 1] = s * r;
                    s = e[i] / r;
                    c = p / r;
                    p = c * d[i] - s * g;
                    d[i + 1] = h + s * (c * g + s * d[i]);
                    let (low, high) = vt.split_at_mut(i + 1);
                    for (a, b) in low[i].iter_mut().zip(high[0].iter_mut()) {
                        let h = *b;
                        *b = s * *a + c * h;
                        *a = c * *a - s * h;
                    }
                }
                p = -s * s2 * c3 * el1 * e[l] / dl1;
                e[l] = s * p;
                d[l] = c * p;
                if e[l].abs() <= eps * tst1 {
                    break;
                }
            }
        }
        d[l] += f;
        e[l] = 0.0;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn least_squares_recovers_coefficients() {
        let x0 = vec![1.0; 6];
        let x1 = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f64> = x1.iter().map(|x| 2.0 - 0.5 * x).collect();
        let ls = LeastSquares::new(&[x0, x1]).unwrap();
        let (b, residuals) = ls.solve(&y);
        assert!((b[0] - 2.0).abs() < 1e-14 && (b[1] + 0.5).abs() < 1e-14);
        assert!(residuals.iter().all(|r| r.abs() < 1e-14));
    }

    #[test]
    fn columns_of_very_different_sizes_are_independent() {
        let ones = vec![1.0; 4];
        let large = vec![1e300, -2e300, 3e299, 5e299];
        let ls = LeastSquares::new(&[ones, large]).unwrap();
        let (b, _) = ls.solve(&[1.0, 1.0, 1.0, 1.0]);
        assert!((b[0] - 1.0).abs() < 1e-12 && b[1].abs() < 1e-300);
    }

    #[test]
    fn dependent_columns_are_reported() {
        let x0 = vec![1.0, 2.0, 3.0];
        let x1 = vec![2.0, 4.0, 6.0];
        assert_eq!(LeastSquares::new(&[x0, x1]).err(), Some(1));
    }

    #[test]
    fn eigendecomposition_reconstructs_the_matrix() {
        let a = [
            4.0, 1.0, 0.5, 0.0, //
            1.0, 3.0, 0.2, 0.1, //
            0.5, 0.2, 2.0, 0.3, //
            0.0, 0.1, 0.3, 1.0,
        ];
        let (values, vectors) = symmetric_eigen(&a, 4).unwrap();
        assert!(values.windows(2).all(|w| w[0] >= w[1]));
        for i in 0..4 {
            for j in 0..4 {
                let rebuilt: f64 = (0..4)
                    .map(|k| values[k] * vectors[k][i] * vectors[k][j])
                    .sum();
                assert!((rebuilt - a[i * 4 + j]).abs() < 1e-14, "({i}, {j})");
                let dot: f64 = (0..4).map(|k| vectors[i][k] * vectors[j][k]).sum();
                assert!((dot - f64::from(u8::from(i == j))).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn one_by_one_and_diagonal_matrices() {
        let (values, vectors) = symmetric_eigen(&[2.5], 1).unwrap();
        assert_eq!((values, vectors), (vec![2.5], vec![vec![1.0]]));
        let (values, _) = symmetric_eigen(&[1.0, 0.0, 0.0, 3.0], 2).unwrap();
        assert_eq!(values, vec![3.0, 1.0]);
    }
}
