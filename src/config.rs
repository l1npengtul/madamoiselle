use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub discord: Discord,
    pub starboard: Starboard,
    pub places: Places,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Places {
    pub db_path: Option<PathBuf>,
    // pub log_path: Option<PathBuf>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Discord {
    pub token: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Starboard {
    pub requirement: i32,
    pub sending_channel: Option<u64>,
    pub emoji: Vec<String>,
    pub overrides: HashMap<u64, Override>,
    pub excluded_channels: Vec<u64>,
    pub ignore_older_than: Option<Duration>,
}

impl Default for Starboard {
    fn default() -> Self {
        Self {
            requirement: 4,
            sending_channel: None,
            emoji: vec![],
            overrides: Default::default(),
            excluded_channels: vec![],
            ignore_older_than: Some(Duration::from_secs(60 * 60 * 24 * 30)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Override {
    pub requirement: i32,
}

impl Default for Override {
    fn default() -> Self {
        Self { requirement: 4 }
    }
}
