use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_until, take_while_m_n, take_while1},
    combinator::{map, map_res, opt},
    multi::many0,
    sequence::{delimited, preceded},
};
use std::str;

use super::calculate_crc;

pub enum DC09Error {
    ParseError,
    ValidationError,
}

/// Represents a parsed DC09 message.
#[derive(Debug, PartialEq)]
pub struct DC09Message {
    pub crc: u16,
    pub len: u16,
    pub token: String,
    pub sequence: u16,
    pub receiver: Option<String>,
    pub line_prefix: Option<String>,
    pub account: String,
    pub data: String,
    pub extended: Vec<String>,
}

impl TryFrom<&str> for DC09Message {
    type Error = DC09Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match parse_dc09(value) {
            Ok((_, message)) => validate_message(value, message),
            Err(_) => Err(DC09Error::ParseError),
        }
    }
}

/// Validates length and CRC of the parsed message.
fn validate_message(origin: &str, message: DC09Message) -> Result<DC09Message, DC09Error> {
    // [\n] + [4 (crc)] + [4 (len)] + [\r] = 10
    if message.len != (origin.len() as u16 - 10) {
        Err(DC09Error::ValidationError)
    } else {
        // [\n] + [4 (crc)] + [4 (len)] = 9
        let crc = calculate_crc(&origin[9..(message.len as usize + 9)]);
        if message.crc != crc {
            Err(DC09Error::ValidationError)
        } else {
            Ok(message)
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
fn parse_dc09(input: &str) -> IResult<&str, DC09Message> {
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
            tag("\r"),
        ),
        |(_, crc, len, token, sequence, receiver, line_prefix, account, data, extended, _)| DC09Message {
            crc,
            len,
            token,
            sequence,
            receiver,
            line_prefix,
            account,
            data,
            extended,
        },
    )
    .parse(input)
}
