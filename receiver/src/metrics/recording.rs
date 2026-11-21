use aws_lc_rs::digest::{SHA256, digest};
use serde::Serialize;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::rustls::ServerConnection;
use tokio_rustls::rustls::pki_types::CertificateDer;
use x509_cert::Certificate;
use x509_cert::der::Decode;

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

#[derive(Debug, Clone, Serialize)]
pub struct ClientCertificate {
    pub subject: Option<String>,
    pub thumbprint: String,
}

impl From<&CertificateDer<'_>> for ClientCertificate {
    fn from(cert: &CertificateDer<'_>) -> Self {
        Self {
            subject: Certificate::from_der(cert.as_ref())
                .ok()
                .map(|cert| cert.tbs_certificate().subject().to_string()),
            thumbprint: hex::encode(digest(&SHA256, cert).as_ref()),
        }
    }
}

impl ClientCertificate {
    /// The end-entity certificate the peer presented, if any.
    pub fn from_connection(conn: &ServerConnection) -> Option<Self> {
        conn.peer_certificates().and_then(|chain| chain.first()).map(Self::from)
    }
}

/// A single captured DC09 exchange.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedEntry {
    pub timestamp_ms: u128,
    pub transport: Transport,
    pub peer: String,
    pub valid: bool,
    pub heartbeat: bool,
    pub message: String,
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_cert: Option<ClientCertificate>,
}

impl RecordedEntry {
    /// Creates a new recorded entry with the given parameters.
    pub fn new(transport: Transport, peer: SocketAddr, valid: bool, heartbeat: bool, message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis();

        Self {
            timestamp_ms,
            transport,
            peer: peer.to_string(),
            valid,
            heartbeat,
            message: message.into(),
            response: None,
            client_cert: None,
        }
    }

    /// Sets the response for the recorded entry.
    pub fn with_response(mut self, response: Option<String>) -> Self {
        self.response = response;
        self
    }

