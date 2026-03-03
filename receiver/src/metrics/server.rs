use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use prometheus::{self, Encoder, TextEncoder};
use std::net::IpAddr;
use tokio::net::TcpListener;

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

async fn ready_handler() -> impl IntoResponse {
    (StatusCode::OK, "ready")
}

pub async fn start_metrics_server(address: IpAddr, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler));

    let listener = TcpListener::bind((address, port)).await?;
    log::info!("start listening on http://{}:{}/metrics", address, port);

    axum::serve(listener, app).await?;
    Ok(())
}
