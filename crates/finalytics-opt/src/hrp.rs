//! Hierarchical risk parity (López de Prado, 2016).

use finalytics_core::Result;
use finalytics_risk::Matrix;

use crate::invalid;
use crate::problem::Linkage;

/// Hierarchical risk parity weights:
///
/// 1. the distance between assets `i` and `j` is `sqrt((1 - rho_ij) / 2)`;
/// 2. assets are clustered agglomeratively with `linkage`, and ordered by the leaves of the
///    tree, left branch first, with the earlier-formed cluster on the left;
/// 3. weights are split by recursive bisection of that order: each half receives a share
///    inversely proportional to its variance under inverse-variance weights within it.
pub(crate) fn weights(covariance: &Matrix, linkage: Linkage) -> Result<Vec<f64>> {
    let n = covariance.dimension();
    let correlation = covariance.correlation().map_err(|_| {
        invalid(
            "covariance",
            "every asset needs variance for hierarchical risk parity; remove riskless assets",
        )
    })?;
    let mut distance = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            distance[i * n + j] = ((1.0 - correlation.get(i, j)) / 2.0).clamp(0.0, 1.0).sqrt();
        }
    }
    let order = leaf_order(&distance, n, linkage);

    let mut weights = vec![1.0; n];
    let mut clusters: Vec<&[usize]> = vec![&order];
    while !clusters.is_empty() {
        let mut next = Vec::new();
        for cluster in clusters {
            if cluster.len() < 2 {
                continue;
            }
            let (left, right) = cluster.split_at(cluster.len() / 2);
            let left_variance = cluster_variance(covariance, left);
            let right_variance = cluster_variance(covariance, right);
            let alpha = 1.0 - left_variance / (left_variance + right_variance);
            left.iter().for_each(|&i| weights[i] *= alpha);
            right.iter().for_each(|&i| weights[i] *= 1.0 - alpha);
            next.push(left);
            next.push(right);
        }
        clusters = next;
    }
    Ok(weights)
}

/// The variance of a cluster under inverse-variance weights.
fn cluster_variance(covariance: &Matrix, members: &[usize]) -> f64 {
    let inverse: Vec<f64> = members
        .iter()
        .map(|&i| 1.0 / covariance.get(i, i))
        .collect();
    let total: f64 = inverse.iter().sum();
    let w: Vec<f64> = inverse.iter().map(|v| v / total).collect();
    let mut variance = 0.0;
    for (a, &i) in members.iter().enumerate() {
        for (b, &j) in members.iter().enumerate() {
            variance += w[a] * w[b] * covariance.get(i, j);
        }
    }
    variance
}

/// The order of the leaves of the agglomerative clustering of a distance matrix.
///
/// Clusters are numbered as in SciPy: the assets are `0..n` and the cluster formed at merge `k`
/// is `n + k`; each merge joins the closest pair, with the lower-numbered cluster on the left.
fn leaf_order(distance: &[f64], n: usize, linkage: Linkage) -> Vec<usize> {
    if n == 1 {
        return vec![0];
    }
    // `d` holds distances between active clusters, indexed by slot; `id` and `size` describe the
    // cluster in each slot.
    let mut d = distance.to_vec();
    let mut id: Vec<usize> = (0..n).collect();
    let mut size = vec![1usize; n];
    let mut active = vec![true; n];
    let mut children: Vec<(usize, usize)> = Vec::with_capacity(n - 1);
    for k in 0..n - 1 {
        let mut best = (f64::INFINITY, 0, 0);
        for i in 0..n {
            if !active[i] {
                continue;
            }
            for j in i + 1..n {
                if active[j] && d[i * n + j] < best.0 {
                    best = (d[i * n + j], i, j);
                }
            }
        }
        let (dij, i, j) = best;
        let (left, right) = if id[i] < id[j] {
            (id[i], id[j])
        } else {
            (id[j], id[i])
        };
        children.push((left, right));
        // The merged cluster takes slot i; slot j is retired.
        for m in 0..n {
            if !active[m] || m == i || m == j {
                continue;
            }
            let (dim, djm) = (d[i * n + m], d[j * n + m]);
            let (si, sj, sm) = (size[i] as f64, size[j] as f64, size[m] as f64);
            let merged = match linkage {
                Linkage::Single => dim.min(djm),
                Linkage::Complete => dim.max(djm),
                Linkage::Average => (si * dim + sj * djm) / (si + sj),
                Linkage::Ward => {
                    let t = si + sj + sm;
                    (((si + sm) * dim * dim + (sj + sm) * djm * djm - sm * dij * dij) / t).sqrt()
                }
            };
            d[i * n + m] = merged;
            d[m * n + i] = merged;
        }
        active[j] = false;
        id[i] = n + k;
        size[i] += size[j];
    }

    let mut order = Vec::with_capacity(n);
    let mut stack = vec![2 * n - 2];
    while let Some(node) = stack.pop() {
        if node < n {
            order.push(node);
        } else {
            let (left, right) = children[node - n];
            stack.push(right);
            stack.push(left);
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_assets_split_by_inverse_variance() {
        let c = Matrix::new(vec!["a".into(), "b".into()], vec![0.04, 0.01, 0.01, 0.01]).unwrap();
        let w = weights(&c, Linkage::Single).unwrap();
        assert!((w[0] - 0.2).abs() < 1e-15 && (w[1] - 0.8).abs() < 1e-15);
    }

    #[test]
    fn leaves_follow_the_tree() {
        // Assets 0 and 2 are closest, then 1 joins them, then 3.
        let d = [
            0.0, 0.5, 0.1, 0.9, //
            0.5, 0.0, 0.6, 0.8, //
            0.1, 0.6, 0.0, 0.7, //
            0.9, 0.8, 0.7, 0.0,
        ];
        assert_eq!(leaf_order(&d, 4, Linkage::Single), [3, 1, 0, 2]);
        assert_eq!(leaf_order(&d, 4, Linkage::Complete), [3, 1, 0, 2]);
    }
}
