use axum::{body::Body, http::Response, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct RateLimitBody {
    error: &'static str,
    retry_after_ms: u64,
    limit: u64,
    remaining: u64,
    reset: u64,
}

pub struct RateLimitHttpError {
    pub retry_after_ms: u64,
    pub ratelimit_limit: u64,
    pub ratelimit_remaining: u64,
    pub ratelimit_reset: u64,
}

impl IntoResponse for RateLimitHttpError {
    fn into_response(self) -> axum::response::Response {
        //convert the millis to seconds using ceiling
        let seconds = (self.retry_after_ms + 999) / 1000;

        let body = RateLimitBody {
            error: "rate_limited",
            retry_after_ms: self.retry_after_ms,
            limit: self.ratelimit_limit,
            remaining: self.ratelimit_remaining,
            reset: self.ratelimit_reset,
        };

        let json = serde_json::to_string(&body).unwrap();

        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", seconds.to_string())
            .header("Content-Type", "application/json")
            .body(Body::from(json))
            .unwrap()
    }
}
