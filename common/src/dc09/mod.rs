use crc::Crc;

pub use self::ack_message::AckMessage;
pub use self::message::DC09Message;

mod ack_message;
mod message;

// Calculates CRC (CRC-16/ARC, as used in SIA DC-09).
pub fn calculate_crc(message: &str) -> u16 {
    let crc = Crc::<u16>::new(&crc::CRC_16_ARC);
    crc.checksum(message.as_bytes())
}
