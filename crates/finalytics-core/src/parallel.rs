//! Independent pieces of work spread over threads, with results that do not depend on how many.
//!
//! With the `parallel` feature, the work runs on the current [`rayon`] thread pool: the global
//! pool, which uses every core unless `RAYON_NUM_THREADS` says otherwise, or a pool the caller
//! installs. Without it, the same work runs in order on the calling thread. Either way each result
//! is computed on its own and the results come back in the order of the items, so they are the
//! same to the last bit whatever the number of threads.
//!
//! [`rayon`]: https://docs.rs/rayon

/// Applies `f` to each of `items` and returns the results in the order of the items.
///
/// ```
/// let squares = finalytics_core::parallel::map(&[1, 2, 3], |x| x * x);
/// assert_eq!(squares, [1, 4, 9]);
/// ```
pub fn map<T, R, F>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        items.iter().map(f).collect()
    }
}

/// Whether the work runs on more than one thread: true with the `parallel` feature and a thread
/// pool of more than one thread.
pub fn is_parallel() -> bool {
    #[cfg(feature = "parallel")]
    {
        rayon::current_num_threads() > 1
    }
    #[cfg(not(feature = "parallel"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::map;

    #[test]
    fn results_come_back_in_the_order_of_the_items() {
        let items: Vec<u64> = (0..1_000).collect();
        let found = map(&items, |x| x * 3);
        assert_eq!(found, items.iter().map(|x| x * 3).collect::<Vec<_>>());
        assert!(map(&[] as &[u64], |x| *x).is_empty());
    }
}
