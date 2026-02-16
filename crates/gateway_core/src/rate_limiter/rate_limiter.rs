use crate::rate_limiter::{AllowResult, TokenBucket, token_bucket::BucketState};
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
pub struct RateLimiter<K> {
    buckets: Arc<DashMap<K, TokenBucket>>,
    capacity: u128,
    refill_rate: u128,
}

impl<K> RateLimiter<K>
where
    K: std::cmp::Eq + std::hash::Hash,
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
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate, now));

        bucket.last_seen = now; //update the last seen here..so TokenBucket remains agnostic

        match bucket.allow(now) {
            AllowResult::Allowed => return Ok(bucket.state(now)),
            AllowResult::Denied { retry_after } => {
                return Err(RateLimitError {
                    retry_after,
                    snapshot: bucket.state(now),
                });
            }
        }
    }

    pub fn cleanup(&self, ttl: Duration) {
        println!("::[BUCKET_COUNT]:: Before Cleanup: {}", self.buckets.len());

        let now = Instant::now();
        self.buckets.retain(|_, bucket| {
            return now.duration_since(bucket.last_seen) <= ttl;
        });
        println!("::[BUCKET_COUNT]:: After Cleanup: {}", self.buckets.len());
    }
}
