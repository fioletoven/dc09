use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until, take_while_m_n, take_while1};
use nom::combinator::{map, map_res, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};
use std::borrow::Cow;

use crate::dc09::{DC09Message, calculate_crc, decrypt};

/// Possible DC09 message parse errors.
#[derive(thiserror::Error, Debug)]
pub enum DC09Error {
    /// Invalid UTF-8 sequence.
    #[error("invalid UTF-8 sequence")]
    InvalidUtf8(#[from] std::str::Utf8Error),

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

    /// Invalid DC09 timestamp (or out of -40 to +20 window).
    #[error("invalid DC09 timestamp")]
    InvalidTimestamp,
}

impl DC09Error {
    /// Returns static str for the error variant.
    pub fn reason(&self) -> &'static str {
        match self {
            DC09Error::InvalidUtf8(_) => "InvalidUtf8",
            DC09Error::ParseHeaderError => "ParseHeaderError",
            DC09Error::ParsePayloadError => "ParsePayloadError",
            DC09Error::DecryptError => "DecryptError",
            DC09Error::InvalidLength => "InvalidLength",
            DC09Error::InvalidCrc => "InvalidCrc",
            DC09Error::InvalidSequenceNumber => "InvalidSequenceNumber",
            DC09Error::InvalidAccountNumber => "InvalidAccountNumber",
            DC09Error::InvalidTimestamp => "InvalidTimestamp",
        }
    }
}

/// Decodes DC09 text as UTF-8, falling back to Windows-1252 for any extended data that isn't valid UTF-8.
pub fn decode_dc09_text(input: &[u8]) -> Cow<'_, str> {
    match str::from_utf8(input) {
        Ok(text) => Cow::Borrowed(text),
        Err(_) => {
            let mut text = String::with_capacity(input.len());
            for field in input.split_inclusive(|byte| matches!(byte, b'[' | b']')) {
                match str::from_utf8(field) {
                    Ok(field) => text.push_str(field),
                    Err(_) => text.push_str(&encoding_rs::WINDOWS_1252.decode_without_bom_handling(field).0),
                }
            }
            Cow::Owned(text)
        },
    }
}

/// Parses the fixed-size DC09 frame prefix: `<LF><crc><len>`.
pub fn parse_frame_prefix(input: &[u8]) -> IResult<&[u8], (u16, u16)> {
    map((tag("\n"), parse_hex, parse_hex), |(_, crc, len)| (crc, len)).parse(input)
}

/// Parses an account name from the DC09 message.\
/// **Note** that this function does not validate CRC of the message.
pub fn parse_dc09_account_name(input: &str) -> Result<String, DC09Error> {
    let Ok((_, header)) = parse_dc09_header(input.as_bytes()) else {
        return Err(DC09Error::ParseHeaderError);
    };
    Ok(header.account.to_owned())
}

/// Parses a complete DC09 message.\
/// Format example: `3BAC0029"SIA-DCS"0002#0123[#0123|Nti20:50:26RP99]`
pub fn parse_dc09(input: &[u8], key: Option<&str>) -> Result<DC09Message, DC09Error> {
    let Ok((payload, header)) = parse_dc09_header(input) else {
        return Err(DC09Error::ParseHeaderError);
    };

    validate(input, header.len, header.crc)?;

    let is_encrypted = header.token.starts_with('*');
    let decrypted = if is_encrypted {
        let key = key.ok_or(DC09Error::DecryptError)?;
        let ciphertext = payload
            .strip_prefix(b"[")
            .and_then(|payload| payload.strip_suffix(b"\r"))
            .filter(|ciphertext| !ciphertext.is_empty())
            .ok_or(DC09Error::DecryptError)?;
        let ciphertext = str::from_utf8(ciphertext).map_err(|_| DC09Error::DecryptError)?;
        let mut plaintext = decrypt(ciphertext, key.as_bytes()).ok_or(DC09Error::DecryptError)?;
        let pad_end = plaintext
            .iter()
            .position(|byte| matches!(byte, b'|' | b'[' | b']'))
            .filter(|position| plaintext[*position] == b'|')
            .ok_or(DC09Error::DecryptError)?;
        plaintext.drain(..=pad_end);
        Some(plaintext)
    } else {
        None
    };

    let payload = parse_dc09_payload(decrypted.as_deref().unwrap_or(payload));

    match payload {
        Ok(p) => {
            let data = str::from_utf8(p.1.data)?.to_owned();
            Ok(DC09Message {
                token: header.token.to_string(),
                sequence: header.sequence,
                receiver: header.receiver.map(|r| format!("R{r}")),
                line_prefix: header.line_prefix.map(|l| format!("L{l}")),
                account: header.account.to_string(),
                data: if data.is_empty() { None } else { Some(data) },
                extended: p.1.extended.iter().map(|data| decode_dc09_text(data).into_owned()).collect(),
                timestamp: p.1.timestamp.map(str::from_utf8).transpose()?.map(str::to_owned),
            })
        },
        Err(_) => Err(DC09Error::ParsePayloadError),
    }
}

