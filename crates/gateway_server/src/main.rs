#![allow(dead_code, unused_variables)]

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, Response},
    routing::{any, get},
};
use gateway_core::rate_limiter::RateLimiter;
use reqwest::{self, Client, StatusCode};
use std::{net::SocketAddr, usize};
use tracing::info;

#[derive(Clone)]
struct AppState {
    client: Client,
    rate_limiters: RateLimiter,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        client: Client::new(),
        rate_limiters: RateLimiter::new(5, 5),
    };
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/health", get(health))
        .route("/{*path}", any(special_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting API Gateway server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind the address");
    axum::serve(listener, app).await.expect("Server failed");
}

async fn health() -> &'static str {
    "OK"
}

async fn special_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    //store the method
    let method = req.method().clone();
    //store the uri
    let uri = req.uri().clone();
    // store the headers
    let headers = req.headers().clone();

    //get the body
    let body = to_bytes(req.into_body(), usize::MAX)
        .await
        .expect("Failed to parse body into bytes");

    let upstream_url = String::from("https://httpbin.org");
    let url = uri.to_string();

    let full_url = upstream_url + &url;

    let upstream = state
        .client
        .request(method, full_url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .expect("Request failed for upstream url");

    //convert reqwest response to axum response

    let status = upstream.status();
    // let headers = upstream.headers().clone();
    let body = upstream.bytes().await.expect("Can't do this");

    let response = Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap();

    Ok(response)
}
