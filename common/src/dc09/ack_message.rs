use std::fmt::Display;
use time::{OffsetDateTime, macros::format_description};

use crate::dc09::calculate_crc;

// Represents an ACK message.
pub struct AckMessage {
    sequence: u16,
    receiver: Option<String>,
    line_prefix: Option<String>,
    account: String,
    timestamp: String,
}

impl AckMessage {
    pub fn new(seq: u16, receiver: Option<&str>, line_prefix: Option<&str>, account: &str) -> Self {
        AckMessage {
            sequence: seq,
            receiver: receiver.map(String::from),
            line_prefix: line_prefix.map(String::from),
            account: format!("#{}", account),
            timestamp: get_timestamp(),
        }
    }
}

impl Display for AckMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = format!(
            "\"ACK\"{:04}{}{}{}[]{}",
            self.sequence,
            self.receiver.as_deref().unwrap_or(""),
            self.line_prefix.as_deref().unwrap_or(""),
            self.account,
            self.timestamp
        );

        write!(f, "\x0A{:04X}{:04X}{}\x0D", calculate_crc(&body), body.len(), body)
    }
}

// Gets current UTC timestamp in DC-09 format.
fn get_timestamp() -> String {
    let format = format_description!("_[hour]:[minute]:[second],[month]-[day]-[year]");
    let now = OffsetDateTime::now_utc();

    now.format(&format).expect("Failed to format timestamp")
}
