use prometheus::{IntCounterVec, Opts};
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

/// Call once at startup to register all metrics with the default registry.
pub fn register_all() {
    let registry = prometheus::default_registry();

    registry
        .register(Box::new(messages_received().clone()))
        .expect("metric registered");
}
