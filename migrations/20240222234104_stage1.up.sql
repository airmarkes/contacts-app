-- Add up migration script here
CREATE TABLE IF NOT EXISTS contacts_table (
    id INTEGER PRIMARY KEY NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT NOT NULL,
    time_creation TEXT NOT NULL
);