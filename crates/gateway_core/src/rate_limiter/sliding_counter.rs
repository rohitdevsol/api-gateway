use crate::rate_limiter::algorithm::RateLimitAlgorithm;

struct SlidingCounter {
    capacity: u128,
    window: Duration,

    current_window_start: Instant,
    current_count: u128,
    previous_count: u128,

    last_seen: Instant,
}

impl RateLimitAlgorithm for SlidingCounter {
    fn allow(&mut self, now: Instant) -> AllowResult {
        SlidingCounter::allow(self, now)
    }
    fn state(&self, now: Instant) -> BucketState {
        SlidingCounter::state(self, now)
    }
    fn last_seen(&self) -> Instant {
        self.last_seen
    }
    fn set_last_seen(&mut self, now: Instant) {
        self.last_seen = now;
    }
}

impl SlidingCounter {
    fn new(capacity: u128, refill_rate: u128, now: Instant) -> Self {
        Self {
            capacity,
            window: Duration::from_secs(refill_rate as u64),

            current_window_start: now,
            current_count: 0,
            previous_count: 0,

            last_seen: now,
        }
    }
    fn roll_window(&mut self, now: Instant) {
        if now.duration_since(self.current_window_start) >= self.window {
            // move current into previous
            self.previous_count = self.current_count;
            self.current_count = 0;

            self.current_window_start = now;
        }
    }

    fn allow(&mut self, now: Instant) -> AllowResult {
        self.last_seen = now;

        self.roll_window(now);

        let elapsed = now.duration_since(self.current_window_start);
        let weight = elapsed.as_secs_f64() / self.window.as_secs_f64();

        let effective = (self.previous_count as f64 * (1.0 - weight)) + self.current_count as f64;

        if effective < self.capacity as f64 {
            self.current_count += 1;
            AllowResult::Allowed
        } else {
            let retry_after = self.window - elapsed;
            AllowResult::Denied { retry_after }
        }
    }

    fn state(&self, now: Instant) -> BucketState {
        let elapsed = now.duration_since(self.current_window_start);

        let weight = elapsed.as_secs_f64() / self.window.as_secs_f64();

        let effective = (self.previous_count as f64 * (1.0 - weight)) + self.current_count as f64;

        let remaining = if effective >= self.capacity as f64 {
            0
        } else {
            (self.capacity as f64 - effective).floor() as u128
        };

        let reset_after = if elapsed >= self.window {
            Duration::ZERO
        } else {
            self.window - elapsed
        };

        BucketState {
            limit: self.capacity,
            remaining,
            reset_after,
        }
    }
}
