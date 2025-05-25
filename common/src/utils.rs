use std::{fs::File, io::Read, path::Path};

use crate::scenarios::Scenarios;

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
