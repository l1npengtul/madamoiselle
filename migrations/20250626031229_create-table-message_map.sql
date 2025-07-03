-- Add migration script here
CREATE TABLE IF NOT EXISTS message_map (
    source_id INTEGER NOT NULL,
    board_id INTEGER NOT NULL,
    PRIMARY KEY(source_id, board_id)
) STRICT;


