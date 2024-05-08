-- Add migration script here
ALTER TABLE users_table
RENAME COLUMN password_ TO passwordd;