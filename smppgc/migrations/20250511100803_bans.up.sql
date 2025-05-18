-- Add up migration script here

CREATE TABLE bans (
  id SERIAL PRIMARY KEY,
  expiration_time INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  reason VARCHAR,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
