use super::*;

#[test]
fn to_string_test() {
    let msg = DC09Message::new("SIA-DCS".to_owned(), "1234".to_owned(), 1, Some("#1234|NRR|Atest".to_owned()))
        .with_line_prefix(Some("L77".to_owned()))
        .with_receiver(Some("RF3".to_owned()));

    assert_eq!("\nF4D20029\"SIA-DCS\"0001RF3L77#1234[#1234|NRR|Atest]\r", msg.to_string());
}

#[test]
fn parse_message_test() {
    let msg = DC09Message::new("SIA-DCS".to_owned(), "1234".to_owned(), 1, None).with_line_prefix(Some("L0".to_owned()));
    let parsed = DC09Message::try_from("\n96ED0016\"SIA-DCS\"0001L0#1234[]\r".as_bytes(), None).unwrap();

    assert_eq!(msg, parsed);
}

#[test]
fn parse_nak_test() {
    let parsed = DC09Message::try_from("\nE4410025\"NAK\"0000R0L0A0[]_16:20:01,09-24-2025\r".as_bytes(), None).unwrap();
    assert_eq!("NAK", parsed.token);
    assert_eq!("A0", parsed.account);
}

#[test]
fn full_test() {
    let msg = DC09Message::new(
        "SIA-DCS".to_owned(),
        "1234".to_owned(),
        55,
        Some("#1234|NRR|AStart".to_owned()),
    )
    .with_line_prefix(Some("L0".to_owned()));
    let parsed = DC09Message::try_from(msg.to_string().as_bytes(), None).unwrap();

    assert_eq!(msg, parsed);
}

#[test]
fn encryption_test() {
    let key = "aaaaaaaaaaaaaaaabbbbbbbbbbbbbbbb";
    let msg = DC09Message::new(
        "*SIA-DCS".to_owned(),
        "1234".to_owned(),
        1,
        Some("#1234|NRR|Atest".to_owned()),
    )
    .with_line_prefix(Some("L0".to_owned()));

    let encrypted = msg.to_encrypted(key).unwrap();
    let decrypted = DC09Message::try_from(encrypted.as_bytes(), Some(key)).unwrap();

    assert_eq!(msg, decrypted);
}

#[test]
fn invalid_key_test() {
    let message = DC09Message::new("SIA-DCS".to_owned(), "1234".to_owned(), 1, Some("#1234|NRR".to_owned()));
    let frame = message.to_encrypted("0123456789ABCDEF").unwrap();

    for key in [None, Some("invalid")] {
        assert!(matches!(
            DC09Message::try_from(frame.as_bytes(), key),
            Err(DC09Error::DecryptError)
        ));
    }
}

#[test]
fn invalid_payload_test() {
    for payload in ["[abc|#1234|NRR]", "[00", "[0", "", "[", "[\u{e9}", "00"] {
        let body = format!("\"*SIA-DCS\"0001L0#1234{payload}");
        let frame = format!("\n{:04X}{:04X}{body}\r", calculate_crc(body.as_bytes()), body.len());

        assert!(
            matches!(
                DC09Message::try_from(frame.as_bytes(), Some("0123456789ABCDEF")),
                Err(DC09Error::DecryptError)
            ),
            "unexpected result for payload {payload:?}"
        );
    }
}
