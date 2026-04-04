use std::fmt::Display;

use poise::serenity_prelude::{GuildId, MessageId};

pub struct Message {
    pub message_id: MessageId,
    pub guild_id: GuildId,
}

pub enum MessageUpdate {
    ContentUpdated,
    Deleted,
    ReactionAdded,
    ReactionRemoved,
    AllReactionsRemoved,
}

#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StarboardId(pub i64);

#[repr(i32)]
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum StarboardMessageTrackingState {
    #[default]
    Tracked = 1,
    Untracked = -1,
    Archived = -2,
}

impl Display for StarboardMessageTrackingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StarboardMessageTrackingState::Tracked => {
                write!(f, "Tracked (1)")
            }
            StarboardMessageTrackingState::Untracked => {
                write!(f, "Untracked (-1)")
            }
            StarboardMessageTrackingState::Archived => {
                write!(f, "Archived (-2)")
            }
        }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum StarboardState {
    #[default]
    Active = 1,
    Inactive = -1,
    Invalid = -2,
}

impl Display for StarboardState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StarboardState::Active => {
                write!(f, "Active (1)")
            }
            StarboardState::Inactive => {
                write!(f, "Inactive (-1)")
            }
            StarboardState::Invalid => {
                write!(f, "Invalid (-2)")
            }
        }
    }
}

impl TryFrom<i32> for StarboardState {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(StarboardState::Active),
            -1 => Ok(StarboardState::Inactive),
            -2 => Ok(StarboardState::Invalid),
            _ => Err(()),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StarboardMessage {
    pub message_id: MessageId,
    pub board_message: MessageId,
    pub starboard_id: Option<StarboardId>,
    pub tracking_state: StarboardMessageTrackingState,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DbStarboard {
    pub starboard_id: StarboardId,
    pub guild_id: GuildId,
    pub starboard_name: String,
    pub active: StarboardState,
}
