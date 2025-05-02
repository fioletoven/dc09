use crc::Crc;

pub use self::cryptography::*;
pub use self::message::DC09Message;
pub use self::parser::*;

mod cryptography;
mod message;
mod parser;

// Calculates CRC (CRC-16/ARC, as used in SIA DC-09).
pub fn calculate_crc(message: &str) -> u16 {
    let crc = Crc::<u16>::new(&crc::CRC_16_ARC);
    crc.checksum(message.as_bytes())
}
