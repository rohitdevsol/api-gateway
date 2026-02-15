use axum::{body::Body, http::Response, http::StatusCode, response::IntoResponse};

pub struct RateLimitHttpError {
    pub retry_after_ms: u64,
}

impl IntoResponse for RateLimitHttpError {
    fn into_response(self) -> axum::response::Response {
        //convert the millis to seconds using ceiling
        let seconds = (self.retry_after_ms + 999) / 1000;
        let body = format!(
            r#"{{"error":"rate_limited","retry_after_ms":{}}}"#,
            self.retry_after_ms
        );
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", seconds.to_string())
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }
}
