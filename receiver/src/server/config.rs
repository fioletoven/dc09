use common::dc09::parse_dc09_account_name;
use common::scenarios::DiallerConfig;
use common::utils::{SharedKeysMap, get_account_name};
use std::collections::HashMap;

pub type DiallerKeys = HashMap<String, u16>;

/// Server configuration.
pub struct ServerConfig {
    pub diallers: DiallerKeys,
    pub keys: SharedKeysMap,
    pub send_naks: bool,
}

impl ServerConfig {
    /// Creates new [`ServerConfig`] instance.
    pub fn new(config: Vec<DiallerConfig>, keys: SharedKeysMap, send_naks: bool) -> Self {
        let mut diallers = DiallerKeys::new();
        for (index, dialler) in config.iter().enumerate() {
            let account = dialler.name.parse::<u32>().ok();
            let index = (index + 1) as u16;

            for i in 0..dialler.count.max(1) {
                let account = get_account_name(i, account, &dialler.name, false);
                diallers.insert(account, index);
            }
        }

        Self {
            diallers,
            keys,
            send_naks,
        }
    }

    /// Returns key for the specified message.
    pub fn get_key_for_message(&self, received_message: &str) -> Option<&str> {
        if !self.diallers.is_empty() {
            if let Ok(name) = parse_dc09_account_name(received_message) {
                if self.diallers.contains_key(&name) {
                    let index = self.diallers[&name];
                    return self.keys.get(&index).map(|k| k.as_str());
                }
            }
        }

        self.keys.get(&0).map(|k| k.as_str())
    }
}
