use anyhow::Result;
use axum::Json;
use axum::extract::Path;
use axum::routing::put;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use prometheus::{self, Encoder, TextEncoder};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use tokio::net::TcpListener;

use crate::server::{ResponseMode, ResponseModes};

/// Shared application state used by the HTTP server handlers.
#[derive(Clone)]
pub struct AppState {
    pub tcp_ready: Arc<AtomicBool>,
    pub udp_ready: Arc<AtomicBool>,
    pub response_modes: Arc<ResponseModes>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    tcp: bool,
    udp: bool,
}

#[derive(Serialize)]
struct ModesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heartbeat: Option<String>,
}

impl ModesResponse {
    fn message(mode: ResponseMode) -> Self {
        ModesResponse {
            message: Some(mode.to_string()),
            heartbeat: None,
        }
    }

    fn heartbeat(mode: ResponseMode) -> Self {
        ModesResponse {
            message: None,
            heartbeat: Some(mode.to_string()),
        }
    }
}

/// Message type path parameter
#[derive(Debug, Clone, Copy)]
enum MessageType {
    Message,
    Heartbeat,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Message => write!(f, "message"),
            MessageType::Heartbeat => write!(f, "heartbeat"),
        }
    }
}

impl FromStr for MessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "message" => Ok(MessageType::Message),
            "heartbeat" => Ok(MessageType::Heartbeat),
            other => Err(format!("unknown message type '{other}'")),
        }
    }
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
    Json(HealthResponse { status: "ok" })
}

/// `GET /readyz` - Kubernetes readiness probe endpoint.
async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    let tcp = state.tcp_ready.load(Ordering::Relaxed);
    let udp = state.udp_ready.load(Ordering::Relaxed);
    if tcp && udp {
        let status = "ready";
        (StatusCode::OK, Json(ReadyResponse { status, tcp, udp }))
    } else {
        let status = "not ready";
        (StatusCode::SERVICE_UNAVAILABLE, Json(ReadyResponse { status, tcp, udp }))
    }
}

/// `GET /mode` - returns the current response modes for messages and heartbeats.
async fn get_modes(State(state): State<AppState>) -> impl IntoResponse {
    let resp = ModesResponse {
        message: Some(state.response_modes.message().to_string()),
        heartbeat: Some(state.response_modes.heartbeat().to_string()),
    };

    (StatusCode::OK, Json(resp))
}

/// `GET /mode/{msg_type}` - returns the current response mode for a specific message type.
async fn get_mode(
    State(state): State<AppState>,
    Path(msg_type): Path<String>,
) -> Result<Json<ModesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let msg_type = MessageType::from_str(&msg_type).map_err(|_| {
        let error = format!("unknown message type '{msg_type}', expected: message, heartbeat");
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error }))
    })?;

    match msg_type {
        MessageType::Message => Ok(Json(ModesResponse::message(state.response_modes.message()))),
        MessageType::Heartbeat => Ok(Json(ModesResponse::heartbeat(state.response_modes.heartbeat()))),
    }
}

/// `PUT /mode/{msg_type}/{mode}` - sets the response mode for messages or heartbeats.
async fn set_mode(
    State(state): State<AppState>,
    Path((msg_type, mode)): Path<(String, String)>,
) -> Result<Json<ModesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let msg_type = MessageType::from_str(&msg_type).map_err(|_| {
        let error = format!("unknown message type '{msg_type}', expected: message, heartbeat");
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error }))
    })?;

    let mode = ResponseMode::from_str(&mode).map_err(|_| {
        let error = format!("unknown mode '{mode}', expected: ack, nak, duh, none");
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error }))
    })?;

    match msg_type {
        MessageType::Message => {
            state.response_modes.set_message(mode);
            Ok(Json(ModesResponse::message(mode)))
        },
        MessageType::Heartbeat => {
            state.response_modes.set_heartbeat(mode);
            Ok(Json(ModesResponse::heartbeat(mode)))
        },
    }
}

/// Starts the auxiliary HTTP server that exposes observability and health
/// endpoints for Kubernetes and Prometheus.
pub async fn start_metrics_server(address: IpAddr, port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler))
        .route("/mode", get(get_modes))
        .route("/mode/{msg_type}", get(get_mode))
        .route("/mode/{msg_type}/{mode}", put(set_mode))
        .with_state(state);

    let listener = TcpListener::bind((address, port)).await?;
    log::info!("start listening on http://{}:{}/metrics", address, port);

    axum::serve(listener, app).await?;
    Ok(())
}
