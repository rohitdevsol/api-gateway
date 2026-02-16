use std::{net::SocketAddr, sync::atomic::Ordering, time::Instant};

use crate::{AppState, http::errors::RateLimitHttpError, metrics};
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

fn inc_global_limit(state: &AppState) {
    state
        .metrics
        .total_rate_limited
        .fetch_add(1, Ordering::Relaxed);
    state
        .metrics
        .global_rate_limited
        .fetch_add(1, Ordering::Relaxed);
}
fn inc_ip_limit(state: &AppState) {
    state
        .metrics
        .total_rate_limited
        .fetch_add(1, Ordering::Relaxed);
    state
        .metrics
        .ip_rate_limited
        .fetch_add(1, Ordering::Relaxed);
}
fn inc_route_limit(state: &AppState) {
    state
        .metrics
        .total_rate_limited
        .fetch_add(1, Ordering::Relaxed);
    state
        .metrics
        .route_rate_limited
        .fetch_add(1, Ordering::Relaxed);
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // Skip internal routes
    if path.starts_with("/metrics") || path.starts_with("/health") {
        return next.run(req).await;
    }
    //handle metrices  (total requests)
    state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

    let ip = addr.ip();
    let route = path
        .trim_start_matches("/api/")
        .split('/')
        .next()
        .unwrap_or("root")
        .to_string();

    let now = Instant::now();

    let span = tracing::info_span!(
        "request",
        ip = %ip,
        route = %route
    );
    let _enter = span.enter();

    let global_snapshot = match state.global_limiter.check((), now) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            inc_global_limit(&state);
            tracing::warn!(limiter = "global", decision = "denied");
            return rate_limit_response(err);
        }
    };

    let route_snapshot = match state.route_limiter.check(route, now) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            inc_route_limit(&state);
            tracing::warn!(limiter = "route", decision = "denied");
            return rate_limit_response(err);
        }
    };

    let ip_snapshot = match state.ip_limiter.check(ip, now) {
        Ok(snapshot) => {
            tracing::info!(decision = "allowed");
            snapshot
        }
        Err(err) => {
            inc_ip_limit(&state);
            tracing::warn!(limiter = "ip", decision = "denied");
            return rate_limit_response(err);
        }
    };

    state.metrics.total_allowed.fetch_add(1, Ordering::Relaxed);

    let mut response = next.run(req).await;

    // attach the header of the snapshot that have least remaining
    let effective_snapshot = {
        let mut snapshots = [&global_snapshot, &route_snapshot, &ip_snapshot];
        snapshots.sort_by_key(|s| s.remaining);
        snapshots[0]
    };

    //add effective snapshot headers
    attach_headers(&mut response, effective_snapshot);

    tracing::info!(
        limiter = "all",
        decision = "allowed",
        remaining = effective_snapshot.remaining
    );
    return response;
}
