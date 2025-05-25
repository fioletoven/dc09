use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DiallerConfig {
    pub name: String,
    pub key: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Scenarios {
    pub diallers: Vec<DiallerConfig>,
}