/// Validates account prefix (receiver line number) and returns `true` on success.
pub fn is_account_prefix_valid(input: &str) -> bool {
    if let Ok((_, line)) = parse_account_prefix(input.as_bytes())
        && line.len() == input.len().saturating_sub(1)
    {
        return true;
    }

    false
}

/// Validates receiver number and returns `true` on success.
pub fn is_receiver_valid(input: &str) -> bool {
    if let Ok((_, line)) = parse_receiver(input.as_bytes())
        && line.len() == input.len().saturating_sub(1)
    {
        return true;
    }

    false
}

/// Validates length and CRC of the parsed message.
fn validate(input: &[u8], len: u16, crc: u16) -> Result<(), DC09Error> {
    let message_len = usize::from(len);
    // [\n] + [4 (crc)] + [4 (len)] + [\r] = 10
    if Some(message_len) == input.len().checked_sub(10) {
        // [\n] + [4 (crc)] + [4 (len)] = 9
        let new_crc = calculate_crc(&input[9..(message_len + 9)]);
        if crc == new_crc { Ok(()) } else { Err(DC09Error::InvalidCrc) }
    } else {
        Err(DC09Error::InvalidLength)
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
    data: &'a [u8],
    extended: Vec<&'a [u8]>,
    timestamp: Option<&'a [u8]>,
}

/// Parses DC09 message header.
fn parse_dc09_header(input: &[u8]) -> IResult<&[u8], ParsedHeader<'_>> {
    map(
        (
            tag("\n"),
            parse_hex,
            parse_hex,
            parse_token,
            parse_sequence,
            alt((
                fixed_nak_account,
                (opt(parse_receiver), opt(parse_account_prefix), parse_account),
            )),
        ),
        |(_, crc, len, token, sequence, (receiver, line_prefix, account))| ParsedHeader {
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
fn parse_dc09_payload(input: &'_ [u8]) -> IResult<&'_ [u8], ParsedPayload<'_>> {
    map(
        (
            parse_data,
            many0(parse_extended_data),
            opt((tag(&b"_"[..]), take_till(|byte| byte == b'\r'))),
            opt(tag(&b"\r"[..])),
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
fn parse_hex(input: &[u8]) -> IResult<&[u8], u16> {
    map_res(
        map_res(take_while_m_n(4, 4, |b: u8| b.is_ascii_hexdigit()), str::from_utf8),
        |s: &str| u16::from_str_radix(s, 16),
    )
    .parse(input)
}

/// Parses a message type enclosed in quotes.
fn parse_token(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(delimited(tag("\""), take_until("\""), tag("\"")), str::from_utf8).parse(input)
}

/// Parses a DC09 sequence number (4-digit number).
fn parse_sequence(input: &[u8]) -> IResult<&[u8], u16> {
    map_res(
        map_res(take_while_m_n(4, 4, |b: u8| b.is_ascii_digit()), str::from_utf8),
        |s: &str| s.parse::<u16>(),
    )
    .parse(input)
}

/// Receiver, line prefix and account parsed from the DC09 header.
type AccountParts<'a> = (Option<&'a str>, Option<&'a str>, &'a str);

/// Parses fixed account for NAK messages: `R0L0A0`.
fn fixed_nak_account(input: &[u8]) -> IResult<&[u8], AccountParts<'_>> {
    map(tag("R0L0A0"), |_| (None, None, "A0")).parse(input)
}

/// Parses a DC09 receiver number (1-6 hex number).
fn parse_receiver(input: &[u8]) -> IResult<&[u8], &str> {
    preceded(
        tag("R"),
        map_res(take_while_m_n(1, 6, |b: u8| b.is_ascii_hexdigit()), str::from_utf8),
    )
    .parse(input)
}

/// Parses a DC09 account prefix (1-6 hex number).
fn parse_account_prefix(input: &[u8]) -> IResult<&[u8], &str> {
    preceded(
        tag("L"),
        map_res(take_while_m_n(1, 6, |b: u8| b.is_ascii_hexdigit()), str::from_utf8),
    )
    .parse(input)
}

/// Parses an account number prefixed by '#'.
fn parse_account(input: &[u8]) -> IResult<&[u8], &str> {
    preceded(
        tag("#"),
        map_res(take_while1(|b: u8| b.is_ascii_alphanumeric()), str::from_utf8),
    )
    .parse(input)
}

/// Parses data enclosed in square brackets.
fn parse_data(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(opt(tag(&b"["[..])), take_until(&b"]"[..]), tag(&b"]"[..])).parse(input)
}

/// Parses extended data enclosed in square brackets.
fn parse_extended_data(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag(&b"["[..]), take_until(&b"]"[..]), tag(&b"]"[..])).parse(input)
}
