use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_till, take_until, take_while_m_n, take_while1},
    combinator::{map, map_res, opt},
    multi::many0,
    sequence::{delimited, preceded},
};

use crate::dc09::decrypt;

use super::{DC09Message, calculate_crc};

/// Possible DC09 message parse errors.
#[derive(thiserror::Error, Debug)]
pub enum DC09Error {
    /// Failed to parse DC09 message header.
    #[error("failed to parse DC09 message header")]
    ParseHeaderError,

    /// Failed to parse DC09 message payload.
    #[error("failed to parse DC09 message payload")]
    ParsePayloadError,

    /// Failed to decrypt DC09 message.
    #[error("failed to decrypt DC09 message")]
    DecryptError,

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

/// Parses a complete DC09 message.  
/// Format example: `3BAC0029"SIA-DCS"0002#0123[#0123|Nti20:50:26RP99]`
pub fn parse_dc09(input: &str, key: Option<&str>) -> Result<DC09Message, DC09Error> {
    let Ok((payload, header)) = parse_dc09_header(input) else {
        return Err(DC09Error::ParseHeaderError);
    };

    validate(input, header.len, header.crc)?;

    let is_encrypted = header.token.chars().next().is_some_and(|ch| ch == '*');
    let decrypted = if is_encrypted {
        if let Some(key) = key {
            decrypt(&payload[1..payload.len() - 1], key.as_bytes())
        } else {
            return Err(DC09Error::DecryptError);
        }
    } else {
        None
    };

    let payload = match &decrypted {
        Some(msg) => parse_dc09_payload(msg),
        None => parse_dc09_payload(payload),
    };

    match payload {
        Ok(p) => Ok(DC09Message {
            token: header.token.to_string(),
            sequence: header.sequence,
            receiver: header.receiver.map(|r| format!("R{}", r)),
            line_prefix: header.line_prefix.map(|l| format!("L{}", l)),
            account: header.account.to_string(),
            data: if p.1.data.is_empty() {
                None
            } else if is_encrypted {
                Some(remove_padding(p.1.data))
            } else {
                Some(p.1.data.to_owned())
            },
            extended: p.1.extended.iter().map(|i| (*i).to_owned()).collect::<Vec<_>>(),
            timestamp: p.1.timestamp.map(String::from),
        }),
        Err(_) => Err(DC09Error::ParsePayloadError),
    }
}

/// Validates length and CRC of the parsed message.
fn validate(input: &str, len: u16, crc: u16) -> Result<(), DC09Error> {
    let message_len = usize::from(len);
    // [\n] + [4 (crc)] + [4 (len)] + [\r] = 10
    if message_len != (input.len() - 10) {
        Err(DC09Error::InvalidLength)
    } else {
        // [\n] + [4 (crc)] + [4 (len)] = 9
        let new_crc = calculate_crc(&input[9..(message_len + 9)]);
        if crc != new_crc { Err(DC09Error::InvalidCrc) } else { Ok(()) }
    }
}

fn remove_padding(data: &str) -> String {
    if let Some(data) = data.split_once('|').map(|x| x.1) {
        data.to_owned()
    } else {
        data.to_owned()
    }
}

struct ParsedHeader<'a> {
    crc: u16,
    len: u16,
    token: &'a str,
    sequence: u16,
    receiver: Option<&'a str>,
    line_prefix: Option<&'a str>,
    account: &'a str,
}

struct ParsedPayload<'a> {
    data: &'a str,
    extended: Vec<&'a str>,
    timestamp: Option<&'a str>,
}

/// Parses DC09 message header.
fn parse_dc09_header(input: &str) -> IResult<&str, ParsedHeader> {
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
        ),
        |(_, crc, len, token, sequence, receiver, line_prefix, account)| ParsedHeader {
            crc,
            len,
            token,
            sequence,
            receiver,
            line_prefix,
            account,
        },
    )
    .parse(input)
}

/// Parses decrypted DC09 message payload.
fn parse_dc09_payload(input: &str) -> IResult<&str, ParsedPayload> {
    map(
        (
            parse_data,
            many0(parse_extended_data),
            opt((tag("_"), take_till(|c| c == '\r'))),
            opt(tag("\r")),
        ),
        |(data, extended, timestamp, _)| ParsedPayload {
            data,
            extended,
            timestamp: timestamp.map(|t| t.1),
        },
    )
    .parse(input)
}

/// Parses 4-digit hex number.
fn parse_hex(input: &str) -> IResult<&str, u16> {
    map_res(take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit()), |s: &str| {
        u16::from_str_radix(s, 16)
    })
    .parse(input)
}

/// Parses a message type enclosed in quotes.
fn parse_token(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), take_until("\""), tag("\"")).parse(input)
}

/// Parses a DC09 sequence number (4-digit number).
fn parse_sequence(input: &str) -> IResult<&str, u16> {
    map_res(take_while_m_n(4, 4, |c: char| c.is_ascii_digit()), |s: &str| s.parse::<u16>()).parse(input)
}

/// Parses a DC09 receiver number (1-6 hex number).
fn parse_receiver(input: &str) -> IResult<&str, &str> {
    preceded(tag("R"), take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())).parse(input)
}

/// Parses a DC09 account prefix (1-6 hex number).
fn parse_account_prefix(input: &str) -> IResult<&str, &str> {
    preceded(tag("L"), take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())).parse(input)
}

/// Parses an account number prefixed by '#'.
fn parse_account(input: &str) -> IResult<&str, &str> {
    preceded(tag("#"), take_while1(|c: char| c.is_alphanumeric())).parse(input)
}

/// Parses data enclosed in square brackets.
fn parse_data(input: &str) -> IResult<&str, &str> {
    delimited(opt(tag("[")), take_until("]"), tag("]")).parse(input)
}

/// Parses extended data enclosed in square brackets.
fn parse_extended_data(input: &str) -> IResult<&str, &str> {
    delimited(tag("["), take_until("]"), tag("]")).parse(input)
}
