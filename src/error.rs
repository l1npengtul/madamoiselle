use serenity::all::EmojiParseError;
use serenity::prelude::SerenityError;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    DatabaseError(sqlx::Error),
    DatabaseCorrupted,
    Discord(SerenityError),
    SetConfigErr,
    BadEmojis(EmojiParseError),
    BadDuration(String),
    BadUser,
    BadTime,
    BadStatus,
    BadInteraction,
    BadReason,
    NotFound,
    Invariant,
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<serenity::Error> for Error {
    fn from(value: SerenityError) -> Self {
        Self::Discord(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
