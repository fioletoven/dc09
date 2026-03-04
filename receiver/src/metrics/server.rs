use anyhow::Result;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use prometheus::{self, Encoder, TextEncoder};
use std::net::IpAddr;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub tcp_ready: Arc<AtomicBool>,
    pub udp_ready: Arc<AtomicBool>,
}

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

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.tcp_ready.load(Ordering::Relaxed) && state.udp_ready.load(Ordering::Relaxed) {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

pub async fn start_metrics_server(address: IpAddr, port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler))
        .with_state(state);

    let listener = TcpListener::bind((address, port)).await?;
    log::info!("start listening on http://{}:{}/metrics", address, port);

    axum::serve(listener, app).await?;
    Ok(())
}
