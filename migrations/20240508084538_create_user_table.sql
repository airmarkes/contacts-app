-- Add migration script here
CREATE TABLE IF NOT EXISTS users_table (
    id INTEGER PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    password_ TEXT NOT NULL    
);