use std::{ cmp::min, time::{ Duration, Instant } };

struct TokenBucket {
    max_capacity: u128,
    current_tokens: u128,
    refill_rate: u128, //tokens per second
    last_refill_time: Instant,
}

impl TokenBucket {
    fn new(max_capacity: u128, refill_rate: u128, now: Instant) -> Self {
        Self {
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
            self.current_tokens = min(self.current_tokens + tokens, self.max_capacity);
            //update the refill time

            //these tokens were made in how many seconds .. so we do not account extra time
            // for last_refill time
            let secs = (tokens as f64) / (self.refill_rate as f64);
            let advance = Duration::from_secs_f64(secs);

            self.last_refill_time += advance;
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
    fn it_works() {
        // let result = add(2, 2);
        // assert_eq!(result, 4);
    }
}