    /// Sets the client certificate for the recorded entry.
    pub fn with_client_certificate(mut self, certificate: Option<ClientCertificate>) -> Self {
        self.client_cert = certificate;
        self
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

#[derive(Debug, Clone)]
pub struct RecorderSnapshot {
    pub status: RecordingStatus,
    pub entries: Vec<Arc<RecordedEntry>>,
}

impl RecorderSnapshot {
    /// Yields the CSV document in ~64 KiB chunks.
    pub fn csv_chunks(self) -> CsvChunks {
        let include_cert = self.entries.iter().any(|e| e.client_cert.is_some());
        CsvChunks {
            entries: self.entries.into_iter(),
            include_cert,
            header_sent: false,
        }
    }

    /// Yields the JSON document in ~64 KiB chunks.
    pub fn json_chunks(self) -> JsonChunks {
        JsonChunks {
            status: self.status,
            entries: self.entries.into_iter(),
            started: false,
            first: true,
            done: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecorderStatus {
    pub status: RecordingStatus,
    pub messages: usize,
    pub heartbeats: usize,
    pub total: usize,
}

/// Spawns recorder task and keeps sender for the [`RecordingEvent`] to drive the recorder.
#[derive(Clone)]
pub struct RecorderHandle {
    is_recording: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<RecordingEvent>,
}

impl RecorderHandle {
    /// Creates new [`RecorderHandle`] instance and spawns recorder task.
    pub fn new() -> Self {
        let is_recording = Arc::new(AtomicBool::new(false));
        let tx = spawn_recorder(Arc::clone(&is_recording));

        Self { is_recording, tx }
    }

    /// Returns `true` if the recorder is in `recording` state.
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// Sends [`RecordedEntry`] to the recorder.\
    /// **Note** that it will be processed only if the recorder is in `recording` state.
    pub fn send_entry(&self, entry: RecordedEntry) {
        let _ = self.tx.send(RecordingEvent::Entry(entry));
    }

    /// Starts signals recording.
    pub fn start(&self) {
        let _ = self.tx.send(RecordingEvent::Start);
    }

    /// Stops signals recording.
    pub fn stop(&self) {
        let _ = self.tx.send(RecordingEvent::Stop);
    }

    /// Restart signals recording.\
    /// **Note** that it discards all previously recorded signals.
    pub fn restart(&self) {
        let _ = self.tx.send(RecordingEvent::Restart);
    }

    /// Returns a snapshot of the current recording state and all entries.
    pub async fn query(&self, messages: bool, heartbeats: bool) -> Option<RecorderSnapshot> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(RecordingEvent::Query(messages, heartbeats, tx)).ok()?;
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
    Query(bool, bool, QueryReply),
    Status(StatusReply),
}

/// Spawns the recorder background task and returns a sender handle.
fn spawn_recorder(status: Arc<AtomicBool>) -> mpsc::UnboundedSender<RecordingEvent> {
    let (tx, mut rx) = mpsc::unbounded_channel::<RecordingEvent>();

    tokio::spawn(async move {
        let mut entries: Vec<Arc<RecordedEntry>> = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                RecordingEvent::Entry(entry) => {
                    if status.load(Ordering::Relaxed) {
                        entries.push(Arc::new(entry));
                    }
                },
                RecordingEvent::Start => {
                    log::info!("started");
                    status.store(true, Ordering::Relaxed);
                },
                RecordingEvent::Stop => {
                    log::info!("stopped ({} entries)", entries.len());
                    status.store(false, Ordering::Relaxed);
                },
                RecordingEvent::Restart => {
                    log::info!("restarted (discarding {} entries)", entries.len());
                    entries.clear();
                    status.store(true, Ordering::Relaxed);
                },
                RecordingEvent::Query(messages, heartbeats, reply) => {
                    let _ = reply.send(RecorderSnapshot {
                        status: status.load(Ordering::Relaxed).into(),
                        entries: entries
                            .iter()
                            .filter(|e| if e.heartbeat { heartbeats } else { messages })
                            .cloned()
                            .collect(),
                    });
                },
                RecordingEvent::Status(reply) => {
                    let heartbeats = entries.iter().filter(|s| s.heartbeat).count();
                    let _ = reply.send(RecorderStatus {
                        status: status.load(Ordering::Relaxed).into(),
                        messages: entries.len().saturating_sub(heartbeats),
                        heartbeats,
                        total: entries.len(),
                    });
                },
            }
        }

        log::warn!("channel closed, task exiting");
    });

    tx
}

const CHUNK_SIZE: usize = 64 * 1024;

const CSV_HEADER: &str = "timestamp_ms,transport,peer,valid,heartbeat,message,response";
const CSV_HEADER_CERT: &str = ",client_cert_subject,client_cert_thumbprint";

/// Iterator over CSV chunks.
pub struct CsvChunks {
    entries: std::vec::IntoIter<Arc<RecordedEntry>>,
    include_cert: bool,
    header_sent: bool,
}

impl Iterator for CsvChunks {
    type Item = Result<String, !>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut out = String::with_capacity(CHUNK_SIZE + 1024);

        if !self.header_sent {
            out.push_str(CSV_HEADER);
            if self.include_cert {
                out.push_str(CSV_HEADER_CERT);
            }
            out.push('\n');
            self.header_sent = true;
        }

        while out.len() < CHUNK_SIZE {
            let Some(e) = self.entries.next() else { break };
            write_csv_row(&mut out, &e, self.include_cert);
        }

        (!out.is_empty()).then_some(Ok(out))
    }
}

fn write_csv_row(out: &mut String, e: &RecordedEntry, include_certificate: bool) {
    let _ = write!(
        out,
        "{},{},{},{},{},",
        e.timestamp_ms, e.transport, e.peer, e.valid, e.heartbeat
    );
    csv_write_escaped(out, &e.message);

    out.push(',');
    if let Some(r) = e.response.as_deref() {
        csv_write_escaped(out, r);
    }

    if include_certificate {
        out.push(',');
        if let Some(cert) = &e.client_cert {
            if let Some(subject) = &cert.subject {
                csv_write_escaped(out, subject);
            }
            out.push(',');
            out.push_str(&cert.thumbprint);
        } else {
            out.push(',');
        }
    }

    out.push('\n');
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

/// Iterator over JSON chunks.
pub struct JsonChunks {
    status: RecordingStatus,
    entries: std::vec::IntoIter<Arc<RecordedEntry>>,
    started: bool,
    first: bool,
    done: bool,
}

impl Iterator for JsonChunks {
    type Item = Result<Vec<u8>, !>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut out: Vec<u8> = Vec::with_capacity(CHUNK_SIZE + 1024);

        if !self.started {
            out.extend_from_slice(br#"{"status":"#);
            let _ = serde_json::to_writer(&mut out, &self.status);
            out.extend_from_slice(br#","entries":["#);
            self.started = true;
        }

        while out.len() < CHUNK_SIZE {
            match self.entries.next() {
                Some(e) => {
                    let rollback = out.len();
                    if !self.first {
                        out.push(b',');
                    }
                    if let Err(err) = serde_json::to_writer(&mut out, &*e) {
                        log::error!("failed to serialize entry: {err}");
                        out.truncate(rollback);
                        continue;
                    }
                    self.first = false;
                },
                None => {
                    out.extend_from_slice(b"]}");
                    self.done = true;
                    break;
                },
            }
        }

        Some(Ok(out))
    }
}
