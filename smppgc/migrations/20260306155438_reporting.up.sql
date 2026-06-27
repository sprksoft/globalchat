-- Add up migration script here

CREATE TABLE reports(
  message_snowflake BIGINT,
  reporter_id INTEGER,
  reason VARCHAR NULL,
  message VARCHAR NULL,

  FOREIGN KEY (reporter_id) REFERENCES users(id) ON DELETE CASCADE,
  UNIQUE (message_snowflake, reporter_id)
)
