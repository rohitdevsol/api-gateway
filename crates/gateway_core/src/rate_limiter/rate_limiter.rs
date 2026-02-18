use crate::rate_limiter::{
    TokenBucket,
    algorithm::{self, AllowResult, BucketState, RateLimitAlgorithm},
    sliding_log::SlidingLog,
};
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
pub enum AlgorithmType {
    TokenBucket,
    SlidingLog,
    SlidingCounter,
}

#[derive(Clone)]
pub struct RateLimiter<K>
where
    K: Eq + std::hash::Hash,
{
    buckets: Arc<DashMap<K, Box<dyn RateLimitAlgorithm>>>,
    capacity: u128,
    refill_rate: u128,
    algorithm: AlgorithmType,
}

impl<K> RateLimiter<K>
where
    K: Eq + std::hash::Hash,
{
    pub fn new(capacity: u128, refill_rate: u128, algorithm: AlgorithmType) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            capacity,
            refill_rate,
            algorithm,
        }
    }

    pub fn check(&self, key: K, now: Instant) -> Result<BucketState, RateLimitError> {
        let mut bucket = self
            .buckets
            .entry(key)
            .or_insert_with(|| match self.algorithm {
                AlgorithmType::TokenBucket => {
                    Box::new(TokenBucket::new(self.capacity, self.refill_rate, now))
                        as Box<dyn RateLimitAlgorithm>
                }
                AlgorithmType::SlidingLog => {
                    Box::new(SlidingLog::new(self.capacity, self.refill_rate, now))
                        as Box<dyn RateLimitAlgorithm>
                }
                AlgorithmType::SlidingCounter => {
                    Box::new(SlidingCounter::new(self.capacity, self.refill_rate, now))
                        as Box<dyn RateLimitAlgorithm + Send + Sync>
                }
            });

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
