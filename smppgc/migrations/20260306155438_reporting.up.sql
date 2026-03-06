-- Add up migration script here

CREATE TABLE reports(
  message_snowflake BIGINT,
  reason VARCHAR NULL,
  reporter_id INTEGER,

  FOREIGN KEY (message_snowflake) REFERENCES messages(snowflake) ON DELETE CASCADE,
  FOREIGN KEY (reporter_id) REFERENCES users(id) ON DELETE CASCADE,
  UNIQUE (message_snowflake, reporter_id)
)
