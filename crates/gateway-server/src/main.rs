#![allow(dead_code)]
use axum::{Router, routing::get};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/health", get(health));

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
