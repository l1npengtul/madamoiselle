use crate::commands::modmail::{ModmailOpenReason, ModmailStatus, ModmailThread};
use crate::error::Error;
use serenity::all::{ChannelId, UserId};
use sqlx::{SqlitePool, query, Sqlite, Pool};
use std::str::FromStr;
use log::warn;
use sqlx::migrate::MigrateDatabase;

type OriginalMessage = u64;
type BoardMessage = u64;
// const SOURCE_TO_BOARD_TABLE: TableDefinition<OriginalMessage, BoardMessage> =
//     TableDefinition::new("original_to_board");
// const BOARD_TO_SOURCE_TABLE: TableDefinition<BoardMessage, OriginalMessage> =
//     TableDefinition::new("board_to_original");
// const DO_NOT_TRACK: TableDefinition<u64, ()> = TableDefinition::new("blacklist");

pub struct Database {
    // source_to_board: Database,
    // board_to_source: Database,
    // blacklist: Database,
    database: SqlitePool,
}

impl Database {
    pub async fn new(address: String) -> Result<Self, Error> {
        if !Sqlite::database_exists(&address).await? {
            warn!("creating database at {}", address);
            Sqlite::create_database(&address).await?
        }
        let database = SqlitePool::connect(&address).await?;
        Ok(Self { database })
    }

    pub async fn original_to_board(
        &self,
        source_message_id: OriginalMessage,
    ) -> Result<Option<BoardMessage>, Error> {
        let id = source_message_id as i64;
        Ok(sqlx::query!(
            r#"
SELECT message_map.board_id FROM message_map WHERE message_map.source_id = ?1
"#,
            id
        )
        .fetch_optional(&self.database)
        .await?
        .map(|record| record.board_id as u64))
    }

    pub async fn board_to_original(
        &self,
        board_message_id: BoardMessage,
    ) -> Result<Option<OriginalMessage>, Error> {
        let id = board_message_id as i64;
        Ok(sqlx::query!(
            r#"
SELECT message_map.source_id FROM message_map WHERE message_map.board_id = ?1
"#,
            id
        )
        .fetch_optional(&self.database)
        .await?
        .map(|record| record.source_id as u64))
    }

    pub async fn record_new(
        &self,
        original_message: OriginalMessage,
        board_message: BoardMessage,
    ) -> Result<(), Error> {
        let original_message = original_message as i64;
        let board_message = board_message as i64;
        sqlx::query!(
            r#"
INSERT INTO message_map VALUES ( ?1, ?2 )
"#,
            original_message,
            board_message
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn remove_record_board(&self, board_message: BoardMessage) -> Result<(), Error> {
        let id = board_message as i64;
        sqlx::query!(
            r#"
DELETE FROM message_map WHERE message_map.board_id = ?1
"#,
            id
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn create_new_modmail(&self, modmail: &ModmailThread) -> Result<(), Error> {
        let values = modmail.values();
        sqlx::query!(
            r#"
INSERT INTO modmail_threads VALUES ( $1, $2, $3, $4, $5, NULL, NULL, NULL )
"#,
            values.0,
            values.1,
            values.2,
            values.3,
            values.4
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn find_modmail_by_id(
        &self,
        thread_id: ChannelId,
    ) -> Result<Option<ModmailThread>, Error> {
        let id = thread_id.get() as i64;
        let query = sqlx::query!(
            r#"
SELECT * FROM modmail_threads WHERE modmail_threads.modmail_thread_id = ?1
"#,
            id
        )
        .fetch_optional(&self.database)
        .await?;
        let mm = match query {
            Some(record) => Some(ModmailThread {
                thread_id: ChannelId::new(record.modmail_thread_id as u64),
                creator_id: UserId::new(record.creator_user_id as u64),
                creation_date: record.creation_date,
                status: ModmailStatus::from_str(&record.status)?,
                reason: ModmailOpenReason::from_str(&record.reason)?,
                resolution_date: record.resolution_date,
                closing_notes: None,
                closing_user: None,
            }),
            None => None,
        };

        Ok(mm)
    }

    pub async fn set_modmail(&self, modmail_thread: ModmailThread) -> Result<(), Error> {
        let status = modmail_thread.status.to_string();
        let resolution_date = modmail_thread.resolution_date.ok_or(Error::BadTime)?;
        let notes = modmail_thread.closing_notes.as_ref();
        let closing_user = modmail_thread.closing_user.ok_or(Error::BadUser)?.get() as i64;
        let thread_id = modmail_thread.thread_id.get() as i64;
        let update = query!(
            r#"
UPDATE modmail_threads SET
                           status = $1,
                           resolution_date = $2,
                           closing_notes = $3,
                           closing_user = $4
WHERE
    modmail_thread_id = $5;
"#,
            status,
            resolution_date,
            notes,
            closing_user,
            thread_id
        )
        .execute(&self.database)
        .await?;

        match update.rows_affected() {
            0 => Err(Error::NotFound),
            1 => Ok(()),
            _ => Err(Error::DatabaseCorrupted),
        }
    }

    pub async fn shutdown(&self) {
        self.database.close().await
    }
}
