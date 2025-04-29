use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_till, take_until, take_while_m_n, take_while1},
    combinator::{map, map_res, opt},
    multi::many0,
    sequence::{delimited, preceded},
};
use std::{fmt::Display, str};

use super::calculate_crc;

/// Possible DC09 message errors.
#[derive(thiserror::Error, Debug)]
pub enum DC09Error {
    /// Failed to parse DC09 message.
    #[error("failed to parse DC09 message")]
    ParseError,

    /// Invalid DC09 message length.
    #[error("invalid DC09 message length")]
    InvalidLength,

    /// Invalid DC09 message CRC.
    #[error("invalid DC09 message CRC")]
    InvalidCrc,

    /// Invalid sequence number for received DC09 message.
    #[error("invalid sequence number")]
    InvalidSequenceNumber,

    /// Invalid account number for received DC09 message.
    #[error("invalid account number")]
    InvalidAccountNumber,
}

/// Represents a DC09 message.
#[derive(Debug, PartialEq)]
pub struct DC09Message {
    pub token: String,
    pub sequence: u16,
    pub receiver: Option<String>,
    pub line_prefix: Option<String>,
    pub account: String,
    pub data: String,
    pub extended: Vec<String>,
    pub timestamp: Option<String>,
}

impl DC09Message {
    /// Creates new [`DC09Message`] instance.
    pub fn new(token: String, account: String, sequence: u16, data: String) -> Self {
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

impl TryFrom<&str> for DC09Message {
    type Error = DC09Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match parse_dc09(value) {
            Ok((_, message)) => match message.validate() {
                Ok(_) => Ok(message.message),
                Err(e) => Err(e),
            },
            Err(_) => Err(DC09Error::ParseError),
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
            self.data
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

/// Represents a parsed DC09 message that can be validated.
struct ParsedMessage<'a> {
    origin: &'a str,
    message: DC09Message,
    crc: u16,
    len: u16,
}

impl ParsedMessage<'_> {
    /// Validates length and CRC of the parsed message.
    fn validate(&self) -> Result<(), DC09Error> {
        let message_len = usize::from(self.len);
        // [\n] + [4 (crc)] + [4 (len)] + [\r] = 10
        if message_len != (self.origin.len() - 10) {
            Err(DC09Error::InvalidLength)
        } else {
            // [\n] + [4 (crc)] + [4 (len)] = 9
            let crc = calculate_crc(&self.origin[9..(message_len + 9)]);
            if self.crc != crc { Err(DC09Error::InvalidCrc) } else { Ok(()) }
        }
    }
}

/// Parses 4-digit hex number.
fn parse_hex(input: &str) -> IResult<&str, u16> {
    map_res(take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit()), |s: &str| {
        u16::from_str_radix(s, 16)
    })
    .parse(input)
}

/// Parses a message type enclosed in quotes.
fn parse_token(input: &str) -> IResult<&str, String> {
    map(delimited(tag("\""), take_until("\""), tag("\"")), |s: &str| s.to_string()).parse(input)
}

/// Parses a DC09 sequence number (4-digit number).
fn parse_sequence(input: &str) -> IResult<&str, u16> {
    map_res(take_while_m_n(4, 4, |c: char| c.is_ascii_digit()), |s: &str| s.parse::<u16>()).parse(input)
}

/// Parses a DC09 receiver number (1-6 hex number).
fn parse_receiver(input: &str) -> IResult<&str, String> {
    map(
        preceded(tag("R"), take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())),
        |s: &str| s.to_string(),
    )
    .parse(input)
}

/// Parses a DC09 account prefix (1-6 hex number).
fn parse_account_prefix(input: &str) -> IResult<&str, String> {
    map(
        preceded(tag("L"), take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())),
        |s: &str| s.to_string(),
    )
    .parse(input)
}

/// Parses an account number prefixed by '#'.
fn parse_account(input: &str) -> IResult<&str, String> {
    map(preceded(tag("#"), take_while1(|c: char| c.is_alphanumeric())), |s: &str| {
        s.to_string()
    })
    .parse(input)
}

/// Parses data enclosed in square brackets.
fn parse_data(input: &str) -> IResult<&str, String> {
    map(delimited(tag("["), take_until("]"), tag("]")), |s: &str| s.to_string()).parse(input)
}

/// Parses a complete DC09 message.  
/// Format example: `3BAC0029"SIA-DCS"0002#0123[#0123|Nti20:50:26RP99]`
fn parse_dc09(input: &str) -> IResult<&str, ParsedMessage> {
    map(
        (
            tag("\n"),
            parse_hex,
            parse_hex,
            parse_token,
            parse_sequence,
            opt(parse_receiver),
            opt(parse_account_prefix),
            parse_account,
            parse_data,
            many0(parse_data),
            opt((tag("_"), take_till(|c| c == '\r'))),
            tag("\r"),
        ),
        |(_, crc, len, token, sequence, receiver, line_prefix, account, data, extended, timestamp, _)| ParsedMessage {
            len,
            crc,
            origin: input,
            message: DC09Message {
                token,
                sequence,
                receiver,
                line_prefix,
                account,
                data,
                extended,
                timestamp: timestamp.map(|t| t.1.to_owned()),
            },
        },
    )
    .parse(input)
}
