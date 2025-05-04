use std::{fmt::Display, str};
use time::{OffsetDateTime, macros::format_description};

use super::{DC09Error, calculate_crc, encrypt, parse_dc09};

#[cfg(test)]
#[path = "./message.tests.rs"]
mod message_tests;

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

    /// Creates new acknowledgement [`DC09Message`] instance with ID token and timestamp.
    pub fn ack(token: String, account: String, sequence: u16) -> Self {
        Self::new(token, account, sequence, None).with_timestamp(OffsetDateTime::now_utc())
    }

    // Adds UTC timestamp to the DC09 message.
    pub fn with_timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        let format = format_description!("[hour]:[minute]:[second],[month]-[day]-[year]");
        self.timestamp = Some(timestamp.format(&format).expect("Failed to format timestamp"));
        self
    }

    /// Adds Receiver to the DC09 message.  
    /// **Note** that it should contain `R` as a prefix.
    pub fn with_receiver(mut self, receiver: Option<String>) -> Self {
        if let Some(receiver) = receiver {
            if receiver.chars().next().is_some_and(|ch| ch == 'R') {
                self.receiver = Some(receiver);
            }
        }
        self
    }

    /// Adds Line Prefix to the DC09 message.  
    /// **Note** that it should contain `L` as a prefix.
    pub fn with_line_prefix(mut self, line_prefix: Option<String>) -> Self {
        if let Some(line_prefix) = line_prefix {
            if line_prefix.chars().next().is_some_and(|ch| ch == 'L') {
                self.line_prefix = Some(line_prefix);
            }
        }
        self
    }

    /// Tries to create [`DC09Message`] from the provided string slice.
    pub fn try_from(value: &str, key: Option<&str>) -> Result<Self, DC09Error> {
        parse_dc09(value, key)
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

    /// Returns `true` if the DC09 message was (or should be) encrypted.
    pub fn was_encrypted(&self) -> bool {
        self.token.chars().next().is_some_and(|ch| ch == '*')
    }

    /// Converts the [`DC09Message`] to encrypted `String` representation.
    pub fn to_encrypted(&self, key: &str) -> Option<String> {
        let mut payload = self.get_payload();
        let data = if self.data.as_ref().is_some_and(|d| !d.is_empty()) {
            payload.replace_range(0..1, "|");
            encrypt(&payload, key.as_bytes())?
        } else {
            encrypt(&payload[1..], key.as_bytes())?
        };
        let body = format!(
            "\"{}{}\"{:04}{}{}#{}[{}",
            if self.token.chars().next().is_none_or(|ch| ch != '*') {
                "*"
            } else {
                ""
            },
            self.token,
            self.sequence,
            self.receiver.as_deref().unwrap_or(""),
            self.line_prefix.as_deref().unwrap_or("L0"),
            self.account,
            data,
        );

        Some(format!("\x0A{:04X}{:04X}{}\x0D", calculate_crc(&body), body.len(), body))
    }

    fn get_payload(&self) -> String {
        let len = self.data.as_ref().map(|d| d.len() + 2).unwrap_or(2);
        let len = self.extended.iter().map(|e| e.len() + 2).sum::<usize>() + len + 21; // + timestamp
        let mut payload = String::with_capacity(len);

        payload.push('[');
        if let Some(data) = &self.data {
            payload.push_str(data);
        }
        payload.push(']');

        for data in &self.extended {
            payload.push('[');
            payload.push_str(data);
            payload.push(']');
        }

        if let Some(timestamp) = &self.timestamp {
            payload.push('_');
            payload.push_str(timestamp);
        }

        payload
    }
}

impl Display for DC09Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = format!(
            "\"{}\"{:04}{}{}#{}{}",
            self.token,
            self.sequence,
            self.receiver.as_deref().unwrap_or(""),
            self.line_prefix.as_deref().unwrap_or("L0"),
            self.account,
            self.get_payload(),
        );

        write!(f, "\x0A{:04X}{:04X}{}\x0D", calculate_crc(&body), body.len(), body)
    }
}
