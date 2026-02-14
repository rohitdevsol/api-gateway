use std::{net::SocketAddr, time::Instant};

use crate::AppState;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::Response,
};

use reqwest::StatusCode;

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = addr.ip();
    println!("{}", ip);

    match state.rate_limiters.check(ip, Instant::now()) {
        Ok(_) => next.run(req).await,
        Err(err) => {
            let retry_after_secs = err.retry_after.as_secs();
            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Retry-After", retry_after_secs.to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"error":"rate_limited","retry_after":{}}}"#,
                    retry_after_secs
                )))
                .unwrap()
        }
    }
}
