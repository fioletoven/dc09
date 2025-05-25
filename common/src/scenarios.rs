use serde::{Deserialize, Serialize};

use crate::utils::VALID_KEY_LENGTHS;

/// Keeps dialler configuration.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DiallerConfig {
    pub name: String,
    pub key: Option<String>,
}

/// Stores dialer configurations and, in the future, will support test scenarios for them.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Scenarios {
    pub diallers: Vec<DiallerConfig>,
}

impl Scenarios {
    /// Checks whether [`Scenarios`] contains valid and meaningful data.
    pub fn validate(&self) -> Result<(), String> {
        for dialler in &self.diallers {
            if let Some(key) = &dialler.key {
                if !VALID_KEY_LENGTHS.contains(&key.len()) {
                    return Err(format!("{}: key length must be 16, 24 or 32 bytes", dialler.name));
                }
            }
        }

        Ok(())
    }
}
