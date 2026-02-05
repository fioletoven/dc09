use common::dc09::parse_dc09_account_name;
use common::logging::DisplayMode;
use common::scenarios::DiallerConfig;
use common::utils::{SharedKeysMap, get_account_name};
use std::collections::HashMap;

pub type DiallerKeys = HashMap<String, u16>;

/// Defines possible responses for received messages.
#[derive(Clone, Copy, PartialEq)]
pub enum AckMode {
    Ack,
    Nak,
    Duh,
}

/// Server configuration.
pub struct ServerConfig {
    pub diallers: DiallerKeys,
    pub keys: SharedKeysMap,
    pub mode: DisplayMode,
    pub ack: AckMode,
}

impl ServerConfig {
    /// Creates new [`ServerConfig`] instance.
    pub fn new(config: &[DiallerConfig], keys: SharedKeysMap) -> Self {
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
            mode: DisplayMode::Target,
            ack: AckMode::Ack,
        }
    }

    /// Sets NAK flag.
    pub fn with_nak(mut self, send_naks: bool) -> Self {
        if send_naks {
            self.ack = AckMode::Nak;
        }

        self
    }

    /// Sets DUH flag.
    pub fn with_duh(mut self, send_duhs: bool) -> Self {
        if send_duhs {
            self.ack = AckMode::Duh;
        }

        self
    }

    /// Sets message display flag.
    pub fn with_msg_mode(mut self, mode: DisplayMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns key for the specified message.
    pub fn get_key_for_message(&self, received_message: &str) -> Option<&str> {
        if !self.diallers.is_empty()
            && let Ok(name) = parse_dc09_account_name(received_message)
            && self.diallers.contains_key(&name)
        {
            let index = self.diallers[&name];
            return self.keys.get(&index).map(String::as_str);
        }

        self.keys.get(&0).map(String::as_str)
    }
}
