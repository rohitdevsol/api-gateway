#![allow(dead_code, unused_variables, unused)]

pub mod config;
pub mod http;
pub mod metrics;
pub mod middleware;

use crate::{
    config::gateway_config::GatewayConfig, metrics::gateway_metrics::GatewayMetrices,
    middleware::rate_limit::rate_limit_middleware,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, Response, StatusCode},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{any, get},
};
use gateway_core::rate_limiter::{RateLimiter, TokenBucket, rate_limiter::AlgorithmType};

use reqwest::Client;
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use tracing::info;

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
const BUCKET_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct AppState {
    client: Client,
    config: GatewayConfig,
    global_limiter: RateLimiter<()>,
    ip_limiter: RateLimiter<IpAddr>,
    route_limiter: RateLimiter<String>,
    metrics: Arc<GatewayMetrices>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = GatewayConfig::from_env().expect("Invalid Gateway Config");
    let metrics = Arc::new(GatewayMetrices::new());
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let algorithm = match config.algorithm.as_str() {
        "sliding_log" => AlgorithmType::SlidingLog,
        _ => AlgorithmType::TokenBucket,
    };

    let state = AppState {
        client,
        config: config.clone(),
        global_limiter: RateLimiter::new(
            config.global_capacity,
            config.global_refill_rate,
            algorithm.clone(),
        ),
        route_limiter: RateLimiter::new(
            config.route_capacity,
            config.route_refill_rate,
            algorithm.clone(),
        ),
        ip_limiter: RateLimiter::new(config.ip_capacity, config.ip_refill_rate, algorithm),
        metrics: metrics.clone(),
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

    {
        let global = state.global_limiter.clone();
        let route = state.route_limiter.clone();
        let ip = state.ip_limiter.clone();
        let mut shutdown_rx = shutdown_rx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(CLEANUP_INTERVAL) => {
                        global.cleanup(BUCKET_TTL);
                        route.cleanup(BUCKET_TTL);
                        ip.cleanup(BUCKET_TTL);
                        tracing::debug!("bucket cleanup executed");
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("cleanup task shutting down");
                        break;
                    }
                }
            }
        });
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting API Gateway server on http://{}", addr);

    let internal = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler));

    let api = Router::new()
        .route("/{*path}", any(special_handler))
        .layer(from_fn_with_state(state.clone(), rate_limit_middleware));

    let app = Router::new()
        .nest("/api", api)
        .merge(internal)
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind the address");

    let shutdown_signal = {
        async move {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");

            tracing::info!("shutdown signal received");

            let _ = shutdown_tx.send(());
        }
    };

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await
    .unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn special_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let body = to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("");

    let full_url = format!(
        "{}/{}",
        state.config.upstream_base_url.trim_end_matches('/'),
        path_and_query.trim_start_matches('/')
    );
    tracing::debug!(%full_url);
    let upstream = state
        .client
        .request(method, full_url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let status = upstream.status();
    let body = upstream
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap())
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let m = &state.metrics;

    let body = format!(
        r#"
        gateway_total_requests {}
        gateway_total_allowed {}
        gateway_total_rate_limited {}
        gateway_global_rate_limited {}
        gateway_route_rate_limited {}
        gateway_ip_rate_limited {}
    "#,
        m.total_requests.load(Ordering::Relaxed),
        m.total_allowed.load(Ordering::Relaxed),
        m.total_rate_limited.load(Ordering::Relaxed),
        m.global_rate_limited.load(Ordering::Relaxed),
        m.route_rate_limited.load(Ordering::Relaxed),
        m.ip_rate_limited.load(Ordering::Relaxed),
    );

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        body,
    )
}
