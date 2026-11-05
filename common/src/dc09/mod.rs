pub use self::cryptography::*;
pub use self::frame::{FrameBuffer, FrameError};
pub use self::message::DC09Message;
pub use self::parser::*;

mod cryptography;
mod frame;
mod message;
mod parser;

// Calculates CRC (CRC-16/ARC, as used in SIA DC-09).
pub fn calculate_crc(message: &str) -> u16 {
    let crc = crc::Crc::<u16>::new(&crc::CRC_16_ARC);
    crc.checksum(message.as_bytes())
}
