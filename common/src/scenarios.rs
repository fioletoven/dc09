use serde::{Deserialize, Serialize};

use crate::utils::VALID_KEY_LENGTHS;

/// Holds dialler configuration.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DiallerConfig {
    pub name: String,
    pub key: Option<String>,
    pub receiver: Option<String>,
    pub prefix: Option<String>,
    pub scenarios: Option<Vec<u16>>,
    #[serde(default)]
    pub sequence: u16,
    #[serde(default)]
    pub udp: bool,
    #[serde(default)]
    pub count: u16,
}

impl DiallerConfig {
    /// Creates new [`DiallerConfig`] instance.
    pub fn new(name: String, sequence: u16, udp: bool, count: u16) -> Self {
        Self {
            name,
            key: None,
            receiver: None,
            prefix: None,
            scenarios: None,
            sequence,
            udp,
            count,
        }
    }
}

/// Holds scenario configuration.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ScenarioConfig {
    pub id: u16,
    #[serde(default)]
    pub sequence: Vec<SignalConfig>,
}

/// Holds signal configuration.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SignalConfig {
    pub token: String,
    pub message: Option<String>,
    #[serde(default)]
    pub delay: u16,
    #[serde(default)]
    pub repeat: u16,
}

impl SignalConfig {
    /// Creates new [`SignalConfig`] instance.
    pub fn new(token: String, message: Option<String>, repeat: u16) -> Self {
        Self {
            token,
            message,
            delay: 0,
            repeat,
        }
    }
}

/// Holds test scenarios and dialer configurations.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Scenarios {
    #[serde(default)]
    pub diallers: Vec<DiallerConfig>,
    #[serde(default)]
    pub scenarios: Vec<ScenarioConfig>,
}

impl Scenarios {
    /// Returns scenario ids for the specified dialler's `account`.
    pub fn get_scenario_ids(&self, account: &str) -> Option<&[u16]> {
        self.diallers
            .iter()
            .find(|d| d.name == account)
            .and_then(|d| d.scenarios.as_deref())
    }

    /// Returns scenarios sequence for the specified `scenario_id`.
    pub fn get_sequence(&self, scenario_id: u16) -> Option<&Vec<SignalConfig>> {
        self.scenarios.iter().find(|s| s.id == scenario_id).map(|s| &s.sequence)
    }

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
