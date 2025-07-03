-- Add migration script here
ALTER TABLE modmail_threads RENAME COLUMN creation_data TO creation_date;
ALTER TABLE modmail_threads ADD COLUMN resolution_date INTEGER;