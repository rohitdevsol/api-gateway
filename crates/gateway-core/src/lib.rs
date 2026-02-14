use std::{
    cmp::min,
    time::{Duration, Instant},
};

struct TokenBucket {
    max_capacity: u128,
    current_tokens: u128,
    refill_rate: u128, //tokens per second
    last_refill_time: Instant,
}

impl TokenBucket {
    fn new(max_capacity: u128, refill_rate: u128, now: Instant) -> Self {
        TokenBucket {
            max_capacity: max_capacity,
            current_tokens: max_capacity,
            refill_rate: refill_rate,
            last_refill_time: now,
        }
    }

    fn allow(&mut self, current_ts: Instant) -> bool {
        let elapsed = current_ts.duration_since(self.last_refill_time);
        let tokens_float = elapsed.as_secs_f64() * (self.refill_rate as f64);

        // floored (how many to add actually)
        let tokens = tokens_float.floor() as u128;

        if tokens > 0 {
            let available_space = self.max_capacity - self.current_tokens;
            let tokens_added = min(tokens, available_space);

            if tokens_added > 0 {
                self.current_tokens += tokens_added;
                // for last_refill time
                let secs = (tokens_added as f64) / (self.refill_rate as f64);
                let advance = Duration::from_secs_f64(secs);
                self.last_refill_time += advance;
            }
        }

        //check if the tokens are present
        if self.current_tokens > 0 {
            self.current_tokens = self.current_tokens - 1;
            return true;
        }

        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_test_pass() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        for _ in 0..5 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
    }

    #[test]
    fn burst_test_fail() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);
        for _ in 0..5 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
    }

    #[test]
    fn refill_after_correct_time() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);
        //empty the bucket
        for _ in 1..6 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
        //bucket empty -> check after the 200ms
        assert!(bucket.allow(t0 + Duration::from_millis(200)));
    }

    #[test]
    fn no_refill_before_correct_time() {
        let t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        //empty the bucket
        for _ in 1..6 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
        //bucket empty -> check after the 100ms
        assert!(!bucket.allow(t0 + Duration::from_millis(100)));
    }

    #[test]
    fn refill_proportionally() {
        let mut t0 = Instant::now();
        let mut bucket = TokenBucket::new(10, 5, t0);
        //empty the bucket
        for _ in 0..10 {
            let _ = bucket.allow(t0);
        }
        //bucket empty -> check after 1400ms or 1.4s -> i.e allow 7 times
        t0 = t0 + Duration::from_secs_f64(1.4);
        for _ in 0..7 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
    }

    #[test]
    fn do_not_exceed_capacity() {
        let mut t0 = Instant::now();
        let mut bucket = TokenBucket::new(5, 5, t0);

        // empty the bucket first
        for _ in 0..5 {
            assert!(bucket.allow(t0));
        }

        t0 = t0 + Duration::from_secs(100);
        for _ in 0..5 {
            assert!(bucket.allow(t0));
        }
        assert!(!bucket.allow(t0));
    }
}
