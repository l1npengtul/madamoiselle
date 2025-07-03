-- Add migration script here
CREATE TABLE IF NOT EXISTS modmail_threads (
    modmail_thread_id INTEGER PRIMARY KEY,
    creator_user_id INTEGER NOT NULL,
    creation_data INTEGER NOT NULL,
    status TEXT NOT NULL,
    reason TEXT NOT NULL
) STRICT;

PRAGMA FOREIGN_KEYS = on;

CREATE TABLE IF NOT EXISTS message_archives (
    pk INTEGER PRIMARY KEY,
    modmail_thread_id INTEGER NOT NULL,
    sender_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    FOREIGN KEY (modmail_thread_id) REFERENCES modmail_threads(modmail_thread_id)
) STRICT;