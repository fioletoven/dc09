use common::dc09::{DC09Error, DC09Message};
use common::logging::DisplayMode;
use std::borrow::Cow;
use time::OffsetDateTime;

use crate::{metrics, server::AckMode};

pub fn build_response_message(msg: DC09Message, key: Option<&str>, ack: AckMode) -> String {
    let was_encrypted = msg.was_encrypted();
    let response = match ack {
        AckMode::Ack => DC09Message::ack(msg.account, msg.sequence)
            .with_receiver(msg.receiver)
            .with_line_prefix(msg.line_prefix),
        AckMode::Nak => DC09Message::nak(),
        AckMode::Duh => DC09Message::duh(msg.account, msg.sequence)
            .with_receiver(msg.receiver)
            .with_line_prefix(msg.line_prefix),
    };

    if was_encrypted && ack == AckMode::Ack {
        if let Some(key) = key {
            response
                .to_encrypted(key)
                .expect("Cannot encrypt DC09 message with the provided key")
        } else {
            response.to_string()
        }
    } else {
        response.to_string()
    }
}

pub fn process_valid_message_metrics(transport: &str, raw_message: &str, parsed_message: &DC09Message) {
    metrics::messages_received()
        .with_label_values(&[&parsed_message.token, &parsed_message.account])
        .inc();

    metrics::message_size_bytes()
        .with_label_values(&[transport])
        .observe(raw_message.len() as f64);

    metrics::last_message_timestamp()
        .with_label_values(&[&parsed_message.account])
        .set(OffsetDateTime::now_utc().unix_timestamp() as f64);

    if parsed_message.is_heartbeat() {
        metrics::heartbeats_received()
            .with_label_values(&[&parsed_message.account])
            .inc();
    }
}

pub fn process_invalid_message_metrics(transport: &str, raw_message: &str, error: &DC09Error) {
    metrics::message_size_bytes()
        .with_label_values(&[transport])
        .observe(raw_message.len() as f64);

    metrics::messages_failed()
        .with_label_values(&[transport, error.reason()])
        .inc();
}

#[inline]
pub fn increase_total_connections(transport: &str) {
    metrics::connections_total().with_label_values(&[transport]).inc();
}

#[inline]
pub fn increase_active_connections() {
    metrics::active_connections().inc();
}

#[inline]
pub fn decrease_active_connections() {
    metrics::active_connections().dec();
}

pub fn get_received_message<'a>(unmodified: &'a str, msg: &DC09Message, mode: DisplayMode) -> Cow<'a, str> {
    match mode {
        DisplayMode::Target => unmodified.trim().into(),
        DisplayMode::Plain => msg.to_string().trim().to_string().into(),
        DisplayMode::Both => {
            if msg.was_encrypted() {
                format!("{} → {}", unmodified.trim(), msg.to_string().trim()).into()
            } else {
                msg.to_string().trim().to_string().into()
            }
        },
    }
}
