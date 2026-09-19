use serde::Serialize;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Udp,
    Tcp,
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transport::Udp => write!(f, "udp"),
            Transport::Tcp => write!(f, "tcp"),
        }
    }
}

/// A single captured DC09 exchange.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedEntry {
    pub timestamp_ms: u128,
    pub transport: Transport,
    pub peer: String,
    pub message: String,
    pub response: Option<String>,
    pub valid: bool,
}

impl RecordedEntry {
    pub fn new(
        transport: Transport,
        peer: SocketAddr,
        received: impl Into<String>,
        response: Option<String>,
        valid: bool,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis();

        Self {
            timestamp_ms,
            transport,
            peer: peer.to_string(),
            message: received.into(),
            response,
            valid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingStatus {
    Idle,
    Recording,
}

impl From<bool> for RecordingStatus {
    fn from(value: bool) -> Self {
        if value {
            RecordingStatus::Recording
        } else {
            RecordingStatus::Idle
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecorderSnapshot {
    pub status: RecordingStatus,
    pub entries: Vec<RecordedEntry>,
}

impl RecorderSnapshot {
    pub fn to_csv(&self) -> String {
        snapshot_to_csv(self)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecorderStatus {
    pub status: RecordingStatus,
    pub count: usize,
}

#[derive(Clone)]
pub struct RecorderHandle {
    is_recording: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<RecordingEvent>,
}

impl RecorderHandle {
    pub fn new() -> Self {
        let is_recording = Arc::new(AtomicBool::new(false));
        let tx = spawn_recorder(Arc::clone(&is_recording));

        Self { is_recording, tx }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn send_entry(&self, entry: RecordedEntry) {
        let _ = self.tx.send(RecordingEvent::Entry(entry));
    }

    pub fn start(&self) {
        let _ = self.tx.send(RecordingEvent::Start);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(RecordingEvent::Stop);
    }

    pub fn restart(&self) {
        let _ = self.tx.send(RecordingEvent::Restart);
    }

    /// Returns a snapshot of the current recording state and all entries.
    pub async fn query(&self) -> Option<RecorderSnapshot> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(RecordingEvent::Query(tx)).ok()?;
        rx.await.ok()
    }

    /// Returns a status of the current recording.
    pub async fn status(&self) -> Option<RecorderStatus> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(RecordingEvent::Status(tx)).ok()?;
        rx.await.ok()
    }
}

type QueryReply = oneshot::Sender<RecorderSnapshot>;
type StatusReply = oneshot::Sender<RecorderStatus>;

enum RecordingEvent {
    Entry(RecordedEntry),
    Start,
    Stop,
    Restart,
    Query(QueryReply),
    Status(StatusReply),
}

/// Spawns the recorder background task and returns a sender handle.
fn spawn_recorder(status: Arc<AtomicBool>) -> mpsc::UnboundedSender<RecordingEvent> {
    let (tx, mut rx) = mpsc::unbounded_channel::<RecordingEvent>();

    tokio::spawn(async move {
        let mut entries: Vec<RecordedEntry> = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                RecordingEvent::Entry(entry) => {
                    if status.load(Ordering::Relaxed) {
                        entries.push(entry);
                    }
                },

                RecordingEvent::Start => {
                    if status.load(Ordering::Relaxed) {
                        log::debug!("start ignored - already recording");
                    } else {
                        log::info!("started");
                        status.store(true, Ordering::Relaxed);
                    }
                },

                RecordingEvent::Stop => {
                    if status.load(Ordering::Relaxed) {
                        log::info!("stopped ({} entries)", entries.len());
                        status.store(false, Ordering::Relaxed);
                    } else {
                        log::debug!("stop ignored - not recording");
                    }
                },

                RecordingEvent::Restart => {
                    log::info!("restarted (discarding {} entries)", entries.len());
                    entries.clear();
                    status.store(true, Ordering::Relaxed);
                },

                RecordingEvent::Query(reply) => {
                    let _ = reply.send(RecorderSnapshot {
                        status: status.load(Ordering::Relaxed).into(),
                        entries: entries.clone(),
                    });
                },

                RecordingEvent::Status(reply) => {
                    let _ = reply.send(RecorderStatus {
                        status: status.load(Ordering::Relaxed).into(),
                        count: entries.len(),
                    });
                },
            }
        }

        log::warn!("channel closed, task exiting");
    });

    tx
}

fn snapshot_to_csv(snapshot: &RecorderSnapshot) -> String {
    const HEADER: &str = "timestamp_ms,transport,peer,valid,message,response\n";

    let mut out = String::with_capacity(HEADER.len() + snapshot.entries.len() * 128);
    out.push_str(HEADER);

    for e in &snapshot.entries {
        let _ = write!(out, "{},{},{},{},", e.timestamp_ms, e.transport, e.peer, e.valid);
        csv_write_escaped(&mut out, &e.message);
        out.push(',');
        if let Some(r) = e.response.as_deref() {
            csv_write_escaped(&mut out, r)
        }
        out.push('\n');
    }

    out
}

fn csv_write_escaped(out: &mut String, s: &str) {
    if !s.contains([',', '"', '\n', '\r']) {
        out.push_str(s);
        return;
    }

    out.push('"');

    let mut remaining = s;
    while let Some(pos) = remaining.find('"') {
        out.push_str(&remaining[..pos]);
        out.push_str("\"\"");
        remaining = &remaining[pos + 1..];
    }
    out.push_str(remaining);

    out.push('"');
}
