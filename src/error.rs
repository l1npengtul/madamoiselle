use poise::serenity_prelude::EmojiIdentifierParseError;
use poise::serenity_prelude::Error as SerenityError;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    DatabaseError(sqlx::Error),
    DatabaseCorrupted,
    Discord(SerenityError),
    SetConfigErr,
    BadEmojis(EmojiIdentifierParseError),
    BadDuration(String),
    BadUser,
    BadTime,
    BadStatus,
    BadInteraction,
    BadReason,
    NotFound,
    Invariant,
    CorruptRecord(String),
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<poise::serenity_prelude::Error> for Error {
    fn from(value: SerenityError) -> Self {
        Self::Discord(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
