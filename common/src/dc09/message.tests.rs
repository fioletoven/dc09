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
fn account_name_invalid_header_test() {
    for frame in ["", "not a frame", "\n00000000\"SIA-DCS\"0001#1234[]\r"] {
        assert!(matches!(
            crate::dc09::parse_dc09_account_name(frame),
            Err(DC09Error::ParseHeaderError)
        ));
    }
}

#[test]
fn parse_nak_test() {
    let frame = "\nE4410025\"NAK\"0000R0L0A0[]_16:20:01,09-24-2025\r";
    let parsed = DC09Message::try_from(frame.as_bytes(), None).unwrap();
    assert_eq!("NAK", parsed.token);
    assert_eq!("A0", parsed.account);
    assert_eq!(crate::dc09::parse_dc09_account_name(frame).unwrap(), "A0");
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
fn decrypt_string_and_bytes_test() {
    for key in [
        "0123456789ABCDEF",
        "0123456789ABCDEF01234567",
        "0123456789ABCDEF0123456789ABCDEF",
    ] {
        let payload = "|#1234|NRR]";
        let ciphertext = crate::dc09::encrypt(payload, key.as_bytes()).unwrap();
        let from_string = crate::dc09::decrypt(ciphertext.as_str(), key.as_bytes()).unwrap();
        let from_bytes = crate::dc09::decrypt(ciphertext.as_bytes(), key.as_bytes()).unwrap();

        assert_eq!(from_string, from_bytes);
        assert!(from_bytes.ends_with(payload.as_bytes()));
        assert!(crate::dc09::decrypt([0xFF; 32], key.as_bytes()).is_none());
    }
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

#[test]
fn missing_line_prefix_test() {
    for token in ["SIA-DCS", "ACK", "*SIA-DCS"] {
        let body = format!("\"{token}\"0001#1234[]");
        let frame = format!("\n{:04X}{:04X}{body}\r", calculate_crc(body.as_bytes()), body.len());

        assert!(matches!(
            DC09Message::try_from(frame.as_bytes(), None),
            Err(DC09Error::ParseHeaderError)
        ));
    }
}

#[test]
fn malformed_payload_test() {
    for payload in [
        "#1234|NRR]",
        "[]GARBAGE",
        "[][Iunfinished",
        "[]\rGARBAGE",
        "[]\r",
        "[]_16:20:01,09-24-2025GARBAGE",
        "[]_16:20:01",
    ] {
        let body = format!("\"SIA-DCS\"0001L0#1234{payload}");
        let frame = format!("\n{:04X}{:04X}{body}\r", calculate_crc(body.as_bytes()), body.len());

        assert!(
            matches!(
                DC09Message::try_from(frame.as_bytes(), None),
                Err(DC09Error::ParsePayloadError)
            ),
            "unexpected result for payload {payload:?}"
        );
    }
}

#[test]
fn missing_or_invalid_message_terminator_test() {
    for encrypted in [false, true] {
        let key = "0123456789ABCDEF";
        let message = DC09Message::new("SIA-DCS".to_owned(), "1234".to_owned(), 1, None);
        let frame = if encrypted {
            message.to_encrypted(key).unwrap()
        } else {
            message.to_string()
        };

        for &terminator in b"X\n" {
            let mut malformed = frame.as_bytes().to_vec();
            *malformed.last_mut().unwrap() = terminator;
            assert!(matches!(
                DC09Message::try_from(&malformed, Some(key)),
                Err(DC09Error::ParsePayloadError)
            ));
        }

        assert!(matches!(
            DC09Message::try_from(&frame.as_bytes()[..frame.len() - 1], Some(key)),
            Err(DC09Error::InvalidLength)
        ));
    }
}

#[test]
fn message_length_limit_test() {
    for length in [0x0FFF, 0x1000, 0x1029] {
        let body = format!("\"SIA-DCS\"0001L0#1234[{}]", "x".repeat(length - 22));
        assert_eq!(body.len(), length);
        let frame = format!("\n{:04X}{:04X}{body}\r", calculate_crc(body.as_bytes()), body.len());
        let parsed = DC09Message::try_from(frame.as_bytes(), None);

        if length == 0x0FFF {
            assert!(parsed.is_ok());
        } else {
            assert!(matches!(parsed, Err(DC09Error::ParseHeaderError)));
        }
    }
}
