use crate::rate_limiter::{AllowResult, TokenBucket};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct RateLimitError {
    pub retry_after: Duration,
}

pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    capacity: u128,
    refill_rate: u128,
}

impl RateLimiter {
    fn check(&self, ip_addr: IpAddr, now: Instant) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.lock().unwrap();

        let bucket = buckets
            .entry(ip_addr)
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate, now));

        match bucket.allow(now) {
            AllowResult::Allowed => {}
            AllowResult::Denied { retry_after } => {
                return Err(RateLimitError {
                    retry_after: retry_after,
                });
            }
        }

        Ok(())
    }
}
