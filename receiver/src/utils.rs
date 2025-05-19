use common::dc09::DC09Message;

pub fn build_response_message(msg: DC09Message, key: Option<&str>, nak: bool) -> String {
    let was_encrypted = msg.was_encrypted();

    let token = if nak { "NAK".to_owned() } else { "ACK".to_owned() };
    let response = DC09Message::ack(token, msg.account, msg.sequence)
        .with_receiver(msg.receiver)
        .with_line_prefix(msg.line_prefix);

    if was_encrypted {
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
