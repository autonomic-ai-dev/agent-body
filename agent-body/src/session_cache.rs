use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, OnceCell};

pub struct SessionCache<T> {
    entries: Mutex<HashMap<&'static str, Arc<OnceCell<Arc<T>>>>>,
}

impl<T> Default for SessionCache<T> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> SessionCache<T> {
    pub async fn get_or_connect<E, F>(&self, key: &'static str, connect: F) -> Result<Arc<T>, E>
    where
        F: Future<Output = Result<T, E>>,
    {
        let cell = {
            let mut entries = self.entries.lock().await;
            Arc::clone(entries.entry(key).or_default())
        };
        match cell
            .get_or_try_init(|| async { connect.await.map(Arc::new) })
            .await
        {
            Ok(session) => Ok(Arc::clone(session)),
            Err(err) => {
                self.forget(key, &cell).await;
                Err(err)
            }
        }
    }

    pub async fn evict(&self, key: &'static str, session: &Arc<T>) {
        let mut entries = self.entries.lock().await;
        let holds_session = entries
            .get(key)
            .and_then(|cell| cell.get())
            .is_some_and(|current| Arc::ptr_eq(current, session));
        if holds_session {
            entries.remove(key);
        }
    }

    async fn forget(&self, key: &'static str, cell: &Arc<OnceCell<Arc<T>>>) {
        let mut entries = self.entries.lock().await;
        if entries
            .get(key)
            .is_some_and(|current| Arc::ptr_eq(current, cell))
        {
            entries.remove(key);
        }
    }
}

pub fn retry_delay(attempt: u32) -> Duration {
    Duration::from_millis((50u64 << attempt.min(10)).min(1000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn a_slow_connect_for_one_organ_does_not_block_another() {
        let cache: Arc<SessionCache<u32>> = Arc::new(SessionCache::default());
        let slow = {
            let cache = Arc::clone(&cache);
            tokio::spawn(async move {
                cache
                    .get_or_connect("muscle", async {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        Ok::<_, String>(1)
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(10)).await;
        let fast = tokio::time::timeout(
            Duration::from_millis(100),
            cache.get_or_connect("heart", async { Ok::<_, String>(2) }),
        )
        .await
        .expect("heart must not wait for muscle's connect");
        assert_eq!(*fast.unwrap(), 2);
        assert_eq!(*slow.await.unwrap().unwrap(), 1);
    }

    #[tokio::test]
    async fn concurrent_callers_share_one_connect() {
        let cache: Arc<SessionCache<u32>> = Arc::new(SessionCache::default());
        let connects = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let connects = Arc::clone(&connects);
            handles.push(tokio::spawn(async move {
                cache
                    .get_or_connect("eyes", async move {
                        connects.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        Ok::<_, String>(7)
                    })
                    .await
                    .map(|s| *s)
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap().unwrap(), 7);
        }
        assert_eq!(connects.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failed_connect_is_retried_on_the_next_call() {
        let cache: SessionCache<u32> = SessionCache::default();
        let first = cache.get_or_connect("spine", async { Err::<u32, _>("down".to_string()) });
        assert!(first.await.is_err());
        let second = cache
            .get_or_connect("spine", async { Ok::<_, String>(3) })
            .await;
        assert_eq!(*second.unwrap(), 3);
    }

    #[tokio::test]
    async fn evicting_a_session_forces_a_reconnect() {
        let cache: SessionCache<u32> = SessionCache::default();
        let session = cache
            .get_or_connect("mouth", async { Ok::<_, String>(1) })
            .await
            .unwrap();
        cache.evict("mouth", &session).await;
        let fresh = cache
            .get_or_connect("mouth", async { Ok::<_, String>(2) })
            .await
            .unwrap();
        assert_eq!(*fresh, 2);
    }

    #[test]
    fn retry_delay_grows_then_caps() {
        assert_eq!(retry_delay(0), Duration::from_millis(50));
        assert_eq!(retry_delay(1), Duration::from_millis(100));
        assert_eq!(retry_delay(10), Duration::from_millis(1000));
    }
}
