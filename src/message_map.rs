use crate::error::Error;
use redb::{Database, TableDefinition};

type OriginalMessage = u64;
type BoardMessage = u64;
const SOURCE_TO_BOARD_TABLE: TableDefinition<OriginalMessage, BoardMessage> =
    TableDefinition::new("original_to_board");
const BOARD_TO_SOURCE_TABLE: TableDefinition<BoardMessage, OriginalMessage> =
    TableDefinition::new("board_to_original");

const DO_NOT_TRACK: TableDefinition<u64, ()> = TableDefinition::new("blacklist");

pub struct MessageMap {
    database: Database,
}

impl MessageMap {
    pub fn new(mut database: Database) -> Result<Self, Error> {
        if !database.check_integrity()? {
            return Err(Error::DatabaseCorrupted);
        }
        database.compact()?;
        Ok(Self { database })
    }

    pub fn original_to_board(
        &self,
        source_message_id: OriginalMessage,
    ) -> Result<Option<BoardMessage>, Error> {
        let read_trans = self.database.begin_read()?;
        let table = read_trans.open_table(SOURCE_TO_BOARD_TABLE)?;
        Ok(table.get(source_message_id)?.map(|x| x.value()))
    }

    pub fn board_to_original(
        &self,
        board_message: BoardMessage,
    ) -> Result<Option<OriginalMessage>, Error> {
        let read_trans = self.database.begin_read()?;
        let table = read_trans.open_table(BOARD_TO_SOURCE_TABLE)?;
        Ok(table.get(board_message)?.map(|x| x.value()))
    }

    pub fn record_new(
        &self,
        original_message: OriginalMessage,
        board_message: BoardMessage,
    ) -> Result<(), Error> {
        let write_trans = self.database.begin_write()?;
        {
            let mut s2b = write_trans.open_table(SOURCE_TO_BOARD_TABLE)?;
            s2b.insert(original_message, board_message)?;
        }
        write_trans.commit()?;

        let write_trans = self.database.begin_write()?;
        {
            let mut b2s = write_trans.open_table(BOARD_TO_SOURCE_TABLE)?;
            b2s.insert(board_message, original_message)?;
        }
        write_trans.commit()?;
        Ok(())
    }

    pub fn remove_record_source(&self, original_message: OriginalMessage) -> Result<(), Error> {
        let board = match self.original_to_board(original_message)? {
            Some(b) => b,
            None => return Ok(()),
        };

        let write_trans = self.database.begin_write()?;
        {
            let mut s2b = write_trans.open_table(SOURCE_TO_BOARD_TABLE)?;
            let mut b2s = write_trans.open_table(BOARD_TO_SOURCE_TABLE)?;
            s2b.remove(original_message)?;
            b2s.remove(board)?;
        }
        write_trans.commit()?;

        Ok(())
    }

    pub fn remove_record_board(&self, board_message: BoardMessage) -> Result<(), Error> {
        let original = match self.board_to_original(board_message)? {
            Some(b) => b,
            None => return Ok(()),
        };

        let write_trans = self.database.begin_write()?;
        {
            let mut s2b = write_trans.open_table(SOURCE_TO_BOARD_TABLE)?;
            let mut b2s = write_trans.open_table(BOARD_TO_SOURCE_TABLE)?;
            s2b.remove(original)?;
            b2s.remove(board_message)?;
        }
        write_trans.commit()?;

        Ok(())
    }

    pub fn blacklist(&self, message: u64) -> Result<(), Error> {
        let write_trans = self.database.begin_write()?;
        {
            let mut nolist = write_trans.open_table(DO_NOT_TRACK)?;
            nolist.insert(message, ())?;
        }
        write_trans.commit()?;
        Ok(())
    }

    pub fn is_blacklisted(&self, message: u64) -> Result<bool, Error> {
        let read_trans = self.database.begin_read()?;
        Ok(read_trans.open_table(DO_NOT_TRACK)?.get(message)?.is_some())
    }

    pub fn unblacklist(&self, message: u64) -> Result<(), Error> {
        let write_trans = self.database.begin_write()?;
        {
            let mut nolist = write_trans.open_table(DO_NOT_TRACK)?;
            nolist.remove(message)?;
        }
        write_trans.commit()?;
        Ok(())
    }
}
