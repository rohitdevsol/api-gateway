use std::{net::SocketAddr, time::Instant};

use crate::{AppState, http::errors::RateLimitHttpError};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use gateway_core::rate_limiter::token_bucket::BucketState;
use reqwest::StatusCode;

fn attach_headers(response: &mut Response<Body>, snapshot: &BucketState) {
    let headers = response.headers_mut();

    headers.insert(
        "X-RateLimit-Limit",
        snapshot.limit.to_string().parse().unwrap(),
    );

    headers.insert(
        "X-RateLimit-Remaining",
        snapshot.remaining.to_string().parse().unwrap(),
    );

    headers.insert(
        "X-RateLimit-Reset",
        snapshot.reset_after.as_secs().to_string().parse().unwrap(),
    );
}
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = addr.ip();

    match state.rate_limiters.check(ip, Instant::now()) {
        Ok(snapshot) => {
            let mut response = next.run(req).await;
            // attach headers
            tracing::info!(ip = %ip, remaining = snapshot.remaining, "rate limit check");
            attach_headers(&mut response, &snapshot);

            response
        }

        Err(err) => {
            let mut response = RateLimitHttpError {
                retry_after_ms: err.retry_after.as_millis() as u64,
            }
            .into_response();
            // attach headers
            tracing::info!(ip = %ip, remaining = err.snapshot.remaining, "rate limit check");
            attach_headers(&mut response, &err.snapshot);
            response
        }
    }
}
