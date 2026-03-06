-- Add up migration script here

CREATE TABLE messages(
  snowflake BIGINT PRIMARY KEY,
  sender_id INTEGER,
  sender_name VARCHAR,
  content VARCHAR,

  FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE
);
