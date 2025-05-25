use common::dc09::parse_dc09_account_name;
use common::scenarios::DiallerConfig;
use std::collections::HashMap;

pub type DiallerKeys = HashMap<String, Option<String>>;

/// Server configuration.
pub struct ServerConfig {
    pub diallers: DiallerKeys,
    pub key: Option<String>,
    pub send_naks: bool,
}

impl ServerConfig {
    /// Creates new [`ServerConfig`] instance.
    pub fn new(diallers: Vec<DiallerConfig>, key: Option<String>, send_naks: bool) -> Self {
        let diallers = diallers.into_iter().map(|d| (d.name, d.key)).collect::<DiallerKeys>();
        Self {
            diallers,
            key,
            send_naks,
        }
    }

    /// Returns key for the specified message.
    pub fn get_key_for_message(&self, received_message: &str) -> Option<&str> {
        if !self.diallers.is_empty() {
            if let Ok(name) = parse_dc09_account_name(received_message) {
                if self.diallers.contains_key(&name) {
                    return self.diallers[&name].as_deref();
                }
            }
        }

        self.key.as_deref()
    }
}
