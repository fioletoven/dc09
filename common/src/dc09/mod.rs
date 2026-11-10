pub use self::cryptography::*;
pub use self::frame::{FrameBuffer, FrameError};
pub use self::message::DC09Message;
pub use self::parser::*;

mod cryptography;
mod frame;
mod message;
mod parser;

/// DC09 timestamp format.
pub const TIMESTAMP_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[hour]:[minute]:[second],[month]-[day]-[year]");

/// Calculates CRC (CRC-16/ARC, as used in DC09).
pub fn calculate_crc(message: &[u8]) -> u16 {
    let crc = crc::Crc::<u16>::new(&crc::CRC_16_ARC);
    crc.checksum(message)
}

/// Returns `true` if specified token is a heartbeat token.
pub fn is_heartbeat(token: &str) -> bool {
    token == "NULL" || token == "*NULL"
}
