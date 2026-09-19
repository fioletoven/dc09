use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router, body::Body, response::IntoResponse};
use prometheus::{self, Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use tokio::net::TcpListener;

use crate::metrics::recording::{RecorderHandle, RecorderStatus, RecordingStatus};
use crate::server::{ResponseMode, ResponseModes};

/// Shared application state used by the HTTP server handlers.
#[derive(Clone)]
pub struct AppState {
    pub tcp_ready: Arc<AtomicBool>,
    pub udp_ready: Arc<AtomicBool>,
    pub response_modes: Arc<ResponseModes>,
    pub recorder: RecorderHandle,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl ErrorResponse {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl From<&str> for ErrorResponse {
    fn from(value: &str) -> Self {
        ErrorResponse { error: value.to_owned() }
    }
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

/// Message type for `GET /mode/{msg_type}`.
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

#[derive(Serialize)]
struct RecordChangeResponse {
    status: RecordingStatus,
}

/// Query parameters for `GET /record`.
#[derive(Debug, Deserialize)]
struct RecordQuery {
    messages: Option<bool>,
    heartbeats: Option<bool>,
}

impl RecordQuery {
    /// Resolve the effective flags, applying defaults where params were omitted.
    fn resolve(&self) -> Result<(bool, bool), &'static str> {
        let (messages, heartbeats) = match (self.messages, self.heartbeats) {
            (None, None) => (true, true),
            (Some(m), None) => (m, !m),
            (None, Some(h)) => (!h, h),
            (Some(m), Some(h)) => (m, h),
        };

        if !messages && !heartbeats {
            return Err("at least one of 'messages' or 'heartbeats' must be true");
        }

        Ok((messages, heartbeats))
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

/// `GET /record` - retrieve recorded entries.
async fn record_get(State(state): State<AppState>, Query(query): Query<RecordQuery>, headers: HeaderMap) -> Response<Body> {
    let (messages, heartbeats) = match query.resolve() {
        Ok(flags) => flags,
        Err(reason) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("content-type", "application/json")
                .body(Body::from(ErrorResponse::from(reason).to_json()))
                .unwrap();
        },
    };

    let want_csv = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/csv"))
        .unwrap_or(false);

    match state.recorder.query(messages, heartbeats).await {
        None => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("content-type", "application/json")
            .body(Body::from(ErrorResponse::from("recorder unavailable").to_json()))
            .unwrap(),
        Some(snapshot) if want_csv => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/csv; charset=utf-8")
            .header("content-disposition", "attachment; filename=\"recording.csv\"")
            .body(Body::from(snapshot.to_csv()))
            .unwrap(),
        Some(snapshot) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(snapshot.to_json()))
            .unwrap(),
    }
}

/// `GET /record/status` - lightweight status check without returning entries.
async fn record_status(State(state): State<AppState>) -> Result<Json<RecorderStatus>, (StatusCode, Json<ErrorResponse>)> {
    match state.recorder.status().await {
        None => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "recorder unavailable".to_owned(),
            }),
        )),
        Some(snapshot) => Ok(Json(snapshot)),
    }
}

/// `POST /record/start` - begin recording DC09 messages.
async fn record_start(State(state): State<AppState>) -> Json<RecordChangeResponse> {
    state.recorder.start();
    Json(RecordChangeResponse {
        status: RecordingStatus::Recording,
    })
}

/// `POST /record/stop` - stop recording (entries are preserved).
async fn record_stop(State(state): State<AppState>) -> Json<RecordChangeResponse> {
    state.recorder.stop();
    Json(RecordChangeResponse {
        status: RecordingStatus::Idle,
    })
}

/// `POST /record/restart` - clear all entries and start fresh.
async fn record_restart(State(state): State<AppState>) -> Json<RecordChangeResponse> {
    state.recorder.restart();
    Json(RecordChangeResponse {
        status: RecordingStatus::Recording,
    })
}

/// Starts the auxiliary HTTP server that exposes observability, health,
/// recording control, and response mode endpoints.
pub async fn start_metrics_server(address: IpAddr, port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler))
        .route("/mode", get(get_modes))
        .route("/mode/{msg_type}", get(get_mode))
        .route("/mode/{msg_type}/{mode}", put(set_mode))
        .route("/record", get(record_get))
        .route("/record/status", get(record_status))
        .route("/record/start", put(record_start))
        .route("/record/stop", put(record_stop))
        .route("/record/restart", put(record_restart))
        .with_state(state);

    let listener = TcpListener::bind((address, port)).await?;
    log::info!("start listening on http://{address}:{port}");

    axum::serve(listener, app).await?;
    Ok(())
}
