use std::{net::SocketAddr, time::Instant};

use crate::{AppState, http::errors::RateLimitHttpError};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

use gateway_core::rate_limiter::{rate_limiter::RateLimitError, token_bucket::BucketState};
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

fn rate_limit_response(err: RateLimitError) -> Response {
    let mut response = RateLimitHttpError {
        retry_after_ms: err.retry_after.as_millis() as u64,
    }
    .into_response();
    attach_headers(&mut response, &err.snapshot);
    return response;
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = addr.ip();
    let route = req
        .uri()
        .path()
        .to_string()
        .split('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("root")
        .to_string();
    let now = Instant::now();

    let global_snapshot = match state.global_limiter.check((), now) {
        Ok(snapshot) => snapshot,
        Err(err) => return rate_limit_response(err),
    };

    let route_snapshot = match state.route_limiter.check(route.clone(), now) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            tracing::warn!(ip = %ip, route = %route, "rate limit exceeded");
            return rate_limit_response(err);
        }
    };

    let ip_snapshot = match state.ip_limiter.check(ip, now) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            tracing::warn!(ip = %ip, route = %route, "rate limit exceeded");
            return rate_limit_response(err);
        }
    };

    let mut response = next.run(req).await;
    // attach the header of the snapshot that have least remaining

    let effective_snapshot = {
        let mut snapshots = [&global_snapshot, &route_snapshot, &ip_snapshot];
        snapshots.sort_by_key(|s| s.remaining);
        snapshots[0]
    };
    attach_headers(&mut response, effective_snapshot);

    return response;
}
