use crate::commands::modmail::{ModmailOpenReason, ModmailStatus, ModmailThread};
use crate::config::Starboard;
use crate::error::Error;
use crate::starboard::{DbStarboard, StarboardId, StarboardMessage, StarboardState};
use log::warn;
use poise::serenity_prelude::GuildId;
use serenity::all::{ChannelId, UserId};
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool, query};
use std::str::FromStr;

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
        sqlx::migrate!()
            .run(&database)
            .await
            .expect("Failed to run DB Migrations");
        Ok(Self { database })
    }

    pub async fn original_message_to_board(
        &self,
        source_msg: OriginalMessage,
    ) -> Result<Option<BoardMessage>, Error> {
        let source_msg_id = source_msg as i64;
        let board_message = sqlx::query!(
            r#"
SELECT starboard_message.board_message FROM starboard_message WHERE starboard_message.message_id = ?1
            "#,
            source_msg_id
        )
        .fetch_optional(&self.database)
        .await?.map(|record| record.board_message.map(|x| x as u64));
        Ok(board_message.flatten())
    }

    pub async fn board_message_to_original(
        &self,
        board_message: BoardMessage,
    ) -> Result<Option<OriginalMessage>, Error> {
        let board_message_id = board_message as i64;
        let source_message = sqlx::query!(
            r#"
SELECT starboard_message.message_id FROM starboard_message WHERE starboard_message.board_message = ?1
            "#,
            board_message_id
        )
        .fetch_optional(&self.database)
        .await?.map(|record| record.message_id as u64);
        Ok(source_message)
    }

    pub async fn starboard_by_name(
        &self,
        starboard_name: &str,
        guild_id: GuildId,
    ) -> Result<Option<DbStarboard>, Error> {
        let guild_id = guild_id.get() as i64;
        let record = match sqlx::query!(
            r#"
SELECT * FROM starboards WHERE starboards.guild_id = $1 AND starboards.starboard_name = $2
            "#,
            guild_id,
            starboard_name
        )
        .fetch_optional(&self.database)
        .await?
        {
            Some(record) => DbStarboard {
                starboard_id: StarboardId(record.starboard_id),
                guild_id: GuildId::new(record.guild_id as u64),
                starboard_name: record.starboard_name,
                active: StarboardState::try_from(record.active as i32)
                    .map_err(|_| Error::CorruptRecord("null active state".to_string()))?,
            },
            None => return Ok(None),
        };

        Ok(Some(record))
    }

    pub async fn starboards_by_guild(&self, guild_id: GuildId) -> Result<Vec<DbStarboard>, Error> {
        let guild_id = guild_id.get() as i64;
        let record = sqlx::query!(
            r#"
SELECT * FROM starboards WHERE starboards.guild_id = $1
            "#,
            guild_id,
        )
        .fetch_all(&self.database)
        .await?
        .into_iter()
        .map(|record| {
            StarboardState::try_from(record.active as i32).map(|state| DbStarboard {
                starboard_id: StarboardId(record.starboard_id),
                guild_id: GuildId::new(record.guild_id as u64),
                starboard_name: record.starboard_name,
                active: state,
            })
        })
        .collect::<Result<Vec<DbStarboard>, ()>>()
        .map_err(|_| Error::CorruptRecord("Bad Starboard State(s)".to_string()))?;

        Ok(record)
    }

    pub async fn starboard_add(
        &self,
        guild_id: GuildId,
        starboard_name: &str,
        active: StarboardState,
    ) -> Result<(), Error> {
        let guild_id = guild_id.get() as i64;
        let active = active as i32;
        sqlx::query!(
            r#"
INSERT INTO starboards (guild_id, starboard_name, active) VALUES ($1, $2, $3)
        "#,
            guild_id,
            starboard_name,
            active
        )
        .execute(&self.database)
        .await
        .map_err(|err| Error::DatabaseError(err))?;
        Ok(())
    }

    pub async fn starboard_update_name(
        &self,
        starboard_id: StarboardId,
        name: &str,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
UPDATE starboards SET starboard_name = $1 WHERE starboard_id = $2
        "#,
            name,
            starboard_id.0
        )
        .execute(&self.database)
        .await
        .map_err(|err| Error::DatabaseError(err))?;
        Ok(())
    }

    pub async fn starboard_update_state(
        &self,
        starboard_id: StarboardId,
        state: StarboardState,
    ) -> Result<(), Error> {
        let active = state as i32;
        sqlx::query!(
            r#"
UPDATE starboards SET active = $1 WHERE starboard_id = $2
        "#,
            active,
            starboard_id.0
        )
        .execute(&self.database)
        .await
        .map_err(|err| Error::DatabaseError(err))?;
        Ok(())
    }

    pub async fn starboard_delete(&self, starboard_id: StarboardId) -> Result<(), Error> {
        sqlx::query!(
            r#"
DELETE FROM starboards WHERE starboard_id = $1
            "#,
            starboard_id.0
        )
        .execute(&self.database)
        .await
        .map_err(|err| Error::DatabaseError(err))?;
        Ok(())
    }

    pub async fn starboard_board_message_add()
}

