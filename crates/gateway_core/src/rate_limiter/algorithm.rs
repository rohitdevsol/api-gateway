use std::time::{Duration, Instant};

pub enum AllowResult {
    Allowed,
    Denied { retry_after: Duration },
}

//adding a snapshot
pub struct BucketState {
    pub limit: u128,
    pub remaining: u128,
    pub reset_after: Duration,
}

pub trait RateLimitAlgorithm: Send + Sync {
    fn allow(&mut self, now: Instant) -> AllowResult;
    fn state(&self, now: Instant) -> BucketState;
    fn last_seen(&self) -> Instant;
    fn set_last_seen(&mut self, now: Instant);
}
