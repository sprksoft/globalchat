-- Add up migration script here

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  smid VARCHAR NOT NULL UNIQUE,
  irl_name VARCHAR NOT NULL,
  role INTEGER NOT NULL DEFAULT 0,
  ban_count INTEGER NOT NULL DEFAULT 0,
  ban_release_timestamp INTEGER NOT NULL DEFAULT 0 -- NULL means not banned
);

CREATE TABLE names (
  name VARCHAR NOT NULL,
  user_id INTEGER NOT NULL,
  last_used TIMESTAMP NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
