use common::dc09::parse_dc09_account_name;
use common::logging::DisplayMode;
use common::scenarios::DiallerConfig;
use common::utils::{SharedKeysMap, get_account_name};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};

pub type DiallerKeys = HashMap<String, u16>;

/// Server configuration.
pub struct ServerConfig {
    pub diallers: DiallerKeys,
    pub keys: SharedKeysMap,
    pub mode: DisplayMode,
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
        }
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

/// Defines possible responses for received messages.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum ResponseMode {
    #[default]
    Ack,
    Nak,
    Duh,
    None,
}

impl From<u8> for ResponseMode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Ack,
            1 => Self::Nak,
            2 => Self::Duh,
            _ => Self::None,
        }
    }
}

impl From<ResponseMode> for u8 {
    fn from(value: ResponseMode) -> Self {
        match value {
            ResponseMode::Ack => 0,
            ResponseMode::Nak => 1,
            ResponseMode::Duh => 2,
            ResponseMode::None => 255,
        }
    }
}

impl Display for ResponseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack => write!(f, "ack"),
            Self::Nak => write!(f, "nak"),
            Self::Duh => write!(f, "duh"),
            Self::None => write!(f, "none"),
        }
    }
}

impl FromStr for ResponseMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "ack" => Ok(Self::Ack),
            "nak" => Ok(Self::Nak),
            "duh" => Ok(Self::Duh),
            "none" => Ok(Self::None),
            other => Err(format!("unknown response mode '{other}'")),
        }
    }
}

/// Holds response mode atomics for both message types.
#[derive(Debug)]
pub struct ResponseModes {
    pub message: AtomicU8,
    pub heartbeat: AtomicU8,
}

impl Default for ResponseModes {
    fn default() -> Self {
        Self::new(ResponseMode::Ack, ResponseMode::Ack)
    }
}

impl ResponseModes {
    /// Creates new [`ResponseModes`] instance.
    pub fn new(message: ResponseMode, heartbeat: ResponseMode) -> Self {
        Self {
            message: AtomicU8::new(message.into()),
            heartbeat: AtomicU8::new(heartbeat.into()),
        }
    }

    /// Gets response mode for messages.
    pub fn message(&self) -> ResponseMode {
        self.message.load(Ordering::Relaxed).into()
    }

    /// Gets response mode for heartbeats.
    pub fn heartbeat(&self) -> ResponseMode {
        self.heartbeat.load(Ordering::Relaxed).into()
    }

    /// Sets response mode for messages.
    pub fn set_message(&self, mode: ResponseMode) {
        self.message.store(mode.into(), Ordering::Relaxed);
    }

    /// Sets response mode for heartbeats.
    pub fn set_heartbeat(&self, mode: ResponseMode) {
        self.heartbeat.store(mode.into(), Ordering::Relaxed);
    }
}
