pub mod algorithm;
pub mod rate_limiter;
pub mod sliding_log;
pub mod token_bucket;
pub use rate_limiter::RateLimiter;
pub use token_bucket::TokenBucket;

#[cfg(feature = "token_bucket")]
pub type DefaultAlgorithm = TokenBucket;

#[cfg(feature = "sliding_log")]
pub type DefaultAlgorithm = SlidingLog;

#[cfg(feature = "sliding_counter")]
pub type DefaultAlgorithm = SlidingCounter;
