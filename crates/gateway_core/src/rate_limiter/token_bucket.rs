use std::{
    cmp::min,
    time::{Duration, Instant},
};

use crate::rate_limiter::algorithm::{AllowResult, BucketState, RateLimitAlgorithm};

#[derive(Clone)]
pub struct TokenBucket {
    max_capacity: u128,
    current_tokens: u128,
    refill_rate: u128,
    last_refill_time: Instant,
    pub last_seen: Instant,
}

impl RateLimitAlgorithm for TokenBucket {
    fn allow(&mut self, now: Instant) -> AllowResult {
        TokenBucket::allow(self, now)
    }
    fn state(&self, now: Instant) -> BucketState {
        TokenBucket::state(self, now)
    }
    fn last_seen(&self) -> Instant {
        self.last_seen
    }
    fn set_last_seen(&mut self, now: Instant) {
        self.last_seen = now;
    }
}
impl TokenBucket {
    pub fn new(max_capacity: u128, refill_rate: u128, now: Instant) -> Self {
        Self {
            max_capacity: max_capacity,
            current_tokens: max_capacity,
            refill_rate: refill_rate,
            last_refill_time: now,
            last_seen: now,
        }
    }

    pub fn state(&self, now: Instant) -> BucketState {
        let token_interval = Duration::from_secs_f64(1.0 / self.refill_rate as f64);
        let elapsed = now - self.last_refill_time;

        let mut reset_after = if elapsed >= token_interval {
            Duration::ZERO
        } else {
            token_interval
                .checked_sub(elapsed)
                .unwrap_or(Duration::ZERO)
        };

        if self.current_tokens == self.max_capacity {
            reset_after = Duration::ZERO;
        }

        BucketState {
            limit: self.max_capacity,
            remaining: self.current_tokens,
            reset_after,
        }
    }

    pub fn allow(&mut self, current_ts: Instant) -> AllowResult {
        let elapsed = current_ts.duration_since(self.last_refill_time);
        let tokens_float = elapsed.as_secs_f64() * (self.refill_rate as f64);

        let tokens = tokens_float.floor() as u128;

        if tokens > 0 {
            let available_space = self.max_capacity - self.current_tokens;
            let tokens_added = min(available_space, tokens);
            if tokens_added > 0 {
                self.current_tokens += tokens_added;
                if tokens_added == available_space {
                    // Bucket became full so discard extra time
                    self.last_refill_time = current_ts;
                } else {
                    // Partial refill so advance proportionaly
                    let secs = (tokens_added as f64) / (self.refill_rate as f64);
                    self.last_refill_time += Duration::from_secs_f64(secs);
                }
            }
        }
        //check if the tokens are present
        if self.current_tokens > 0 {
            self.current_tokens = self.current_tokens - 1;
            return AllowResult::Allowed;
        }

        let token_interval = Duration::from_secs_f64(1.0 / self.refill_rate as f64); // 1. time taken to generate 1 token

        // 2. difference of current time received and last refill time
        let elapsed_time_since_last = current_ts - self.last_refill_time;

        // retry after (1-2)
        // let retry_after = token_interval - elapsed_time_since_last;
        let retry_after = token_interval
            .checked_sub(elapsed_time_since_last)
            .unwrap_or(Duration::ZERO);

        return AllowResult::Denied { retry_after };
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn burst_test_pass() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        for _ in 0..5 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }
        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }));
    }

    #[test]
    pub fn burst_test_fail() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);
        for _ in 0..5 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }
        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }))
    }

    #[test]
    pub fn refill_after_correct_time() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);
        //empty the bucket
        for _ in 1..6 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }
        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }));
        //bucket empty -> check after the 200ms
        assert!(matches!(
            bucket.allow(t0 + Duration::from_millis(200)),
            AllowResult::Allowed
        ));
    }

    #[test]
    pub fn no_refill_before_correct_time() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        //empty the bucket
        for _ in 0..5 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }
        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }));
        //bucket empty -> check after the 100ms
        assert!(matches!(
            bucket.allow(t0 + Duration::from_millis(100)),
            AllowResult::Denied { .. }
        ));
    }

    #[test]
    pub fn refill_proportionally() {
        let mut t0 = Instant::now();
        let mut bucket = TokenBucket::new(10, 5, t0);
        //empty the bucket
        for _ in 0..10 {
            let _ = bucket.allow(t0);
        }
        //bucket empty -> check after 1400ms or 1.4s -> i.e allow 7 times
        t0 = t0 + Duration::from_secs_f64(1.4);
        for _ in 0..7 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }
        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }));
    }

    #[test]
    pub fn do_not_exceed_capacity() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        // empty the bucket first
        for _ in 0..5 {
            assert!(matches!(bucket.allow(t0), AllowResult::Allowed));
        }

        assert!(matches!(bucket.allow(t0), AllowResult::Denied { .. }));

        //jump forward 100 seconds
        let t1 = t0 + Duration::from_secs(100);
        // 5 rapid requests
        for _ in 0..5 {
            assert!(matches!(bucket.allow(t1), AllowResult::Allowed));
        }

        assert!(matches!(bucket.allow(t1), AllowResult::Denied { .. }));
    }
}
