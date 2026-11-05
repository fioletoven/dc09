use super::*;

const FRAME: &[u8] = b"\n96ED0016\"SIA-DCS\"0001L0#1234[]\r";
const FRAME_2: &[u8] = b"\nE4410025\"NAK\"0000R0L0A0[]_16:20:01,09-24-2025\r";

#[test]
fn single_frame_test() {
    let mut buffer = FrameBuffer::new();
    buffer.extend(FRAME);

    assert_eq!(Ok(Some(FRAME.to_vec())), buffer.next_frame());
    assert_eq!(Ok(None), buffer.next_frame());
    assert!(!buffer.has_pending());
}

#[test]
fn frame_split_byte_by_byte_test() {
    let mut buffer = FrameBuffer::new();

    for (i, byte) in FRAME.iter().enumerate() {
        assert_eq!(Ok(None), buffer.next_frame(), "frame returned early at byte {i}");
        buffer.extend(&[*byte]);
    }

    assert_eq!(Ok(Some(FRAME.to_vec())), buffer.next_frame());
    assert!(!buffer.has_pending());
}

#[test]
fn frame_split_after_header_test() {
    let mut buffer = FrameBuffer::new();
    buffer.extend(&FRAME[..9]);
    assert_eq!(Ok(None), buffer.next_frame());

    buffer.extend(&FRAME[9..]);
    assert_eq!(Ok(Some(FRAME.to_vec())), buffer.next_frame());
}

#[test]
fn multiple_frames_in_one_read_test() {
    let mut buffer = FrameBuffer::new();
    buffer.extend(&[FRAME, FRAME_2, &FRAME[..5]].concat());

    assert_eq!(Ok(Some(FRAME.to_vec())), buffer.next_frame());
    assert_eq!(Ok(Some(FRAME_2.to_vec())), buffer.next_frame());
    assert_eq!(Ok(None), buffer.next_frame());
    assert!(buffer.has_pending());

    buffer.extend(&FRAME[5..]);
    assert_eq!(Ok(Some(FRAME.to_vec())), buffer.next_frame());
}

#[test]
fn missing_lf_test() {
    let mut buffer = FrameBuffer::new();
    buffer.extend(&FRAME[1..]);

    assert_eq!(Err(FrameError::InvalidHeader), buffer.next_frame());
}

#[test]
fn invalid_length_field_test() {
    let mut buffer = FrameBuffer::new();
    buffer.extend(b"\n96ED00G6\"SIA-DCS\"0001L0#1234[]\r");

    assert_eq!(Err(FrameError::InvalidHeader), buffer.next_frame());
}

#[test]
fn invalid_terminator_test() {
    let mut buffer = FrameBuffer::new();
    // length says 0x15 instead of 0x16, so the byte at the CR position is ']'
    buffer.extend(b"\n96ED0015\"SIA-DCS\"0001L0#1234[]\r");

    assert_eq!(Err(FrameError::InvalidTerminator), buffer.next_frame());
}
