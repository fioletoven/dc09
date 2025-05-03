const VALID_KEY_LENGTHS: [usize; 3] = [16, 24, 32];

/// Parses key and validates its length.
pub fn parse_key(s: &str) -> Result<String, String> {
    if VALID_KEY_LENGTHS.contains(&s.len()) {
        Ok(s.to_owned())
    } else {
        Err("Key length must be 16, 24 or 32 bytes".to_owned())
    }
}
