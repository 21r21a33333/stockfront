//! Time-bounded memoisation of upstream reads.
//!
//! One Stockfront request asks the same provider question several times — the
//! bStock catalogue and the Venus market list are each read by most services —
//! and those answers move far more slowly than the requests that consume them.
//! Holding each answer for a short window turns the repeats into memory reads.
//!
//! Misses that arrive together are collapsed into a single upstream call, so a
//! cold cache cannot turn a burst of page loads into a burst of provider load.

use std::{future::Future, hash::Hash, time::Duration};

use moka::future::Cache;

pub(crate) struct TtlCache<K, V> {
    entries: Cache<K, V>,
}

impl<K, V> TtlCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub(crate) fn new(capacity: u64, ttl: Duration) -> Self {
        Self {
            entries: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    /// Returns the cached value, loading it when absent or expired.
    ///
    /// Failures are never stored: the next caller retries the provider rather
    /// than being served an outage for the rest of the window.
    pub(crate) async fn get_or_load<E, F>(&self, key: K, load: F) -> Result<V, E>
    where
        E: Copy + Send + Sync + 'static,
        F: Future<Output = Result<V, E>>,
    {
        self.entries
            .try_get_with(key, load)
            .await
            .map_err(|error| *error)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[tokio::test]
    async fn concurrent_misses_load_once_and_failures_are_not_retained() {
        let cache = TtlCache::<(), usize>::new(4, Duration::from_mins(1));
        let calls = Arc::new(AtomicUsize::new(0));
        let load = || {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(7)
            }
        };

        let failed = cache.get_or_load((), async { Err::<usize, _>(()) }).await;
        let (first, second) = futures_util::future::join(
            cache.get_or_load((), load()),
            cache.get_or_load((), load()),
        )
        .await;

        assert_eq!(failed, Err(()));
        assert_eq!((first, second), (Ok(7), Ok(7)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
