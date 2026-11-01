//! The standard normal distribution.

use std::f64::consts::{FRAC_1_SQRT_2, PI};

/// The standard normal density at `x`.
pub fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// The standard normal distribution function: the probability of a value at or below `x`.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x * FRAC_1_SQRT_2)
}

/// The standard normal quantile: the value with probability `p` at or below it.
///
/// Returns negative or positive infinity for `p` of 0 or 1, and NaN outside `[0, 1]`. The result is
/// accurate to a few units in the last place: a rational first approximation (Acklam) is refined by
/// two Halley steps against [`normal_cdf`].
pub fn normal_quantile(p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    if p > 0.5 {
        // 1 - p is exact for p in (0.5, 1).
        return -lower_quantile(1.0 - p);
    }
    lower_quantile(p)
}

/// The quantile for `p` in (0, 0.5].
fn lower_quantile(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const P_LOW: f64 = 0.024_25;

    let mut x = if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    };
    for _ in 0..2 {
        let e = normal_cdf(x) - p;
        let u = e * (2.0 * PI).sqrt() * (0.5 * x * x).exp();
        x -= u / (1.0 + 0.5 * x * u);
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        assert_eq!(normal_quantile(0.5), 0.0);
        assert!((normal_quantile(0.975) - 1.959_963_984_540_054).abs() < 1e-15);
        assert!((normal_quantile(0.05) + 1.644_853_626_951_472_2).abs() < 1e-15);
        assert!((normal_cdf(1.959_963_984_540_054) - 0.975).abs() < 1e-16);
        assert!((normal_pdf(0.0) - 0.398_942_280_401_432_7).abs() < 1e-16);
        assert_eq!(normal_quantile(0.0), f64::NEG_INFINITY);
        assert!(normal_quantile(1.5).is_nan());
    }

    #[test]
    fn quantile_inverts_the_distribution_function() {
        for k in 1..1000 {
            let p = f64::from(k) / 1000.0;
            let x = normal_quantile(p);
            assert!((normal_cdf(x) - p).abs() < 1e-15 * p.max(1e-3) * 4.0 + 2e-16);
            assert!((normal_quantile(1.0 - p) + x).abs() < 1e-14);
        }
        for p in [1e-300, 1e-100, 1e-20, 1e-10] {
            let x = normal_quantile(p);
            assert!(((normal_cdf(x) - p) / p).abs() < 1e-13, "{p}");
        }
    }
}
