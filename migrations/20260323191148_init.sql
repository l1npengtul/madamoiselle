-- Add migration script here
PRAGMA foreign_keys = on; 

CREATE TABLE modmail(
  pk INTEGER PRIMARY KEY NOT NULL UNIQUE,
  guild_id INTEGER NOT NULL,
  thread_id INTEGER NOT NULL,
  reason TEXT NOT NULL,
  opening_time INTEGER NOT NULL,
  opening_user INTEGER NOT NULL,
  
  closing_user INTEGER,
  closing_time INTEGER,
  closing_reason INTEGER
) STRICT;

CREATE TABLE starboards(
  starboard_id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  guild_id INTEGER NOT NULL,
  starboard_name TEXT NOT NULL,
  active INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_starboard ON starboards (guild_id, starboard_name);

CREATE TABLE starboard_board_message(
  board_message_id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  channel_id INTEGER NOT NULL,
  starboard_id INTEGER,
  FOREIGN KEY (starboard_id) 
  REFERENCES starboards (starboard_id)
    ON UPDATE NO ACTION
    ON DELETE SET NULL
) STRICT;

CREATE TABLE starboard_message(
  message_id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  channel_id INTEGER NOT NULL,
  board_message INTEGER REFERENCES starboard_board_message (board_message_id) ON UPDATE NO ACTION ON DELETE SET NULL,
  tracking_state INTEGER NOT NULL,
  starboard_id INTEGER REFERENCES starboards (starboard_id) ON UPDATE NO ACTION ON DELETE SET NULL
) STRICT;

CREATE INDEX idx_starboard_message_channel_id ON starboard_message (channel_id, message_id);

CREATE TABLE banners(
  banner_id INTEGER PRIMARY KEY NOT NULL UNIQUE,
  guild_id INTEGER NOT NULL,
  illustrator TEXT NOT NULL,
  image_path TEXT NOT NULL,
  banner_weight REAL NOT NULL,
  fallback_pool INTEGER NOT NULL,
  
  applicable_month_start INTEGER NOT NULL,
  applicable_day_start INTEGER NOT NULL,
  applicable_month_end INTEGER NOT NULL,
  applicable_day_end INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_banners ON banners (guild_id, illustrator);

CREATE TABLE banner_log(
  pk INTEGER PRIMARY KEY NOT NULL UNIQUE,

  time_start INTEGER NOT NULL,
  time_end INTEGER NOT NULL,
  banner_state TEXT NOT NULL,

  banner_id INTEGER,
  FOREIGN KEY (banner_id)
  REFERENCES banners (banner_id)
    ON UPDATE NO ACTION
    ON DELETE SET NULL
 ) STRICT;