use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::rate_limiter::algorithm::{AllowResult, BucketState, RateLimitAlgorithm};

pub struct SlidingLog {
    capacity: u128,
    window: Duration,
    entries: VecDeque<Instant>,
    pub last_seen: Instant,
}

impl RateLimitAlgorithm for SlidingLog {
    fn new(capacity: u128, window_seconds: u128, now: Instant) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
            last_seen: now,
            window: Duration::from_secs(window_seconds as u64),
        }
    }

    fn last_seen(&self) -> Instant {
        self.last_seen
    }

    fn set_last_seen(&mut self, now: Instant) {
        self.last_seen = now;
    }
    fn allow(&mut self, now: Instant) -> AllowResult {
        self.allow(now)
    }

    fn state(&self, now: Instant) -> BucketState {
        self.state(now)
    }
}
impl SlidingLog {
    pub fn new(capacity: u128, window_seconds: u128, now: Instant) -> Self {
        Self {
            capacity,
            window: Duration::from_secs(window_seconds as u64),
            entries: VecDeque::new(),
            last_seen: now,
        }
    }
    pub fn allow(&mut self, now: Instant) -> AllowResult {
        self.last_seen = now;

        //trim old entries
        while let Some(front) = self.entries.front() {
            if now.duration_since(*front) >= self.window {
                self.entries.pop_front();
            } else {
                break;
            }
        }

        // check capacity
        if self.entries.len() < self.capacity as usize {
            self.entries.push_back(now);
            AllowResult::Allowed
        } else {
            //compute retry_after
            let oldest = *self.entries.front().unwrap();
            let retry_after = self
                .window
                .checked_sub(now.duration_since(oldest))
                .unwrap_or(Duration::ZERO);
            AllowResult::Denied { retry_after }
        }
    }

    pub fn state(&self, now: Instant) -> BucketState {
        let remaining = self.capacity - self.entries.len() as u128;

        let reset_after = if let Some(oldest) = self.entries.front() {
            let elapsed = now.duration_since(*oldest);

            if elapsed >= self.window {
                Duration::ZERO
            } else {
                self.window - elapsed
            }
        } else {
            Duration::ZERO
        };

        BucketState {
            limit: self.capacity,
            remaining,
            reset_after,
        }
    }
}
