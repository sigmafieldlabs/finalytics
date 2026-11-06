//! Reproducible random numbers: SplitMix64 (Steele, Lea and Flood, 2014), with uniform numbers
//! and standard normal numbers by the Box–Muller transform. The functions come from `libm`, so
//! every platform gives the same numbers.
//!
//! Simulations that run their draws on several threads give each draw a stream of its own with
//! [`stream_seed`]: the `k`-th number of the generator started at the seed. A draw's numbers then
//! depend only on the seed and the draw's number, never on the order the draws run in.
//!
//! ```
//! use finalytics_core::random::{Random, stream_seed};
//!
//! let mut a = Random::new(stream_seed(7, 3));
//! let mut b = Random::new(stream_seed(7, 3));
//! assert_eq!(a.normal(), b.normal());
//! let u = a.uniform();
//! assert!(u > 0.0 && u < 1.0);
//! ```

/// SplitMix64's increment, the golden ratio's fractional part in 64 bits.
const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

/// A SplitMix64 generator, with uniform and standard normal numbers.
#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
    spare: Option<f64>,
}

impl Random {
    /// A generator started at `seed`.
    pub fn new(seed: u64) -> Random {
        Random {
            state: seed,
            spare: None,
        }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniform number in (0, 1), never 0 or 1.
    pub fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    /// A whole number from 0 up to, but not including, `n`, which must be at least 1.
    pub fn below(&mut self, n: usize) -> usize {
        // The uniform number is never 1, so the product is below n.
        ((self.uniform() * n as f64) as usize).min(n - 1)
    }

    /// A standard normal number.
    pub fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let radius = libm::sqrt(-2.0 * libm::log(self.uniform()));
        let angle = 2.0 * std::f64::consts::PI * self.uniform();
        self.spare = Some(radius * libm::sin(angle));
        radius * libm::cos(angle)
    }
}

/// The seed of stream `k`: the `k`-th number (counting from 0) that SplitMix64 gives from
/// `seed`.
pub fn stream_seed(seed: u64, k: usize) -> u64 {
    Random::new(seed.wrapping_add(GOLDEN.wrapping_mul(k as u64))).next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_numbers_are_reproducible_and_standard() {
        let mut a = Random::new(42);
        let mut b = Random::new(42);
        let draws: Vec<f64> = (0..20_000).map(|_| a.normal()).collect();
        assert!(
            draws
                .iter()
                .zip((0..20_000).map(|_| b.normal()))
                .all(|(x, y)| *x == y)
        );
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        let variance =
            draws.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (draws.len() as f64 - 1.0);
        assert!(mean.abs() < 0.03 && (variance - 1.0).abs() < 0.03);
    }

    #[test]
    fn each_stream_has_a_seed_of_its_own() {
        // The k-th seed is the k-th number of the generator started at the seed.
        let mut stream = Random::new(2026);
        for k in 0..5 {
            assert_eq!(stream_seed(2026, k), stream.next_u64());
        }
        assert_ne!(stream_seed(2026, 0), stream_seed(2027, 0));
    }

    #[test]
    fn whole_numbers_stay_in_range_and_cover_it() {
        let mut r = Random::new(9);
        let mut seen = [false; 7];
        for _ in 0..1_000 {
            let k = r.below(7);
            assert!(k < 7);
            seen[k] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }
}
