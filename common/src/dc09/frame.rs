use super::DC09Error;
use super::parser::parse_frame_prefix;

#[cfg(test)]
#[path = "./frame.tests.rs"]
mod frame_tests;

/// `<LF>` + 4 (crc) + 4 (len).
const HEADER_LEN: usize = 9;

/// Possible DC09 frame errors.
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FrameError {
    /// Frame does not start with `<LF>` or its length field is not a 4-digit hex number.
    #[error("invalid DC09 frame header")]
    InvalidHeader,

    /// Frame is not terminated with `<CR>` at the position given by its length field.
    #[error("invalid DC09 frame terminator")]
    InvalidTerminator,
}

impl From<FrameError> for DC09Error {
    fn from(error: FrameError) -> Self {
        match error {
            FrameError::InvalidHeader => DC09Error::ParseHeaderError,
            FrameError::InvalidTerminator => DC09Error::InvalidLength,
        }
    }
}

/// Splits a byte stream into complete DC09 frames.
#[derive(Default)]
pub struct FrameBuffer {
    pending: Vec<u8>,
}

impl FrameBuffer {
    /// Creates new empty [`FrameBuffer`] instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends bytes received from the stream.
    pub fn extend(&mut self, data: &[u8]) {
        self.pending.extend_from_slice(data);
    }

    /// Returns `true` if there are buffered bytes that do not form a complete frame yet.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Takes all buffered bytes, e.g. to report them after a framing error.
    pub fn take_pending(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }

    /// Takes the next complete frame (including `<LF>` and `<CR>`) from the buffer.\
    /// **Note** that after an error the stream cannot be realigned and the connection should be closed.
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, FrameError> {
        if self.pending.is_empty() {
            return Ok(None);
        }

        if self.pending[0] != b'\n' {
            return Err(FrameError::InvalidHeader);
        }

        if self.pending.len() < HEADER_LEN {
            return Ok(None);
        }

        let (_, (_, len)) = std::str::from_utf8(&self.pending[..HEADER_LEN])
            .ok()
            .and_then(|header| parse_frame_prefix(header).ok())
            .ok_or(FrameError::InvalidHeader)?;

        let frame_len = HEADER_LEN + usize::from(len) + 1;
        if self.pending.len() < frame_len {
            return Ok(None);
        }

        if self.pending[frame_len - 1] != b'\r' {
            return Err(FrameError::InvalidTerminator);
        }

        Ok(Some(self.pending.drain(..frame_len).collect()))
    }
}
