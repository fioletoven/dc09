use anyhow::Result;
use axum::extract::Path;
use axum::routing::put;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use prometheus::{self, Encoder, TextEncoder};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use tokio::net::TcpListener;

use crate::server::ResponseMode;

/// Shared application state used by the HTTP server handlers.
#[derive(Clone)]
pub struct AppState {
    pub tcp_ready: Arc<AtomicBool>,
    pub udp_ready: Arc<AtomicBool>,
    pub response_mode: Arc<AtomicU8>,
}

/// `GET /metrics` - Prometheus metrics endpoint.
async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            buffer,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain; charset=utf-8")],
            format!("Failed to encode metrics: {e}").into_bytes(),
        ),
    }
}

/// `GET /healthz` - Kubernetes liveness probe endpoint.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// `GET /readyz` - Kubernetes readiness probe endpoint.
async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.tcp_ready.load(Ordering::Relaxed) && state.udp_ready.load(Ordering::Relaxed) {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

/// `GET /mode` - returns the current response mode.
async fn get_mode(State(state): State<AppState>) -> impl IntoResponse {
    let mode: ResponseMode = state.response_mode.load(Ordering::Relaxed).into();
    (StatusCode::OK, mode.to_string())
}

/// `PUT /mode/:mode` - sets the response mode.
async fn set_mode(State(state): State<AppState>, Path(mode): Path<String>) -> impl IntoResponse {
    match ResponseMode::from_str(&mode) {
        Ok(mode) => {
            state.response_mode.store(mode.into(), Ordering::Relaxed);
            (StatusCode::OK, format!("mode set to {mode}"))
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            format!("unknown mode '{mode}', expected one of: ack, nak, duh, none"),
        ),
    }
}

/// Starts the auxiliary HTTP server that exposes observability and health
/// endpoints for Kubernetes and Prometheus.
pub async fn start_metrics_server(address: IpAddr, port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler))
        .route("/mode", get(get_mode))
        .route("/mode/{mode}", put(set_mode))
        .with_state(state);

    let listener = TcpListener::bind((address, port)).await?;
    log::info!("start listening on http://{}:{}/metrics", address, port);

    axum::serve(listener, app).await?;
    Ok(())
}
