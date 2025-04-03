use redb::{
    CommitError, CompactionError, DatabaseError, StorageError, TableError, TransactionError,
};
use serenity::all::EmojiParseError;
use serenity::prelude::SerenityError;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    DatabaseError(DatabaseError),
    DatabaseCorrupted,
    Transaction(TransactionError),
    Table(TableError),
    Storage(StorageError),
    Compact(CompactionError),
    Write(CommitError),
    Discord(SerenityError),
    SetConfigErr,
    BadEmojis(EmojiParseError),
    BadDuration(String),
}

impl From<DatabaseError> for Error {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<TransactionError> for Error {
    fn from(value: TransactionError) -> Self {
        Self::Transaction(value)
    }
}

impl From<TableError> for Error {
    fn from(value: TableError) -> Self {
        Self::Table(value)
    }
}

impl From<StorageError> for Error {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
impl From<CompactionError> for Error {
    fn from(value: CompactionError) -> Self {
        Self::Compact(value)
    }
}

impl From<CommitError> for Error {
    fn from(value: CommitError) -> Self {
        Self::Write(value)
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
