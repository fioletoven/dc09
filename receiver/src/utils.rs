use common::{dc09::DC09Message, logging::DisplayMode};
use std::borrow::Cow;

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

pub fn get_received_message<'a>(unmodified: &'a str, msg: &DC09Message, mode: DisplayMode) -> Cow<'a, str> {
    match mode {
        DisplayMode::Target => unmodified.trim().into(),
        DisplayMode::Plain => msg.to_string().trim().to_string().into(),
        DisplayMode::Both => {
            if msg.was_encrypted() {
                format!("{} → {}", unmodified.trim(), msg.to_string().trim()).into()
            } else {
                msg.to_string().trim().to_string().into()
            }
        },
    }
}
