use prometheus::{GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts};
use std::sync::OnceLock;

/// Counter for total DC-09 messages received.
pub fn messages_received() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        IntCounterVec::new(
            Opts::new("dc09_messages_received_total", "Total DC-09 messages received"),
            &["token", "account"],
        )
        .expect("metric can be created")
    })
}

/// Counter for total DC-09 messages that failed processing.
pub fn messages_failed() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        IntCounterVec::new(
            Opts::new("dc09_messages_failed_total", "Total DC-09 messages that failed processing"),
            &["transport", "reason"],
        )
        .expect("metric can be created")
    })
}

/// Counter for total connections accepted.
pub fn connections_total() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        IntCounterVec::new(
            Opts::new("dc09_connections_total", "Total connections accepted"),
            &["transport"],
        )
        .expect("metric can be created")
    })
}

/// Counter for total heartbeat/null messages received.
pub fn heartbeats_received() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        IntCounterVec::new(
            Opts::new("dc09_heartbeat_received_total", "Total heartbeat/null messages received"),
            &["account"],
        )
        .expect("metric can be created")
    })
}

/// Gauge for the number of currently active connections.
pub fn active_connections() -> &'static IntGauge {
    static METRIC: OnceLock<IntGauge> = OnceLock::new();
    METRIC.get_or_init(|| {
        IntGauge::new("dc09_active_connections", "Number of currently active connections").expect("metric can be created")
    })
}

/// Gauge for the last message received per account.
pub fn last_message_timestamp() -> &'static GaugeVec {
    static METRIC: OnceLock<GaugeVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        GaugeVec::new(
            Opts::new(
                "dc09_last_message_timestamp_seconds",
                "Unix timestamp of last message received per account",
            ),
            &["account"],
        )
        .expect("metric can be created")
    })
}

/// Histogram for size of received DC-09 messages in bytes.
pub fn message_size_bytes() -> &'static HistogramVec {
    static METRIC: OnceLock<HistogramVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        HistogramVec::new(
            HistogramOpts::new("dc09_message_size_bytes", "Size of received DC-09 messages in bytes")
                .buckets(vec![16.0, 32.0, 48.0, 64.0, 128.0, 256.0, 512.0, 1024.0, 1536.0]),
            &["transport"],
        )
        .expect("metric can be created")
    })
}

/// Call once at startup to register all metrics with the default registry.
pub fn register_all() {
    let registry = prometheus::default_registry();

    registry
        .register(Box::new(messages_received().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(messages_failed().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(connections_total().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(heartbeats_received().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(active_connections().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(last_message_timestamp().clone()))
        .expect("metric registered");
    registry
        .register(Box::new(message_size_bytes().clone()))
        .expect("metric registered");

    for transport in &["TCP", "UDP"] {
        connections_total().with_label_values(&[transport]);
        message_size_bytes().with_label_values(&[transport]);
    }
}
