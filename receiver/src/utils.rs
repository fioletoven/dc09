use clap::ValueEnum;
use common::dc09::DC09Message;
use std::net::SocketAddr;

pub fn build_response_message(msg: DC09Message, key: Option<&str>, nak: bool) -> String {
    let was_encrypted = msg.was_encrypted();
    let response = if nak {
        DC09Message::nak()
    } else {
        DC09Message::ack(msg.account, msg.sequence)
            .with_receiver(msg.receiver)
            .with_line_prefix(msg.line_prefix)
    };

    if was_encrypted && !nak {
        if let Some(key) = key {
            response
                .to_encrypted(key)
                .expect("Cannot encrypt DC09 message with the provided key")
        } else {
            response.to_string()
        }
    } else {
        response.to_string()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
pub enum DisplayMode {
    Unmodified,
    Decrypted,
    Both,
}

pub fn log_received_message(addr: &SocketAddr, unmodified: &str, msg: &DC09Message, mode: DisplayMode) {
    let addr = addr.to_string();
    match mode {
        DisplayMode::Unmodified => log::info!("{} -> {}", addr, unmodified.trim()),
        DisplayMode::Decrypted => log::info!("{} -> {}", addr, msg.to_string().trim()),
        DisplayMode::Both => {
            if msg.was_encrypted() {
                log::info!("{} -> {} >> {}", addr, unmodified.trim(), msg.to_string().trim())
            } else {
                log::info!("{} -> {}", addr, msg.to_string().trim())
            }
        },
    }
}
