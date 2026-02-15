use crate::rate_limiter::{AllowResult, TokenBucket, token_bucket::BucketState};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct RateLimitError {
    pub retry_after: Duration,
    pub snapshot: BucketState,
}

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    capacity: u128,
    refill_rate: u128,
}

impl RateLimiter {
    pub fn new(capacity: u128, refill_rate: u128) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            refill_rate,
        }
    }

    pub fn check(&self, ip_addr: IpAddr, now: Instant) -> Result<BucketState, RateLimitError> {
        let mut buckets = self.buckets.lock().unwrap();

        let bucket = buckets
            .entry(ip_addr)
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate, now));

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
}
