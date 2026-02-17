use crate::rate_limiter::algorithm::{AllowResult, BucketState, RateLimitAlgorithm};
use dashmap::DashMap;
use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct RateLimitError {
    pub retry_after: Duration,
    pub snapshot: BucketState,
}

#[derive(Clone)]
pub struct RateLimiter<K, A>
where
    A: RateLimitAlgorithm,
{
    buckets: Arc<DashMap<K, A>>,
    capacity: u128,
    refill_rate: u128,
}

impl<K, A> RateLimiter<K, A>
where
    K: Eq + std::hash::Hash,
    A: RateLimitAlgorithm,
{
    pub fn new(capacity: u128, refill_rate: u128) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            capacity,
            refill_rate,
        }
    }

    pub fn check(&self, key: K, now: Instant) -> Result<BucketState, RateLimitError> {
        let mut bucket = self
            .buckets
            .entry(key)
            .or_insert_with(|| A::new(self.capacity, self.refill_rate, now));

        bucket.set_last_seen(now);

        match bucket.allow(now) {
            AllowResult::Allowed => Ok(bucket.state(now)),
            AllowResult::Denied { retry_after } => Err(RateLimitError {
                retry_after,
                snapshot: bucket.state(now),
            }),
        }
    }

    pub fn cleanup(&self, ttl: Duration) {
        let now = Instant::now();

        self.buckets
            .retain(|_, bucket| now.duration_since(bucket.last_seen()) <= ttl);
    }
}
