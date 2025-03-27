use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serenity::all::Emoji;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub discord: Discord,
    pub starboard: Starboard,
    pub places: Places,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Places {
    pub db_path: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Discord {
    pub token: Option<String>,
    pub admin_role: u64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Starboard {
    pub requirement: i32,
    pub sending_channel: u64,
    pub emoji: Vec<String>,
    pub overrides: HashMap<u64, Override>
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Override {
    pub requirement: i32,
    pub emoji: Vec<String>,
}
