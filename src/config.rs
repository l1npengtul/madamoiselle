use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub discord: Discord,
    pub places: Places,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GuildConfig {
    pub guild_id: u64,
    pub log_channel: u64,
    pub admin_roles: Vec<u64>,
    pub mod_roles: Vec<u64>,
    pub starboard: StarboardConfig,
    pub modmail: Modmail,
    pub banner: Banner,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct StarboardConfig {
    pub enable: bool,
    pub starboards: HashMap<String, Starboard>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub discord: Discord,
    pub starboard: Starboard,
    pub places: Places,
    pub modmail: Modmail,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Places {
    pub db_path: String,
    pub state_path: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Discord {
    pub token_env: String,
    pub status: Option<String>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Modmail {
    pub enable: bool,
    pub channel: Option<u64>,
    pub bot_message: Option<u64>,
    pub roles: Vec<u64>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Starboard {
    pub sending_channel: Option<u64>,
    pub requirement: u32,
    pub requirement_overrides: HashMap<u64, u32>,
    pub valid_emoji: Vec<String>,
    pub ignore_older_than: Option<Duration>,
    pub role_to_assign: Option<u64>,
    pub channel_filter_type_include: bool,
    pub channels: Vec<u64>,
    pub handle_delete: bool,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Banner {
    pub enable: bool,
    pub banner_debug_channel: Option<u64>,
    pub banner_announcement: Option<u64>,
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct Starboard {
//     pub requirement: u64,
//     pub sending_channel: Option<u64>,
//     pub emoji: Vec<String>,
//     pub overrides: HashMap<String, Override>,
//     pub excluded_channels: Vec<u64>,
//     pub ignore_older_than: Option<Duration>,
// }

// impl Default for Starboard {
//     fn default() -> Self {
//         Self {
//             requirement: 4,
//             sending_channel: None,
//             emoji: vec![],
//             overrides: Default::default(),
//             excluded_channels: vec![],
//             ignore_older_than: Some(Duration::from_secs(60 * 60 * 24 * 30)),
//         }
//     }
// }

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct Override {
//     pub requirement: Option<u64>,
// }

// impl Default for Override {
//     fn default() -> Self {
//         Self { requirement: None }
//     }
// }
