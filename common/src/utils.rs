use std::{collections::HashMap, fs::File, io::Read, path::Path, sync::Arc};

use crate::scenarios::Scenarios;

pub type SharedKeysMap = Arc<HashMap<u16, String>>;

pub const VALID_KEY_LENGTHS: [usize; 3] = [16, 24, 32];

/// Parses key and validates its length.
pub fn parse_key(s: &str) -> Result<String, String> {
    if VALID_KEY_LENGTHS.contains(&s.len()) {
        Ok(s.to_owned())
    } else {
        Err("Key length must be 16, 24 or 32 bytes".to_owned())
    }
}

/// Loads [`Scenarios`] from the provided file path.
pub fn parse_scenarios_path(s: &str) -> Result<Scenarios, String> {
    let path = Path::new(s);
    if !path.exists() {
        return Err("The provided file does not exist".to_owned());
    }

    if let Ok(mut file) = File::open(path) {
        let mut scenarios_str = String::new();
        if file.read_to_string(&mut scenarios_str).is_ok() {
            if let Ok(scenarios) = serde_json::from_str::<Scenarios>(&scenarios_str) {
                return match scenarios.validate() {
                    Ok(_) => Ok(scenarios),
                    Err(e) => Err(e),
                };
            }
        }
    }

    Err("Unable to deserialize the provided file into a Scenarios object".to_owned())
}

/// Builds a hash map with all keys provided to the app.
pub fn build_keys_map(scenarios: Option<&Scenarios>, default_key: Option<&str>) -> SharedKeysMap {
    let mut result = HashMap::new();

    if let Some(key) = default_key {
        result.insert(0, key.to_owned());
    }

    if let Some(scenarios) = scenarios {
        for (index, dialler) in scenarios.diallers.iter().enumerate() {
            if let Some(key) = &dialler.key {
                result.insert((index + 1) as u16, key.to_owned());
            }
        }
    }

    Arc::new(result)
}

/// Returns account name from either `account_num` (incremented by `index`) or `account_str`.\
/// **Note** that it will always return `account_str` if fixed.
pub fn get_account_name(index: u16, account_num: Option<u32>, account_str: &str, fixed: bool) -> String {
    if fixed {
        account_str.to_owned()
    } else {
        account_num
            .map(|a| (a + index as u32).to_string())
            .unwrap_or_else(|| account_str.to_owned())
    }
}
