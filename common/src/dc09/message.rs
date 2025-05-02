use std::{fmt::Display, str};
use time::{OffsetDateTime, macros::format_description};

use super::{DC09Error, calculate_crc, parse_dc09};

/// Represents a DC09 message.
#[derive(Debug, PartialEq)]
pub struct DC09Message {
    pub token: String,
    pub sequence: u16,
    pub receiver: Option<String>,
    pub line_prefix: Option<String>,
    pub account: String,
    pub data: Option<String>,
    pub extended: Vec<String>,
    pub timestamp: Option<String>,
}

impl DC09Message {
    /// Creates new [`DC09Message`] instance.
    pub fn new(token: String, account: String, sequence: u16, data: Option<String>) -> Self {
        Self {
            token,
            sequence,
            receiver: None,
            line_prefix: None,
            account,
            data,
            extended: Vec::new(),
            timestamp: None,
        }
    }

    /// Tries to create [`DC09Message`] from the provided string slice.
    pub fn try_from(value: &str, key: Option<&str>) -> Result<Self, DC09Error> {
        parse_dc09(value, key)
    }

    /// Creates new [`DC09Message`] instance with `ACK` ID token.
    pub fn ack(account: String, sequence: u16) -> Self {
        Self::new("ACK".to_owned(), account, sequence, None).with_timestamp(OffsetDateTime::now_utc())
    }

    // Adds UTC timestamp to DC-09 message.
    pub fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        let format = format_description!("[hour]:[minute]:[second],[month]-[day]-[year]");
        self.timestamp = Some(timestamp.format(&format).expect("Failed to format timestamp"));
        self
    }

    /// Validates account and sequence numbers in the DC09 message.
    pub fn validate(&self, account: &str, sequence: u16) -> Result<(), DC09Error> {
        if self.sequence != sequence {
            Err(DC09Error::InvalidSequenceNumber)
        } else if self.account != account {
            Err(DC09Error::InvalidAccountNumber)
        } else {
            Ok(())
        }
    }
}

impl Display for DC09Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut body = format!(
            "\"{}\"{:04}{}{}#{}[{}]",
            self.token,
            self.sequence,
            self.receiver.as_deref().unwrap_or(""),
            self.line_prefix.as_deref().unwrap_or(""),
            self.account,
            self.data.as_deref().unwrap_or(""),
        );

        for data in &self.extended {
            body.push('[');
            body.push_str(data);
            body.push(']');
        }

        if let Some(timestamp) = &self.timestamp {
            body.push('_');
            body.push_str(timestamp);
        }

        write!(f, "\x0A{:04X}{:04X}{}\x0D", calculate_crc(&body), body.len(), body)
    }
}