// impl Database {
//     pub async fn new(address: String) -> Result<Self, Error> {
//         if !Sqlite::database_exists(&address).await? {
//             warn!("creating database at {}", address);
//             Sqlite::create_database(&address).await?
//         }
//         let database = SqlitePool::connect(&address).await?;
//         sqlx::migrate!()
//             .run(&database)
//             .await
//             .expect("Failed to run DB Migrations");
//         Ok(Self { database })
//     }

//     pub async fn original_to_board(
//         &self,
//         source_message_id: OriginalMessage,
//     ) -> Result<Option<BoardMessage>, Error> {
//         let id = source_message_id as i64;
//         Ok(sqlx::query!(
//             r#"
// SELECT message_map.board_id FROM message_map WHERE message_map.source_id = ?1
// "#,
//             id
//         )
//         .fetch_optional(&self.database)
//         .await?
//         .map(|record| record.board_id as u64))
//     }

//     pub async fn board_to_original(
//         &self,
//         board_message_id: BoardMessage,
//     ) -> Result<Option<OriginalMessage>, Error> {
//         let id = board_message_id as i64;
//         Ok(sqlx::query!(
//             r#"
// SELECT message_map.source_id FROM message_map WHERE message_map.board_id = ?1
// "#,
//             id
//         )
//         .fetch_optional(&self.database)
//         .await?
//         .map(|record| record.source_id as u64))
//     }

//     pub async fn record_new(
//         &self,
//         original_message: OriginalMessage,
//         board_message: BoardMessage,
//     ) -> Result<(), Error> {
//         let original_message = original_message as i64;
//         let board_message = board_message as i64;
//         sqlx::query!(
//             r#"
// INSERT INTO message_map VALUES ( ?1, ?2 )
// "#,
//             original_message,
//             board_message
//         )
//         .execute(&self.database)
//         .await?;
//         Ok(())
//     }

//     pub async fn remove_record_board(&self, board_message: BoardMessage) -> Result<(), Error> {
//         let id = board_message as i64;
//         sqlx::query!(
//             r#"
// DELETE FROM message_map WHERE message_map.board_id = ?1
// "#,
//             id
//         )
//         .execute(&self.database)
//         .await?;
//         Ok(())
//     }

//     pub async fn create_new_modmail(&self, modmail: &ModmailThread) -> Result<(), Error> {
//         let values = modmail.values();
//         sqlx::query!(
//             r#"
// INSERT INTO modmail_threads VALUES ( $1, $2, $3, $4, $5, NULL, NULL, NULL )
// "#,
//             values.0,
//             values.1,
//             values.2,
//             values.3,
//             values.4
//         )
//         .execute(&self.database)
//         .await?;
//         Ok(())
//     }

//     pub async fn find_modmail_by_id(
//         &self,
//         thread_id: ChannelId,
//     ) -> Result<Option<ModmailThread>, Error> {
//         let id = thread_id.get() as i64;
//         let query = sqlx::query!(
//             r#"
// SELECT * FROM modmail_threads WHERE modmail_threads.modmail_thread_id = ?1
// "#,
//             id
//         )
//         .fetch_optional(&self.database)
//         .await?;
//         let mm = match query {
//             Some(record) => Some(ModmailThread {
//                 thread_id: ChannelId::new(record.modmail_thread_id as u64),
//                 creator_id: UserId::new(record.creator_user_id as u64),
//                 creation_date: record.creation_date,
//                 status: ModmailStatus::from_str(&record.status)?,
//                 reason: ModmailOpenReason::from_str(&record.reason)?,
//                 resolution_date: record.resolution_date,
//                 closing_notes: None,
//                 closing_user: None,
//             }),
//             None => None,
//         };

//         Ok(mm)
//     }

//     pub async fn set_modmail(&self, modmail_thread: ModmailThread) -> Result<(), Error> {
//         let status = modmail_thread.status.to_string();
//         let resolution_date = modmail_thread.resolution_date.ok_or(Error::BadTime)?;
//         let notes = modmail_thread.closing_notes.as_ref();
//         let closing_user = modmail_thread.closing_user.ok_or(Error::BadUser)?.get() as i64;
//         let thread_id = modmail_thread.thread_id.get() as i64;
//         let update = query!(
//             r#"
// UPDATE modmail_threads SET
//                            status = $1,
//                            resolution_date = $2,
//                            closing_notes = $3,
//                            closing_user = $4
// WHERE
//     modmail_thread_id = $5;
// "#,
//             status,
//             resolution_date,
//             notes,
//             closing_user,
//             thread_id
//         )
//         .execute(&self.database)
//         .await?;

//         match update.rows_affected() {
//             0 => Err(Error::NotFound),
//             1 => Ok(()),
//             _ => Err(Error::DatabaseCorrupted),
//         }
//     }

//     pub async fn shutdown(&self) {
//         self.database.close().await
//     }
// }
