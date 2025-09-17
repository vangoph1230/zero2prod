-- Add migration script here
CREATE TABLE users (
    user_id uuid PRIMARY key,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);